# Module: share (SH)

Crate: `crates/relay-share` · Error prefix: `RLY-SH-<N>`
First built: P2 Trust (per-folder grants and browsing).

## Purpose

Folder grants, directory listing, and access checks.

## Owns

- Folder grants
- Directory listing
- Access checks

## Must never

- Skip the audit write

## Depends on

- session (SS), audit (AU), store (ST), proto (PR)

## Module checklist

- [ ] Browsing needs a per-folder grant. A request outside the grant is refused with an `RLY-SH-<N>` error.
- [ ] Every grant, browse, and refusal writes to the audit log. Test that the audit entry exists.
- [ ] Access checks run the advanced checklist, including a member reaching outside a grant.

## Memory

### Notes

None yet.
