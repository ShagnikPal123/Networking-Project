# Relay

A portable, self-healing local network with file transfer for one site. Works offline. The host
approves every join, the owner approves host handoff, and folder browsing needs a per-folder grant
and is audit-logged. Windows first, then Android. Rust core, Tauri app.

- Design: [`docs/design/`](docs/design/00-overview.md)
- What comes next: [`NEXT-UPDATES.md`](NEXT-UPDATES.md)

- P0 plan: [`docs/plans/2026-11-17-p0-foundation.md`](docs/plans/2026-11-17-p0-foundation.md)
- For AI agents: [`CLAUDE.md`](CLAUDE.md)

Status: design and P0 plan drafted, waiting for approval. The workspace has module interfaces and
tested pure logic (frame codec, fingerprints, path scoring, health board, network simulator). No
networking yet.

```
cargo test --workspace
cargo run -p relay-cli -- health
```
