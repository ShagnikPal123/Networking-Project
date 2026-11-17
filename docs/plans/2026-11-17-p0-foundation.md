# Relay P0 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two Windows machines run `relay init`, see each other with `relay peers`, pin each other with `relay trust`, and `relay ping <peer>` gets an authenticated, encrypted reply.

**Architecture:** One Rust crate per module, dependencies pointing downward only. Discovery finds addresses, link picks the (single) path, session runs QUIC with both sides proving their device keys and a trust policy handed down from above, node wires it together, the CLI talks only to node.

**Tech Stack:** Rust 1.80+, tokio, quinn (QUIC), rustls + rcgen (TLS with device keys), ed25519-dalek, blake3, mdns-sd, directories. Versions seen on crates.io on 2026-11-17: ed25519-dalek 3.0.0, blake3 1.8.7, quinn 0.11.12, rcgen 0.14.10, tokio 1.54.1, mdns-sd 0.21.7, rand 0.10.4, directories 6.0.0.

**Spec:** `docs/design/01-architecture.md`, `docs/design/02-protocol.md` (sections 1, 2, 4, 9), `docs/design/03-phases.md` (P0). Test levels: `docs/testing/test-plan.md`.

## Global Constraints

- Follow `CLAUDE.md` and run `docs/agent/checklists/basic.md` for every task. Tasks 1, 6, 7 and 8 also run `advanced.md` and need lead-tier review (keys, wire messages, trust).
- No new dependency without adding it to `[workspace.dependencies]` in the root `Cargo.toml`. Before using a library, read its docs.rs page for the exact version cargo resolved; do not write its API from memory.
- Every error variant's `Display` starts with its `RLY-<MODULE>-<NNN>` code; numbers are never reused; each crate's error-code test covers every variant.
- `unsafe` is forbidden. Every public item is documented. CI runs `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` on Windows and Linux.
- Tests never sleep on the real clock for logic; time is passed in as a value. Network integration tests use `127.0.0.1` and port `0` (OS-assigned).
- Secrets never appear in logs, `Debug` output, or error text.
- Relay never changes system settings. P0 adds no firewall rules; it prints a hint when no traffic arrives.
- Default UDP port: `47800` for sessions, `47801` for beacons. Both overridable (`--port`, `RELAY_PORT`).
- Data directory: `RELAY_HOME` if set, otherwise the platform config dir from `directories` (`ProjectDirs::from("", "", "Relay")`).

## Review Focus

1. **Two devices started from a copied data directory (same key).** Expected: `relay peers` marks the peer as "same identity as this device" and `ping` refuses with `RLY-SS-006`. Test in Task 7.
2. **Windows Firewall silently drops inbound UDP.** Expected: `relay peers` finding nothing after its wait prints a hint naming the port and firewall, exit code 0. Test in Task 9.
3. **Machine with several network interfaces (Ethernet + Wi-Fi + virtual adapters).** Expected: beacons go out on every IPv4 broadcast-capable interface and the same peer seen twice is listed once. Test in Task 5.
4. **Garbage or oversized UDP packets on the beacon port.** Expected: ignored without error, never a panic, counter incremented. Test in Task 4.
5. **Corrupt or half-written key file (power cut during `relay init`).** Expected: `RLY-ID-006 key file unreadable`, never a silent new identity. Test in Tasks 1 and 2.

---

### Task 1: Real device keys (relay-identity)

**Files:** Modify `crates/relay-identity/src/lib.rs`, `crates/relay-identity/Cargo.toml`, root `Cargo.toml` (add `ed25519-dalek`, `blake3`, `rand`).

**Interfaces:**
- Produces: `generate() -> Result<DeviceKeyPair>`; `DeviceKeyPair::to_bytes(&self) -> [u8; 32]` (secret seed); `DeviceKeyPair::from_bytes(&[u8]) -> Result<DeviceKeyPair>`; `device_id(&PublicKey) -> DeviceId` (now infallible: BLAKE3 of the 32 key bytes); `sign(&DeviceKeyPair, &[u8]) -> Signature`; `verify(&PublicKey, &[u8], &Signature) -> Result<()>`. New error `KeyUnreadable = 6`.

