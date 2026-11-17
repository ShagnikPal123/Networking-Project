# Module: proto (PR)

Crate: `crates/relay-proto` · Error prefix: `RLY-PR-<N>`
First built: P0 Foundation.

## Purpose

Message types, encoding, and version numbers.

## Owns

- Message types, encoding, and version numbers

## Must never

- Do any I/O

## Depends on

Nothing. Proto sits at the base.

## Module checklist

- [ ] Every wire message change bumps the version number.
- [ ] Old peers are still handled, and `proto` tests cover the change.
- [ ] Network input is size-limited and validated before use.

## Memory

### Notes

None yet.
