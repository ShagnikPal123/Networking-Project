# Relay: design overview

Status: **draft for approval** · 2026-11-17

> The originally approved `DESIGN.md` lives on the owner's PC and was not available when this was
> written. These docs are built from the approved points below. Anything marked **(call)** is a
> proposal made here, not something the owner has agreed to yet.

## What Relay is

A portable, self-healing local network with file transfer for **one site** (a home, an office, an
event, a field team). It keeps working with no internet. If a link drops, traffic moves to another
link. If the host drops, an owner-approved device takes over.

**Why it exists:** it is the owner's resume project. The headline demo is a file transfer between
two machines that keeps going when the link under it is pulled.

## Approved points (fixed)

| # | Decision |
|---|----------|
| 1 | Standard radios only: Ethernet, Wi-Fi, hotspot. No custom hardware. |
| 2 | First target is **P0 to P3**. Everything else is roadmap. |
| 3 | Windows first, then Android. |
| 4 | The name is "Relay". |
| 5 | The host approves every join. |
| 6 | The owner approves host handoff. |
| 7 | File browsing needs a per-folder grant and is written to an audit log. |
| 8 | Works offline. |

## Goals

- Two Windows machines find each other on a LAN with no setup beyond approving the join.
- Files move fast, resume after any interruption, and are verified end to end.
- A dropped link or a dead host heals without the user redoing anything.
- Every failure points at exactly one module (see `01-architecture.md`, error codes).

## Non-goals (for P0 to P3)

- Internet relay, cloud accounts, or NAT traversal across sites.
- More than one site per install.
- Multi-hop mesh routing (roadmap).
- Android app (roadmap, but the core must not block it).

## Anti-goals

- A device that was never approved can read or learn anything beyond "a Relay site exists here".
- A transfer that silently delivers a corrupt file.
- A handoff that happens without the owner's approval, even "to be helpful".
- A design where one module failing takes down modules that do not depend on it.

## Approaches considered

```
A. Rust core + Tauri app          B. Go core + Flutter app        C. Kotlin everywhere
┌──────────┐  ┌──────────┐        ┌──────────┐  ┌──────────┐      ┌──────────────────────┐
│ Windows  │  │ Android  │        │ Windows  │  │ Android  │      │ Compose Multiplatform │
│ Tauri UI │  │ Tauri UI │        │ Flutter  │  │ Flutter  │      │   Windows + Android   │
└────┬─────┘  └────┬─────┘        └────┬─────┘  └────┬─────┘      └──────────┬───────────┘
     └──────┬──────┘                   │  bridge     │ bridge                │
     ┌──────┴──────┐              ┌────┴─────────────┴────┐       ┌──────────┴───────────┐
     │  Rust core  │              │        Go core        │       │     Kotlin core      │
     └─────────────┘              └───────────────────────┘       └──────────────────────┘
```

| | A. Rust + Tauri | B. Go + Flutter | C. Kotlin |
|---|---|---|---|
| Shared core on Windows and Android | Yes, same crate | Yes, through a bridge | Yes |
| Low-level networking (interfaces, QUIC, failover) | Strong | Strong | Weaker on Windows |
| Extra failure points | None | UI to core bridge | JVM on Windows |
| Speed to first demo | Slower | Fastest | Medium |
| Resume signal | Strongest | Good | Good for Android roles |

**Chosen: A, Rust core + Tauri app** (owner's decision, 2026-11-17). One Rust core serves both
platforms with no bridge to fail, and Rust has mature QUIC and crypto libraries.

## Document map

| File | Covers |
|------|--------|
| `01-architecture.md` | Modules, boundaries, dependency rule, error codes |
| `02-protocol.md` | Identity, discovery, join, session, transfer, failover, handoff, browsing |
| `03-phases.md` | P0 to P3 scope and exit tests, roadmap |
| `04-agent-system.md` | How AI agents work on this repo: playbooks, checklists, memory, the idea agent |
| `05-vision.md` | End goal, where Relay can matter, features aimed at that |
| `../../NEXT-UPDATES.md` | Living list of what comes next, kept current as work lands |