- [ ] **Step 1: failing tests**
  - `sign_then_verify_ok`: verify(pk, b"hello", sign(kp, b"hello")) is `Ok`.
  - `verify_rejects_changed_message` and `verify_rejects_other_key`: both `Err(Error::BadSignature)`.
  - `keypair_round_trips`: `from_bytes(&kp.to_bytes())` has the same `public()`.
  - `from_bytes_rejects_wrong_length`: 31 and 33 bytes give `Err(Error::KeyUnreadable)`.
  - `device_id_is_stable`: same key gives same ID twice; two generated keys give different IDs.
  - `debug_hides_secret`: `format!("{kp:?}")` does not contain the hex of `to_bytes()`.
- [ ] **Step 2:** run `cargo test -p relay-identity`, expect the new tests to fail.
- [ ] **Step 3:** implement with ed25519-dalek for keys and signatures, blake3 for `device_id`. Remove the `NotImplemented` returns from these five functions; `verify_chain` stays a stub (P2).
- [ ] **Step 4:** run tests and clippy for the crate, expect pass.
- [ ] **Step 5:** commit `feat(identity): real Ed25519 device keys and device IDs`.

### Task 2: File store with atomic writes (relay-store)

**Files:** Modify `crates/relay-store/src/lib.rs`.

**Interfaces:**
- Produces: working `FileStore` implementing `Store`. Layout: `<root>/<namespace>/<name>`. `put` writes `<name>.tmp`, flushes (`sync_all`), then renames over the target. New errors `Io { op, detail } = 3`.

- [ ] **Step 1: failing tests** (use a temp dir under `std::env::temp_dir()` with a unique name, removed at the end)
  - `file_put_get_delete_round_trip`.
  - `put_replaces_atomically`: after two puts the value is the second and no `.tmp` file remains.
  - `leftover_tmp_is_ignored`: a stray `<name>.tmp` with garbage does not affect `get`.
  - `get_missing_is_none`, `delete_missing_is_false`.
  - `values_survive_reopen`: a new `FileStore` on the same root reads the value.
- [ ] **Step 2:** run, expect fail. **Step 3:** implement. **Step 4:** run, expect pass.
- [ ] **Step 5:** commit `feat(store): file-backed store with atomic writes`.

### Task 3: Hello and Ping payloads (relay-proto)

**Files:** Create `crates/relay-proto/src/payload.rs`; modify `crates/relay-proto/src/lib.rs` (`pub mod payload;`).

**Interfaces:**
- Produces, all with `encode(&self) -> Vec<u8>` and `decode(&[u8]) -> Result<Self>`; fixed big-endian layouts documented in the module doc:
  - `Hello { protocol: u16, device: [u8; 32], nonce: [u8; 16] }` (50 bytes)
  - `HelloAck { protocol: u16, device: [u8; 32], nonce_echo: [u8; 16] }` (50 bytes)
  - `Ping { seq: u32, sent_ms: u64 }` and `Pong { seq: u32, sent_ms: u64 }` (12 bytes)
- New error `BadPayload { frame_type: u16, len: usize } = 5`.

- [ ] **Step 1: failing tests:** round trip for each type; decode of a buffer one byte short and one byte long gives `BadPayload`; `Hello` wrapped in `Frame::new(FrameType::Hello, ..)` survives `encode` then `decode`.
- [ ] **Steps 2 to 4:** fail, implement, pass.
- [ ] **Step 5:** commit `feat(proto): hello and ping payloads`.

### Task 4: Signed beacon codec and sighting table (relay-discovery, pure)

**Files:** Create `crates/relay-discovery/src/beacon.rs`, `crates/relay-discovery/src/table.rs`; modify `lib.rs`, `Cargo.toml` (add `relay-identity`).

**Interfaces:**
- Consumes: `relay_identity::{sign, verify, device_id, DeviceKeyPair, PublicKey, Signature}`.
- Produces:
  - `SignedBeacon { beacon: Beacon, key: PublicKey, signature: Signature }` with `SignedBeacon::create(&Beacon, &DeviceKeyPair) -> SignedBeacon`, `encode(&self) -> Vec<u8>`, `decode_and_verify(&[u8]) -> Result<SignedBeacon, Error>`. Layout: magic `b"RLY1"`, version u8, site flag u8 + 32 bytes (zeros when `None`), device 32, port u16, key 32, signature 64. Verification checks the signature and that `device_id(&key) == beacon.device`.
  - `SightingTable::new(ttl_ms: u64)`, `observe(&mut self, PeerSighting, now_ms: u64)`, `current(&self, now_ms: u64) -> Vec<PeerSighting>` (one entry per device, newest address wins, entries older than `ttl_ms` dropped), `ignored(&self) -> u64` plus `note_ignored(&mut self)`. TTL default constant `SIGHTING_TTL_MS = 6_000` (three missed beacons).
