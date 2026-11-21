# Reoclo for macOS

Reoclo for macOS is a thin client for the Reoclo tunnel daemon on a Mac. DevOps engineers use it to bring saved tunnels up and down, open endpoints, review agent requests, and inspect local hostnames and certificate trust.

The primary action is bringing a saved tunnel up. The daemon owns credentials, configuration, networking, certificate material, and privileged operations. The desktop owns presentation and user intent.

Releases are signed with a Developer ID and notarized by Apple.

Development and the test suite use a fake daemon. Nothing in them starts a real tunnel, installs a system helper, changes the keychain or registers a login item.
