# Reoclo desktop

Reoclo desktop is a macOS thin client for the local tunnel daemon. DevOps engineers use it to bring saved tunnels up and down, open endpoints, review agent requests, and inspect local hostnames and certificate trust.

The primary action is bringing a saved tunnel up. The daemon owns credentials, configuration, networking, certificate material, and privileged operations. The desktop owns presentation and user intent.

This internal build is ad-hoc signed. Public signing and notarization require the owner's Apple Developer account.

The implementation brief is REO-627, dated November 9, 2026. Development uses a fake daemon exclusively. No real tunnel daemon, system helper, keychain changes, or login registration is exercised during development.
