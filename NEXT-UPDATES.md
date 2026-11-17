# Relay: next updates

A living file. Updated in the same pull request as the work it describes. The idea agent adds to
the **Ideas** and **Tech watch** sections; only the owner moves an idea into **Planned**.

Last updated: 2026-11-17

## Now

| Item | Phase | Status |
|------|-------|--------|
| Design docs (`docs/design/`) | pre-P0 | Revised after design review, waiting for owner approval |
| Repo skeleton: Rust workspace, one crate per module, CI | P0 | Done |
| Module interfaces for P0 crates, `relay-sim` harness | P0 | Done (interfaces and pure logic only) |
| Agent system files (`CLAUDE.md`, `docs/agent/`) | pre-P0 | Done |
| Test plan and phone plan | pre-P0 | Done (`docs/testing/`, `docs/design/07-android.md`) |
| P0 implementation plan (`docs/plans/2026-11-17-p0-foundation.md`) | P0 | Written, waiting for owner review |
| UI mockup and UI design doc | P2 | Not started (own thread) |
| Tech watch research | pre-P0 | Not started (own thread) |

## Next (P0 to P3, in order)

| # | Update | Phase | Exit test |
|---|--------|-------|-----------|
| 1 | Device identity and fingerprints | P0 | `relay init` prints a stable fingerprint |
| 2 | Wire messages and versioning | P0 | Round-trip tests pass |
| 3 | LAN discovery (mDNS + signed beacon) | P0 | `relay peers` lists the other machine |
| 4 | Encrypted, authenticated session | P0 | `relay ping <peer>` succeeds |
| 5 | `sim` harness: many nodes, scriptable network | P0 | A dropped link can be scripted in a test |
| 6 | Chunked, verified, resumable transfer | P1 | 1 GB file resumes after the app is killed |
| 7 | Site creation and host-approved joins | P2 | Unapproved device sees nothing |
| 8 | Folder grants, browsing, audit log | P2 | Out-of-grant read is refused and logged |
| 9 | Desktop app window (Tauri) | P2 | Approve a join from the window |
| 10 | Multi-path link manager and failover | P3 | Transfer survives Wi-Fi being switched off |
| 11 | Host hotspot mode | P3 | Site works with no router |
| 12 | Host handoff and deputies | P3 | Deputy takes over when the host is shut down |

## Planned after P3

Nothing yet. Items move here from **Ideas** when the owner approves them.

## Ideas (not approved, not scheduled)

Each idea names the impact area it serves (see `docs/design/05-vision.md`).

| Idea | Serves | Size | Added |
|------|--------|------|-------|
| Android app on the same Rust core | All | L | 2026-11-17 |
| Multi-hop relaying between members | Outages, field teams | L | 2026-11-17 |
| Messages and presence | All | M | 2026-11-17 |
| Site dashboard with exportable audit report | All | M | 2026-11-17 |
| Store-and-forward messages for devices that come and go | Outages | M | 2026-11-17 |
| "I'm safe" check-in board and host broadcast notice | Outages | S | 2026-11-17 |
| Low-power mode (slower heartbeats on battery) | Outages | S | 2026-11-17 |
| Folder sync | Field teams | L | 2026-11-17 |
| Transfer queue with priorities and bandwidth limits | Field teams | M | 2026-11-17 |
| Delivery receipts signed by the receiver | Field teams | S | 2026-11-17 |
| Guest role with time-limited membership | Classrooms, events | S | 2026-11-17 |
| One-to-many send | Classrooms, events | M | 2026-11-17 |
| Drop-box folder (write-only grant) | Classrooms, events | S | 2026-11-17 |
| QR-code join | Classrooms, events | S | 2026-11-17 |
| Share one device's internet with the site | Low-connectivity | L | 2026-11-17 |
| Local cache of common files, sync when uplink returns | Low-connectivity | M | 2026-11-17 |

## Tech watch (to be verified by the idea agent)

Nothing here is confirmed. Each line is a question to research, with sources, before it can
influence the design.

| Topic | Question | Checked |
|-------|----------|---------|
| Multipath QUIC | Is the IETF multipath extension usable in Rust QUIC libraries yet, and would it replace our own failover? | No |
| Wi-Fi Aware / Wi-Fi Direct | Can Windows and Android form a link with no router and no hotspot using these? | No |
| Rust peer-to-peer libraries | Would an existing library cover discovery and sessions well enough to shrink P0? | No |
| Tauri on Android | What are the current limits for background networking in a Tauri mobile app? | No |
| Windows hotspot control | Which Windows API lets an app start a hotspot, and what permissions does it need? | No |

## Changelog

| Date | Change |
|------|--------|
| 2026-11-17 | File created. Stack chosen: Rust core + Tauri app. Design docs drafted. |
| 2026-11-17 | Workspace skeleton, P0 interfaces, sim harness, agent files, test plan, phone plan added. |
| 2026-11-17 | Protocol revised after design review: commit-then-reveal join code, deputy certificates bound to host and epoch, failover at Suspect with warm standby, per-chunk hash list, signed audit checkpoints, trust policy pushed down into session. |
| 2026-11-17 | P0 implementation plan written: 10 tasks, library versions checked on crates.io. |
