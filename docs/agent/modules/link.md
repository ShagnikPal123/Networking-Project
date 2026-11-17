# Module: link (LK)

Crate: `crates/relay-link` · Error prefix: `RLY-LK-<N>`
First built: P0 with one path only. P3 adds multi-path, warm standby, and failover.

## Purpose

Network interfaces, paths to a peer, probing, and picking the best path.

## Owns

- Network interfaces and paths to a peer
- Probing and picking the best path

## Must never

- Read or interpret payloads

## Depends on

Nothing. Link has no outgoing edges in the architecture diagram.

## Module checklist

- [ ] A path loss returns an `RLY-LK-<N>` code, and the transfer moves to another path.
- [ ] Failover is tested in the `sim` harness with a link drop.
- [ ] The P3 exit test holds: the transfer continues within 3 seconds and the hash matches.

## Memory

### Notes

None yet.
