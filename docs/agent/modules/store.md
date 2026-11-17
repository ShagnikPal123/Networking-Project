# Module: store (ST)

Crate: `crates/relay-store` · Error prefix: `RLY-ST-<N>`
First built: P0 Foundation (config only). Transfer state comes in P1.

## Purpose

Local files: config, roster, and transfer state.

## Owns

- Config files, roster file, and transfer state

## Must never

- Know what the data means

## Depends on

Nothing. Store sits at the base.

## Module checklist

- [ ] Any change to what is written to disk runs the advanced checklist.
- [ ] Old data still loads, or a migration exists and is tested.
- [ ] Store holds bytes and state only. It does not interpret them.

## Memory

### Notes

None yet.
