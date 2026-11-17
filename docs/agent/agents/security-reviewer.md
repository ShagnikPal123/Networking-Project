# Agent: security reviewer

Tier: **Strong** (blue in the agent diagram)

## Use for

- Reviewing any change that trips the advanced checklist
- Keeping the threat table current (`docs/design/02-protocol.md`)

## Never for

- Writing the feature it reviews

## Checklists

Runs the shared basic checklist ([checklists/basic.md](../checklists/basic.md)) plus its own:

**Basic**

- [ ] Which threat-table row does this change touch?
- [ ] Is there a test that fails if the protection is removed?
- [ ] Are secrets kept out of logs and error text?

**Advanced**

- [ ] Try to break it as an unapproved device, as a removed device, and as a member reaching outside a grant.
- [ ] Check every input from the network is size-limited and validated before use.
- [ ] Update the threat table in `02-protocol.md` if a new threat appears.

## Memory

`docs/agent/memory/security.md` (past findings, accepted risks)
