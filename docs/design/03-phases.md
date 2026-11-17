# Relay: phases P0 to P3

Status: **draft for approval** · 2026-11-17

The first target is **P0 to P3** (approved). Phase contents below are a proposal, since the
original plan file was not available. Each phase ends with a test a person can watch.

```mermaid
flowchart LR
    classDef p0 fill:#073B4C,stroke:#FFD166,color:#FFFFFF,font-weight:bold
    classDef p1 fill:#118AB2,stroke:#1A1A2E,color:#FFFFFF,font-weight:bold
    classDef p2 fill:#06D6A0,stroke:#1A1A2E,color:#1A1A2E,font-weight:bold
    classDef p3 fill:#FF6B35,stroke:#1A1A2E,color:#1A1A2E,font-weight:bold
    classDef rm fill:#FFFFFF,stroke:#999,color:#555,stroke-dasharray: 4 3
    P0[P0 Foundation<br/>find + secure link]:::p0 --> P1[P1 Transfer<br/>resumable files]:::p1 --> P2[P2 Trust<br/>joins, grants, audit]:::p2 --> P3[P3 Self-heal<br/>failover + handoff]:::p3 --> R[Roadmap]:::rm
```

## P0 — Foundation

| | |
|---|---|
| Modules | identity, proto, store (config only), discovery, session, node, cli |
| Builds | Device keys and fingerprints · LAN discovery · one encrypted session between two machines · the `sim` test harness · module health and error codes |
| Trust in P0 | Manual: each side pins the other's fingerprint with a command. Replaced by real joins in P2. |
| Exit test | On two Windows machines: `relay init`, `relay peers` shows the other one, `relay ping <peer>` gets an authenticated encrypted reply. |

## P1 — Transfer

| | |
|---|---|
| Modules | transfer, store (transfer state) |
| Builds | Offer and accept · chunking and hashing · parallel streams · resume from disk · progress events |
| Exit test | Send a 1 GB file. Kill the receiving app halfway. Restart. It resumes from where it stopped and the hash matches. |

## P2 — Trust

| | |
|---|---|
| Modules | membership, share, audit, first desktop UI |
| Builds | Site creation · host-approved joins with the 6-digit code · roster and revocation · per-folder grants · browsing · audit log · approval prompts in a desktop window |
| Exit test | An unapproved device sees no roster and no files. An approved device browses a granted folder, is refused outside it, and both events are in the audit log. Editing the log is detected. |

## P3 — Self-heal

| | |
|---|---|
| Modules | link (multi-path), membership (handoff) |
| Builds | Path discovery across interfaces · heartbeats and scoring · failover · host-run hotspot · planned handoff · deputy takeover |
| Exit test (**the resume demo**) | A transfer runs over Wi-Fi. Wi-Fi is switched off. The transfer continues on Ethernet or the hotspot within 3 seconds and the file hash matches. Then the host is shut down and a deputy becomes host without the owner touching anything. |

## Roadmap (after P3, not designed yet)

- Android app on the same core.
- Multi-hop: members relaying for members that cannot reach each other directly.
- Bluetooth as a low-speed control link for bootstrapping.
- Folder sync, write grants, bandwidth limits.

## What "done" means for any phase

1. Exit test passes on two real Windows machines.
2. The same scenario passes in the `sim` harness in CI.
3. Every new failure mode has an `RLY-` error code and a test that triggers it.
4. The module playbooks and memory in `docs/agent/` are updated.