- New errors: `BadBeacon { reason } = 5`.

- [ ] **Step 1: failing tests**
  - `beacon_round_trip_with_and_without_site`.
  - `rejects_bad_magic`, `rejects_truncated`, `rejects_oversized` (anything not exactly the layout length), `rejects_flipped_bit_in_port` (signature fails), `rejects_key_not_matching_device`.
  - `decode_never_panics_on_garbage`: loop over 1,000 pseudo-random buffers of length 0 to 300 from a fixed-seed xorshift; every call returns `Err`.
  - `table_dedupes_by_device`, `table_expires_after_ttl`, `table_newest_address_wins`.
- [ ] **Steps 2 to 4:** fail, implement, pass.
- [ ] **Step 5:** commit `feat(discovery): signed beacon codec and sighting table`.

### Task 5: Beacon over UDP and mDNS (relay-discovery, I/O)

**Files:** Create `crates/relay-discovery/src/udp.rs`, `crates/relay-discovery/src/mdns.rs`; modify `lib.rs`, `Cargo.toml` (add `tokio`, `mdns-sd`).

**Interfaces:**
- Consumes: Task 4.
- Produces: `LanDiscovery::new(config: DiscoveryConfig, key: DeviceKeyPair) -> LanDiscovery` implementing `Discovery`; `DiscoveryConfig { session_port: u16, beacon_port: u16, enable_mdns: bool, targets: Vec<SocketAddr> }`. Empty `targets` means "broadcast address of every IPv4 interface"; tests pass explicit loopback targets. Own beacons (same device ID) are dropped. mDNS advertises `_relay._udp.local` with TXT `id=<device id hex>` and feeds the same table with `DiscoveryMethod::Mdns`. If mDNS fails to start, discovery reports `Health::Degraded` with `RLY-DS-006 mdns unavailable` and keeps running on beacons.

- [ ] **Step 1: failing tests** (tokio test runtime, loopback, port 0)
  - `two_instances_see_each_other`: A and B each target the other's beacon port; within 3 beacon intervals `A.sightings()` contains B's device ID and B's session port.
  - `own_beacon_is_ignored`.
  - `garbage_packet_is_counted_not_fatal`: send 10 random packets to A's beacon port; `sightings()` still works and the ignored counter is 10.
  - `same_peer_on_two_addresses_listed_once`.
  - `mdns_failure_degrades_not_fails` (inject a failing mDNS starter through a small trait).
- [ ] **Steps 2 to 4:** fail, implement, pass. Interval is injectable so tests use 50 ms.
- [ ] **Step 5:** commit `feat(discovery): UDP beacon and mDNS back ends`.

### Task 6: Single-path link manager (relay-link)

**Files:** Modify `crates/relay-link/src/lib.rs`.

**Interfaces:**
- Produces: `LinkManager::new()`, `learn(&mut self, peer: DeviceId, addr: SocketAddr, kind: LinkKind) -> PathId`, `forget_peer(&mut self, peer: DeviceId)`, `active_path(&self, peer: DeviceId) -> Result<(PathId, Path), Error>` (returns `NoUsablePath` when none), `report(&mut self, path: PathId, event: PathEvent)`. In P0 each peer keeps exactly one path (the newest address); the struct already stores a `Vec` so P3 can add more without changing the interface.

- [ ] **Step 1: failing tests:** `unknown_peer_has_no_path`; `learned_path_is_active`; `new_address_replaces_old`; `reported_down_path_is_not_returned`; `interface_back_makes_it_usable_again`.
- [ ] **Steps 2 to 4:** fail, implement, pass.
- [ ] **Step 5:** commit `feat(link): single-path link manager`.

### Task 7: Trust policy and QUIC session (relay-session)

**Files:** Create `crates/relay-session/src/trust.rs`, `crates/relay-session/src/tls.rs`, `crates/relay-session/src/quic.rs`; modify `lib.rs`, `Cargo.toml` (add `relay-identity`, `relay-proto`, `tokio`, `quinn`, `rustls`, `rcgen`).

