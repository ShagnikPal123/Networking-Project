# Module: audit (AU)

Crate: `crates/relay-audit` · Error prefix: `RLY-AU-<N>`
First built: P2 Trust (audit log).

## Purpose

Append-only, hash-chained log.

## Owns

- Append-only, hash-chained log entries

## Must never

- Edit or delete entries

## Depends on

- store (ST)

## Module checklist

- [ ] Editing the log is detected. Test a tampered entry and expect an `RLY-AU-<N>` error.
- [ ] Only appends are possible. No code path edits or deletes an entry.
- [ ] Any change to the log format is written to disk, so it runs the advanced checklist.

## Memory

### Notes

None yet.
