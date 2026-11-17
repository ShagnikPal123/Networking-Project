# Advanced checklist (risky tasks)

Run this when a trigger below applies, in addition to the basic checklist.

Triggers:

- [ ] Touches keys, certificates, or access checks
- [ ] Changes a wire message
- [ ] Changes a public module interface
- [ ] Touches failover or handoff
- [ ] Touches anything written to disk

Steps:

- [ ] 1. State the threat or failure this could introduce, and the test that proves it does not.
- [ ] 2. Wire change: version bumped, old peers still handled, `proto` tests updated.
- [ ] 3. Interface change: every caller updated, playbook updated, decision logged.
- [ ] 4. Run the full `sim` suite, including link-drop and host-loss scenarios.
- [ ] 5. Disk change: old data still loads, or a migration exists and is tested.
- [ ] 6. Lead-tier review before merge.