**Interfaces:**
- Consumes: Tasks 1, 3, 6.
- Produces:
  - `trait TrustPolicy: Send + Sync { fn allows(&self, peer: &DeviceId) -> bool; }`
  - `PinnedPeers::new(ids: impl IntoIterator<Item = DeviceId>)` implementing `TrustPolicy`.
  - `Endpoint::bind(addr: SocketAddr, key: DeviceKeyPair, trust: Arc<dyn TrustPolicy>) -> Result<Endpoint, Error>`, `local_addr()`, `async connect(&self, peer: DeviceId, addr: SocketAddr) -> Result<QuicSession, Error>`, `async accept(&self) -> Result<QuicSession, Error>`.
  - `QuicSession` implementing `Session`, plus `async ping(&mut self, seq: u32) -> Result<Duration, Error>`; the accepting side answers `Ping` with `Pong` on the control stream.
- How identity is proven: each side presents a self-signed certificate made from its device key (rcgen). A custom rustls verifier on **both** client and server accepts any certificate shape, extracts the Ed25519 public key, computes `device_id`, and rejects unless (a) on connect it equals the expected `peer`, and (b) `trust.allows(&id)`. The TLS handshake itself proves key possession. ALPN is `relay/1`.
- New errors: `PeerNotTrusted { peer } = 4`, `WrongPeer { expected, got } = 5`, `SameIdentity = 6`, `Handshake { detail } = 7`, `Timeout = 8`.

- [ ] **Step 1: failing tests** (loopback, port 0)
  - `trusted_peers_connect_and_ping`: both pin each other; `ping(1)` returns `Ok` and session state is `Established`.
  - `unpinned_client_is_rejected`: server does not pin the client; connect fails and the server never yields a session.
  - `client_rejects_unpinned_server`.
  - `wrong_peer_id_is_rejected`: connect expecting ID X to an address served by Y gives `WrongPeer`.
  - `same_identity_is_refused`: both ends use the same key; `SameIdentity`.
  - `no_frames_before_authentication`: a raw QUIC client with an untrusted key can open no stream that reaches application code.
  - `connect_to_dead_address_times_out` within the configured 2 s timeout (injectable; test uses 200 ms).
- [ ] **Steps 2 to 4:** fail, implement, pass.
- [ ] **Step 5:** commit `feat(session): QUIC sessions authenticated by device keys with pinned trust`.

### Task 8: Node wiring (relay-node)

**Files:** Create `crates/relay-node/src/node.rs`, `crates/relay-node/src/paths.rs`; modify `lib.rs`, `Cargo.toml` (add `relay-identity`, `relay-store`, `relay-link`, `tokio`, `directories`).

**Interfaces:**
- Consumes: Tasks 1, 2, 5, 6, 7.
- Produces:
  - `data_dir() -> PathBuf` (rule in Global Constraints).
  - `Node::init(store: Arc<dyn Store>) -> Result<InitOutcome, Error>` where `InitOutcome { fingerprint: Fingerprint, created: bool }`; idempotent, never overwrites an existing key; unreadable key gives `RLY-ID-006` wrapped, not a new key.
  - `Node::open(store, NodeConfig) -> Result<Node, Error>`; `NodeConfig { session_port, beacon_port, enable_mdns, discovery_targets }`.
  - `async peers(&self, wait: Duration) -> Vec<PeerInfo>` with `PeerInfo { device: DeviceId, fingerprint: Fingerprint, addr: SocketAddr, via: DiscoveryMethod, trusted: bool, is_self_identity: bool }`.
  - `trust(&self, fp: &Fingerprint, candidates: &[PeerInfo]) -> Result<DeviceId, Error>`: stores the **full device ID** of the one discovered peer matching the fingerprint under store key `trust/<device id hex>`; zero or two matches is an error (`UnknownFingerprint = 3`, `AmbiguousFingerprint = 4`).
  - `async serve(&self, stop: impl Future<Output = ()>)` and `async ping(&self, fp: &Fingerprint) -> Result<Duration, Error>`.
  - `health(&self) -> HealthBoard`; discovery and session failures update it and emit `Event::HealthChanged`.
- Store keys: `identity/device.key`, `trust/<hex>`.

