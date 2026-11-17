# Builder B memory (network crates)

Scope: `relay-discovery`, `relay-link`, `relay-session`, `relay-node`, `relay-cli`.
Read before work, update after work. Delete wrong entries.

## Decisions

- 2026-11-17 · discovery · Beacon is a fixed 176-byte datagram (`wire.rs` has the layout).
  Reason: fixed size makes "drop anything else" a one-line check.
- 2026-11-17 · discovery · Signing goes through `BeaconSigner` / `BeaconVerifier` traits, not a
  direct `relay-identity` call. Reason: identity's sign/verify were still stubs, and discovery
  should never hold a secret key. Node must supply both when it wires discovery up.
- 2026-11-17 · discovery · Default beacon port 47470 (a proposal, not owner-approved).
- 2026-11-17 · discovery · One std thread + one blocking socket with a 50 ms read timeout. No
  async runtime, no new dependencies.

## Gotchas

- Windows returns "connection reset" from `recv_from` on a UDP socket after a send hit a closed
  port. Treat receive errors as non-fatal (done in `udp.rs`).
- "Nothing arrived" is `WouldBlock` on Unix and `TimedOut` on Windows. Match both.
- Tests bind `127.0.0.1:0` and aim beacons at the listener's real port. Never use the default
  port or broadcast in tests: CI runners may block it and parallel tests would collide.

## Open items

- Broadcast to `255.255.255.255` leaves on one interface only on Windows. Per-interface
  broadcast addresses need interface listing (link module); `BeaconConfig.targets` already
  takes a list.
- Beacon content is fixed between `start` and `stop`. A host change (new epoch) needs a
  stop/start or a new `set_beacon` call.
- Only the device ID and address go up as a `PeerSighting`; site ID and epoch from the beacon
  are checked but not reported yet.
- mDNS (design method 1) is not built.
- Two Relay processes on one machine cannot share the default port (std has no SO_REUSEADDR).
