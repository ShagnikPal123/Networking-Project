# Relay: the agent system

Status: **draft for approval** · 2026-11-17

One system works on the whole repo. It behaves differently per module because it loads that
module's playbook, checklists, and memory before touching it.

```mermaid
flowchart TB
    classDef lead fill:#FF6B35,stroke:#1A1A2E,color:#1A1A2E,font-weight:bold
    classDef strong fill:#118AB2,stroke:#1A1A2E,color:#FFFFFF,font-weight:bold
    classDef mid fill:#FFD166,stroke:#1A1A2E,color:#1A1A2E
    classDef low fill:#06D6A0,stroke:#1A1A2E,color:#1A1A2E
    classDef mem fill:#073B4C,stroke:#FFD166,color:#FFFFFF
    L[Lead · strongest model<br/>coordination, hard design, final review]:::lead
    I[Idea agent<br/>what next, new options]:::strong
    S[Security reviewer<br/>threats, keys, access checks]:::strong
    D[Design reviewer<br/>reads designs as a builder would]:::strong
    B[Builder<br/>implements one module task]:::mid
    T[Test agent<br/>sim scenarios, failure tests]:::mid
    W[Docs agent<br/>playbooks, README, changelog]:::low
    H[Helper<br/>boilerplate, searches]:::low
    M[(Memory<br/>one file per agent + shared decisions)]:::mem
    L --> I & S & D
    L --> B & T
    B --> H
    T --> H
    L --> W
    I & S & D & B & T & W <--> M
```

Colors: orange = lead · blue = strong model · yellow = mid model · green = cheapest model.

## Who does what

| Agent | Tier | Use for | Never for |
|-------|------|---------|-----------|
| Lead | Strongest | Coordination; anything touching identity, session, membership, failover; design changes; final review | Renames, formatting |
| Idea agent | Strong | Keeping `NEXT-UPDATES.md` and `05-vision.md` current; researching new options | Editing code or approved design docs |
| Security reviewer | Strong | Reviewing any change that trips the advanced checklist; keeping the threat table current | Writing the feature it reviews |
| Design reviewer | Strong | Reading a design or plan cold and listing what a builder would still have to guess | Rewriting the design itself |
| Builder | Mid | A scoped task inside one module with a written plan | Changing a module's public interface on its own |
| Test agent | Mid | `sim` scenarios, link-drop and host-loss tests, a failing test for every new error code | Changing product code to make a test pass |
| Docs agent | Cheapest | Module playbooks, README, changelog, keeping docs in step with merged code | Deciding design questions |
| Helper | Cheapest | Boilerplate, test scaffolds, finding files | Security or protocol logic |

A task always goes to the cheapest agent that can do it safely. Work flows down; results are
checked on the way back up. Reviewers never review their own work.

## Splitting work

Any agent may ask for sub-agents, but it must **propose the split first** and wait for a yes:

1. List the pieces, which tier each needs, and which files each piece owns.
2. No two pieces may own the same file. Shared files (workspace manifest, `proto` types) are
   written by the requester before the split starts.
3. The requester checks every result against the checklists before merging it.

Lead proposes splits to the owner. Every other agent proposes splits to the lead.

## The idea agent

Its own module of the agent system, with its own playbook, checklists, and memory.

- **Job:** know what should change next, track the newest relevant technology, and keep adding
  ideas tied to a real impact area.
- **Writes to:** `NEXT-UPDATES.md` (Ideas, Tech watch, Changelog) and proposals for `05-vision.md`.
- **Cannot:** move an idea to Planned (owner only), change P0 to P3 scope, or touch code.
- **Runs:** at the end of each phase, and whenever the owner asks.

Idea agent basic checklist:

1. Read `NEXT-UPDATES.md`, `05-vision.md`, and `memory/ideas.md` (past ideas, including rejected
   ones and why).