- [ ] **Step 1: failing tests** (two nodes in one process, `MemoryStore`, loopback, explicit discovery targets)
  - `init_is_idempotent`: second call returns `created: false` and the same fingerprint.
  - `init_refuses_corrupt_key`: store holds 5 bytes at `identity/device.key`; error code prints `RLY-ID-006`; the stored bytes are unchanged.
  - `peers_lists_other_node`, `self_identity_is_flagged` (second node opened on a copy of the first node's store).
  - `ping_requires_trust_both_ways`: untrusted ping fails with `RLY-SS-004`; after both `trust`, ping returns `Ok`.
  - `discovery_down_does_not_stop_ping`: disable discovery after trust and supply the address manually; ping still works and the health strip shows `DS ○`.
- [ ] **Steps 2 to 4:** fail, implement, pass.
- [ ] **Step 5:** commit `feat(node): init, peers, trust, serve, ping`.

### Task 9: CLI commands and the exit test (relay-cli)

**Files:** Modify `crates/relay-cli/src/main.rs`, `crates/relay-cli/Cargo.toml` (add `tokio`, `relay-identity`, `relay-store`); create `crates/relay-cli/tests/p0_exit.rs`; modify `README.md`, `NEXT-UPDATES.md`, `docs/testing/test-plan.md` (real command names).

**Interfaces:**
- Consumes: Task 8.
- Produces these commands (exit 0 on success, 1 on a Relay error printed as `RLY-..` on stderr, 64 on usage):
  - `relay init` prints `Device fingerprint: K7QF-2M9X` (and `already initialised` on repeat).
  - `relay peers [--wait <seconds, default 3>]` prints one line per peer: fingerprint, address, `trusted`/`untrusted`, method. With none found: `No peers found. If another device is running "relay serve", check that UDP port 47801 is allowed through the firewall on both machines.`
  - `relay trust <fingerprint>` runs a discovery wait, then pins.
  - `relay serve` runs until Ctrl-C, printing the health strip on change.
  - `relay ping <fingerprint>` prints `Reply from K7QF-2M9X in 4 ms (encrypted, authenticated)`.
  - Global flags `--home <dir>`, `--port <n>`, `--beacon-port <n>`, hidden `--discovery-target <addr>` for tests.

- [ ] **Step 1: failing tests**
  - Unit: argument parsing for every command and flag; bad fingerprint gives exit 64 and `RLY-ID-005` text.
  - `tests/p0_exit.rs` `p0_exit_test_on_loopback`: two temp homes; spawn `relay serve` for each with distinct ports and each other as discovery target; run `init`, `peers`, `trust` both ways, then `ping`; assert exit 0 and stdout contains `encrypted, authenticated`. Kill both children at the end even on failure.
  - `peers_with_nobody_prints_firewall_hint`.
- [ ] **Steps 2 to 4:** fail, implement, pass on Linux locally; CI proves Windows.
- [ ] **Step 5:** update `README.md` quick start, mark P0 rows done in `NEXT-UPDATES.md`, fix command names in the test plan, commit `feat(cli): P0 commands and loopback exit test`.

### Task 10: Sim scenario for discovery

**Files:** Create `crates/relay-sim/tests/p0_discovery.rs`; modify `crates/relay-sim/Cargo.toml` (dev-dependencies `relay-discovery`, `relay-identity`).

**Interfaces:**
- Consumes: Task 4 (`SignedBeacon`, `SightingTable`) and `SimNet`.

- [ ] **Step 1: failing tests** carrying encoded beacons as `SimNet` packets
  - `peer_appears_within_one_interval_and_expires_after_link_down`: link up, beacon every 2,000 ms, peer listed by 2,100 ms; set link down at 5,000 ms; peer gone by 11,100 ms.
  - `slow_link_still_discovers` (delay 800 ms).
  - `killed_node_disappears`.
- [ ] **Steps 2 to 4:** fail, implement, pass. **Step 5:** commit `test(sim): P0 discovery scenarios`.

---

## Order and parallel work

```
Task 1 ─┬─ Task 4 ── Task 5 ─┐
Task 2 ─┤                    ├─ Task 8 ── Task 9
Task 3 ─┼─ Task 7 ───────────┤
Task 6 ─┘        Task 4 ── Task 10
```

Tasks 1, 2, 3 and 6 touch different crates and can run at the same time. Task 7 is the riskiest (custom certificate checking) and gets the strongest tier plus a security review.

## Known P0 limits (accepted, fixed in P2)

- An 8-character fingerprint is 40 bits. `relay trust` pins the full device ID of a peer that is on the network at that moment, so an attacker would need to be present and to have ground out a matching key. P2's join code replaces this.
- No daemon: `relay serve` must be running on the machine being pinged.
- The real-hardware exit test (two Windows PCs) is run by the owner by hand, following `docs/testing/test-plan.md`.
