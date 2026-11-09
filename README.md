# Reoclo desktop

A native macOS client for the Reoclo tunnel daemon. Rust and Rune render the window and menu-bar panel. The app speaks HTTP/1.1 and server-sent events over the daemon's Unix socket. It never stores Reoclo credentials.

Rune and rune-kit are pinned to `938ba14f09bb3381946b3486ef8aa36363a3e8b2`. The matching Rust compiler is `1.99.1`, recorded in [rust-toolchain.toml](rust-toolchain.toml). Dependencies are locked in [Cargo.lock](Cargo.lock).

## Development

```sh
cargo run --bin reoclo-desktop -- --demo
```

Demo mode starts an in-process fake daemon under `.cache/run`, uses an inert system runner, and opens the normal window. It does not start a real tunnel, install a helper, touch the keychain, or register a login item. Quit the app to remove the fake socket.

Headless rendering exits after writing a PNG and an element tree:

```sh
cargo run --bin reoclo-desktop -- \
  --fixture --screen tunnels --state loaded --theme dark \
  --snap snapshots/tunnels.png --tree snapshots/tunnels.json
```

Screens: `tunnels`, `add`, `edit`, `hosts`, `certificates`, `settings`, `panel`.
Data states: `loaded`, `loading`, `empty`, `offline`, `signed-out`, `device-code`, `error`.
Tunnel-state fixtures accept the daemon's twelve state names. `--reduced-motion` forces reduced motion for evidence.

For an explicit fake socket, run the standalone peer with its socket path as the first argument. It prints `READY` after binding; send `quit` on stdin to stop it. Then pass `--socket /path/to/socket` to the app. `--demo` is the safe way to exercise system-action buttons as well, because it injects the fake runner.

Normal launches honor `--socket` and `REOCLO_CONTROL_SOCKET`, then match the CLI's config resolution: `REOCLO_CONFIG_DIR`, `XDG_CONFIG_HOME/reoclo`, or the existing legacy/XDG config location. The socket is `tunnels/run/tunneld.sock` under that directory. Starting the daemon requires an explicit button. Hosts setup explains and invokes the macOS administrator prompt through the bundled CLI. CA trust goes through the control API. Start at login writes only the app's own LaunchAgent and applies at the next macOS login.

This workspace's development compiler and Cargo cache were installed under `.cache` without changing the machine's default toolchain. To use that local installation:

```sh
export RUSTUP_HOME="$PWD/.cache/rustup"
export CARGO_HOME="$PWD/.cache/cargo"
```

## Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
bash scripts/snapshots.sh
scripts/bundle-macos.sh /absolute/path/to/reoclo
```

The snapshot script captures every screen and required data state in both themes, compact and wide windows, tunnel states, scrolled content, and motion frames. `snapshot-check` verifies PNG signatures, viewport dimensions, element roles and forbidden dash characters. It complements visual inspection rather than replacing it.

Tests use real Unix sockets and explicit subscriber signals with bounded timeouts. Rune interaction tests advance its virtual clock. They never use sleeps to synchronize.

The packaging script requires the CLI binary as an input. It creates `dist/Reoclo.app`, signs the copied CLI and app ad hoc, verifies the signature, and creates `dist/Reoclo.dmg`. It neither installs nor launches the app. `hdiutil` may emit a deprecation warning on newer macOS releases.

The CLI included in the initial local artifact is a **development snapshot**, compiled from the phase-two/three CLI worktree on November 9, 2026. Replace it with the matching release binary when distributing the app. The script does not fetch or silently substitute a CLI.

## Layout and ownership

- `src/control*`: bounded blocking HTTP framing, typed errors, chunk decoding, SSE.
- `src/model.rs`, `src/reducer.rs`: tolerant wire types and pure event reduction.
- `src/fake*`, `fixtures/`: deterministic local development peer.
- `src/connection.rs`: off-thread requests and reconnecting event stream.
- `src/ui*`: shared native components, screen state, forms and replies.
- `src/theme.rs`, `assets/theme.json`, `assets/fonts/`: Survey Sheet tokens and licensed dashboard fonts.
- `src/tray.rs`: main-thread tray creation and click-to-panel bridge.
- `src/system*`: injected daemon, login-item and administrator-prompt operations.
- `scripts/`: snapshots, icon generation and packaging.

The visual contract is in [DESIGN.md](DESIGN.md). The main window uses Rune's SplitView, Sidebar and Table. The panel is a 340-point native window. Browser and phone safe-area tests do not apply to this renderer.

## Status

This is an internal build, ad-hoc signed for the team. Menu-bar clicks, device sign-in and the keychain and administrator prompts need a manual run on a real Mac. Developer ID signing and notarization are not set up yet.
