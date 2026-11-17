# Design reviewer: working memory

Kept under `docs/reviews/` because this reviewer writes only there. The lead can fold it into
`docs/agent/memory/reviews.md`. Wrong entries get deleted, not appended to.

## Reviews done

| Date | Commit | File | Result |
|------|--------|------|--------|
| 2026-11-17 | `ae31b3e` | `p0-readiness-review.md` | 5 blockers (B1 to B5), 7 security, 9 failure-mode findings. P0 plan not yet pushed. |

## Questions already raised (do not raise twice)

- B1 run mode and local control channel · B2 sans-IO session and sim broadcast · B3 what is parsed
  before trust · B4 40-bit fingerprint pin · B5 device key at rest and `relay init` twice.
- Beacon signature cannot be verified (S2). Frame version unchecked (S4).
- Windows reserved file names in store keys (F2). Health strip green by default (F6).
- Later phases: offer name path escape (P1), no fetch message for browsing (P2), clocks offline
  (P2), single Wi-Fi radio vs "third path" (P3), same-epoch deputies and hotspot host loss (P3).

## Already fixed by the lead (checked at `ae31b3e`, do not re-raise)

Commit-then-reveal join code · deputy certificates bound to host and epoch · member certificates
survive a host change · switch at Suspect with warm standby · per-chunk hash list and flush before
mark · signed audit checkpoints · beacon no longer carries host or epoch · `MB --> AU` edge.

## Lessons

- Fetch the design branch again before writing. It moved while the first cold read was in progress.
- Check a Windows claim against Microsoft's docs before stating it. One claim (limited broadcast
  leaves on one interface only) was contradicted by the `sendto` page and was dropped.
- The owner wants short tables, diagrams, and suggested answers, not open questions alone.
