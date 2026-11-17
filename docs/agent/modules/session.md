# Module: session (SS)

Crate: `crates/relay-session` · Error prefix: `RLY-SS-<N>`
First built: P0 Foundation (one encrypted session between two machines).

## Purpose

One encrypted, authenticated connection per peer, with reconnect and resume.

## Owns

- One encrypted, authenticated connection per peer
- Reconnect and resume

## Must never

- Know about files or folders

## Depends on

- link (LK), identity (ID), proto (PR)

## Module checklist

- [ ] Each peer has one encrypted, authenticated connection. Test that no data passes before authentication.
- [ ] A link drop triggers reconnect and resume without the user redoing anything.
- [ ] A device that was never approved learns nothing beyond "a Relay site exists here".

## Memory

### Notes

None yet.
