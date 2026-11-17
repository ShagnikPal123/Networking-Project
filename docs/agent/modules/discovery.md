# Module: discovery (DS)

Crate: `crates/relay-discovery` · Error prefix: `RLY-DS-<N>`
First built: P0 Foundation (LAN discovery).

## Purpose

Finding sites and peers on the LAN.

## Owns

- Finding sites on the LAN
- Finding peers on the LAN

## Must never

- Decide who is trusted

## Depends on

- identity (ID), proto (PR)

## Module checklist

- [ ] Discovery only reports that a site or peer exists. It grants no trust.
- [ ] Discovery down does not stop existing sessions or transfers. Test it.
- [ ] Failures report `Degraded` or `Down` to node, with an `RLY-DS-<N>` code.

## Memory

### Notes

None yet.
