# Module: membership (MB)

Crate: `crates/relay-membership` · Error prefix: `RLY-MB-<N>`
First built: P2 Trust (host-approved joins, roster, revocation). Handoff comes in P3.

## Purpose

Join requests, approvals, roster, host certificate, and handoff.

## Owns

- Join requests and approvals
- Roster and host certificate
- Handoff

## Must never

- Move file data

## Depends on

- session (SS), audit (AU), identity (ID), store (ST), proto (PR)

## Module checklist

- [ ] A join needs host approval and the 6-digit code. Test both.
- [ ] Revocation removes the device from the roster. An unapproved device sees no roster.
- [ ] Handoff happens only with the owner's approval. Test that it never happens without it.

## Memory

### Notes

None yet.
