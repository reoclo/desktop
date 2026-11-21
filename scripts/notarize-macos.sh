#!/bin/bash
# Notarize the disk image and staple it and the app that the update archive carries. Never print the credentials.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ASC_KEY_PATH:?ASC_KEY_PATH must name the App Store Connect API key file}"
: "${ASC_KEY_ID:?ASC_KEY_ID must be set}"
: "${ASC_ISSUER_ID:?ASC_ISSUER_ID must be set}"
: "${APPLE_TEAM_ID:?APPLE_TEAM_ID must be set}"
app=dist/Reoclo.app
dmg=dist/Reoclo.dmg
[[ -d "$app" && -f "$dmg" ]] || { printf 'Run scripts/bundle-macos.sh first\n' >&2; exit 2; }

team_of() { codesign -dv "$1" 2>&1 | sed -n 's/^TeamIdentifier=//p'; }
for code in "$app" "$app/Contents/Resources/reoclo" "$dmg"; do
  team="$(team_of "$code")"
  [[ "$team" == "$APPLE_TEAM_ID" ]] || {
    printf '%s is signed by team %s, expected %s\n' "$code" "${team:-none}" "$APPLE_TEAM_ID" >&2
    exit 1
  }
done

auth=(--key "$ASC_KEY_PATH" --key-id "$ASC_KEY_ID" --issuer "$ASC_ISSUER_ID")
id="$(xcrun notarytool submit "$dmg" "${auth[@]}" --output-format json | jq -r '.id // empty')"
[[ -n "$id" ]] || { printf 'The notary service returned no submission id\n' >&2; exit 1; }
printf 'Submitted %s\n' "$id"
# A dropped status poll must not fail a release, so the wait is retried on the same submission.
status=
for attempt in 1 2 3; do
  if result="$(xcrun notarytool wait "$id" "${auth[@]}" --timeout 45m --output-format json)"; then
    status="$(jq -r '.status // empty' <<< "$result")"
  else
    status="$(xcrun notarytool info "$id" "${auth[@]}" --output-format json | jq -r '.status // empty')" || status=
  fi
  [[ -n "$status" && "$status" != "In Progress" ]] && break
  printf 'No final status for %s yet (attempt %s)\n' "$id" "$attempt" >&2
done
printf 'Notarization %s: %s\n' "$id" "${status:-unknown}"
if [[ "$status" != Accepted ]]; then
  xcrun notarytool log "$id" "${auth[@]}" >&2 || true
  exit 1
fi

xcrun stapler staple "$dmg"
xcrun stapler staple "$app"
xcrun stapler validate "$dmg"
xcrun stapler validate "$app"
assessment="$(spctl --assess --type execute --verbose=2 "$app" 2>&1)"
printf '%s\n' "$assessment"
grep -q 'source=Notarized Developer ID' <<< "$assessment" || {
  printf 'Gatekeeper does not accept %s as notarized\n' "$app" >&2
  exit 1
}
shasum -a 256 "$dmg"
printf 'NOTARIZE PASS: %s and %s\n' "$dmg" "$app"
