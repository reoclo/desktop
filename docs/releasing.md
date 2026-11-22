# Releasing

## Packaging

`scripts/bundle-macos.sh /path/to/reoclo` builds `dist/Reoclo.app` with that CLI binary inside, signs both with the hardened runtime, verifies the signature and writes `dist/Reoclo.dmg`. Local builds are signed ad hoc. Set `REOCLO_SIGN_IDENTITY` to a Developer ID Application identity to sign with it and a secure timestamp, and the disk image is signed too; `REOCLO_SIGN_KEYCHAIN` names the keychain that holds it. `scripts/notarize-macos.sh` then submits the disk image to Apple's notary service with the App Store Connect API key file in `ASC_KEY_PATH` and its ids in `ASC_KEY_ID` and `ASC_ISSUER_ID`. It staples the ticket to the disk image and the app, and fails unless Gatekeeper accepts the app as notarized. Pass a prebuilt desktop binary as a second argument to package a universal build. The script neither installs nor launches the app, and it never fetches or substitutes a CLI. The disk image window is built with `dmgbuild` through `uvx`; its background comes from `assets/dmg/background.html`.

## Releases

The private Gitea repository mirrors branches and tags to `reoclo/desktop` on GitHub. Pushing a tag such as `v0.5.0` runs [.github/workflows/release.yml](../.github/workflows/release.yml) on a macOS GitHub runner. The tag must match the Cargo package version. You can also dispatch the workflow with an existing tag.

The workflow builds both Mac architectures and combines them with `lipo`. It fetches the latest released CLI binaries and verifies their published checksums before making a universal CLI. The release notes record the CLI version. It imports the Developer ID Application certificate of BOXMARSHALL LTD (team `2446T6MBQU`) into a temporary keychain, signs the bundle with it and notarizes the disk image before packaging, so the download and the update archive both carry the stapled ticket. `scripts/package-release.sh` packages the signed app with permissions and symlinks intact and runs `update-sign` to write `latest.json`.

`MACOS_DEVELOPER_ID_P12` is a GitHub Actions secret holding that certificate and its private key as a base64 `.p12`, with its password in `MACOS_DEVELOPER_ID_P12_PASSWORD`. The App Store Connect API key used for notarization is `ASC_API_KEY_P8`, with its ids in `ASC_API_KEY_ID` and `ASC_API_ISSUER_ID`. A release fails rather than publish an unsigned build when they are missing.

`UPDATE_SIGNING_KEY` is a GitHub Actions secret containing the base64 raw 32-byte Ed25519 seed. The signer checks that it matches `assets/update-public-key.txt`. The app embeds only that public key. `update-sign --test-key` accepts a throwaway seed for local packaging exercises; it does not change the app's trusted key. For development, `REOCLO_UPDATE_URL` overrides the manifest endpoint. HTTPS is required except on loopback.

After building the local bundle, run `cargo build --bin update-sign` followed by `cargo run --example update-smoke`. This packages and signs the actual bundle with a generated test key, serves it over loopback, and uses `REOCLO_UPDATE_URL` in a child process to install it over a temporary older app. The test validates the installed bundle with `codesign` and never opens a live app. It prints `UPDATE INSTALL PASS` on success. `dist/latest.json` from this exercise is test-signed and must not be published.

Copy [packaging/homebrew/reoclo-desktop.rb](../packaging/homebrew/reoclo-desktop.rb) to `Casks/reoclo-desktop.rb` in `reoclo/homebrew-tap`. Copy [packaging/homebrew/update-desktop-cask.yml](../packaging/homebrew/update-desktop-cask.yml) to that repository's `.github/workflows/`. The tap's own token refreshes the cask every 30 minutes. Run it manually after the first release to replace the bootstrap `:no_check` checksum with the published DMG hash. No deploy key is needed.
