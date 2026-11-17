# Idea agent memory

Owned by the idea agent. Read before work, write after work. Wrong entries get deleted, not
appended to.

## Idea ledger

Status: **proposed** (in `NEXT-UPDATES.md` Ideas), **planned** (owner approved), **rejected** (with
why). Do not re-propose a rejected idea unless something changed.

| Date | Ideas | Status |
|------|-------|--------|
| 2026-11-17 | First 16 ideas written with the vision doc (Android app, multi-hop, messages, dashboard, and the per-area lists) | proposed |
| 2026-11-17 | 18 more from the first research pass: BLE find-me, pinned site pack, needs/offers board, battery-aware handoff, carrier mode, link forecast, watch folder, hash manifest, two-link speed, collect mode, late-joiner catch-up, end-of-event close, Relay spreads Relay, metered-link guard, uplink scheduler, link black box, post-quantum handshake, demo mode | proposed |

Rejected: none yet.

Considered and not proposed: a browser-only guest join (conflicts with the call in
`07-android.md` section 1, "do not build a web client").

## Research log

| Date | Topic | Result | Source checked |
|------|-------|--------|----------------|
| 2026-11-17 | Multipath QUIC | RFC 10069 published Nov 2026. quinn 0.11.12 does not list it. noq (Quinn fork) has it. Recommendation: own failover for P3. | rfc-editor.org, docs.rs/quinn, iroh blog 2026-03-16 |
| 2026-11-17 | Wi-Fi Aware | Android yes (API 26+). Windows no, per a community answer from July 2025. | developer.android.com, Microsoft Q&A |
| 2026-11-17 | No-router link on Windows | Wi-Fi Direct legacy AP (`WiFiDirectLegacySettings`) fits. Mobile Hotspot API is created from a connection profile and its sample needs internet. | Microsoft Learn |
| 2026-11-17 | iroh | 1.0 on 2026-06-15, local-first mode, Kotlin bindings. Recommendation: do not adopt whole. | iroh.computer/blog/v1 |
| 2026-11-17 | rustls | 0.23.46 has RFC 7250 raw public keys and X25519MLKEM768. | docs.rs/rustls |
| 2026-11-17 | mdns-sd | 0.21.7, Windows yes, Android not listed, "still beta". | docs.rs/mdns-sd |
| 2026-11-17 | Bluetooth | btleplug is central only. BLE is for small data. | docs.rs/btleplug, developer.android.com |
| 2026-11-17 | Tauri Android background | tauri-plugin-background-service 1.0.1 exists, young. | docs.rs |
| 2026-11-17 | Store-and-forward prior art | RFC 9171 (Bundle Protocol v7), bitchat (BLE mesh, 7 hops, Noise). | rfc-editor.org, GitHub |

## Not verified yet (do next)

1. Wi-Fi Direct legacy AP with no internet on the owner's hardware. Needs the owner or a test on
   his PCs; cloud agents cannot test radios.
2. quinn + rustls raw public keys: read the quinn crypto config docs.
3. noq: is it published on crates.io, what version.
4. rust-libp2p status.
5. Content-defined chunking for folder sync.
6. Windows official position on Wi-Fi Aware (only a community answer found).

## Waiting on the owner

- Is Bluetooth inside "standard radios only"? Blocks the BLE find-me idea.
- Which impact area to prioritise first after P3.

## Working notes

- GitHub issue search pages cannot be fetched (robots). Use docs.rs, release notes, or project
  blogs for library status.
- Microsoft Learn search returns full API pages and is the fastest primary source for Windows APIs.
- docs.rs `/crate/<name>/latest` gives the version and release date in one read.
- A page saying nothing about a feature is "not listed", not "unsupported". Record it as inferred.
- Scope: this agent writes `NEXT-UPDATES.md` (Ideas, Tech watch, Library candidates) and this
  file. Design changes go to the lead as proposals.