2. Update **Now** and **Next** to match what actually merged.
3. For each new idea: name the impact area, the user it helps, and a rough size. No area, no idea.
4. Do not re-propose a rejected idea unless something changed; say what changed.

Idea agent advanced checklist (when proposing a technology or a design change):

1. Check the claim against a primary source dated within the last year and link it.
2. State what it would replace in the current design and what it would cost to switch.
3. State whether it works on both Windows and Android and offline.
4. Mark it "verified" in Tech watch only after steps 1 to 3 are written down.

## Specialist checklists

Each specialist runs the shared basic checklist plus its own.

**Security reviewer** · memory: `memory/security.md` (past findings, accepted risks)

- Basic: which threat-table row does this change touch? Is there a test that fails if the
  protection is removed? Are secrets kept out of logs and error text?
- Advanced: try to break it as an unapproved device, as a removed device, and as a member reaching
  outside a grant. Check every input from the network is size-limited and validated before use.
  Update the threat table in `02-protocol.md` if a new threat appears.

**Design reviewer** · memory: `memory/reviews.md` (questions that came up before)

- Basic: could I plan this without asking anyone? List only gaps where a wrong guess matters.
- Advanced: does it contradict an approved point, another design doc, or a logged decision? Does
  any module now need to call upward?

**Test agent** · memory: `memory/tests.md` (flaky tests, timing lessons)

- Basic: every new behavior has a test; every new `RLY-` code has a test that triggers it; tests
  use the fake clock, never real sleeps.
- Advanced: add a `sim` scenario for each failure the change could meet (link drop, slow link,
  peer restart, host loss). Run it 20 times to catch timing-dependent failures.

**Docs agent** · memory: `memory/docs.md` (style rules, terms to use)

- Basic: the module playbook matches the merged interface; `NEXT-UPDATES.md` changelog has a line;
  links work.
- Advanced: when a wire message or interface changed, update the diagrams in `docs/design/` and ask
  the lead to confirm.

## Files

```
CLAUDE.md                     entry point: the rules below + router table (module → playbook)
docs/agent/
  checklists/basic.md         run for every task
  checklists/advanced.md      run for risky tasks (see trigger list)
  modules/<module>.md         per-module: purpose, interface, rules, its own checklist items
  memory/decisions.md         dated decisions and why
  memory/gotchas.md           things that went wrong once and how to avoid them
  memory/ideas.md             idea agent: ideas proposed, accepted, rejected, and why
  memory/security.md          security reviewer: findings and accepted risks
  memory/reviews.md           design reviewer: questions raised before
  memory/tests.md             test agent: flaky tests, timing lessons
  memory/docs.md              docs agent: style rules and terms
  agents/<agent>.md           one playbook per specialist with its two checklists
```

## Basic checklist (every task)

1. Name the one module this task belongs to. If it is two, split the task.
2. Read that module's playbook and the memory entries tagged with it.
3. Write or update the failing test first.
4. Make the change without breaking the downward-only dependency rule.
5. Every new failure has an `RLY-<MODULE>-<N>` code.
6. Build, lint, and run the module's tests. Paste the real output.
7. Add anything surprising to `memory/gotchas.md`.

## Advanced checklist (also run when a trigger applies)

Triggers: touches keys, certificates, or access checks · changes a wire message · changes a public
module interface · touches failover or handoff · touches anything written to disk.

1. State the threat or failure this could introduce, and the test that proves it does not.
2. Wire change: version bumped, old peers still handled, `proto` tests updated.
3. Interface change: every caller updated, playbook updated, decision logged.
4. Run the full `sim` suite, including link-drop and host-loss scenarios.
5. Disk change: old data still loads, or a migration exists and is tested.
6. Lead-tier review before merge.

## Memory rules

- Save decisions with a date and a reason. Save gotchas with the symptom and the fix.
- Do not save what the code or git history already says.
- Read before work, write after work. Wrong entries get deleted, not appended to.
