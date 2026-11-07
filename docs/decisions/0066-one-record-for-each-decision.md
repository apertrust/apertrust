# 0066. One record for each decision

## Context

Architecture decisions must be easy to find, read and keep current, by people and by the agents that help write the code. One long table of rows is hard to read; history mixed into a record makes it hard to tell what is decided now.

## Decision

**One file per decision in `docs/decisions/`,** named `NNNN-slug.md`: the next free four-digit number, then the title in lower case with apostrophes and punctuation dropped and spaces as hyphens.

Each record has four sections:

- **Context:** what question it settles and what constrains it, in a few short sentences;
- **Decision:** what is decided, as it stands now, with the names, types, numbers and limits that matter; a part not built yet says "planned";
- **Options considered:** each real alternative in a line, with why it was not chosen;
- **Consequences:** what follows for the code and the docs.

Each record has a line in `docs/decisions/README.md`, its number linked to its file, with its title. Records cite each other by number and link.

**Records describe what is decided now.** A decision that changes another edits that record to the current state; history is not kept in the records, since git keeps it. A new question gets a new record; a change to an existing decision is an edit of its record, with every other place that describes it brought into agreement.

**What else agrees with the records:** the current state by area in [the index](README.md#the-current-state-by-area), the sections of the design docs that describe the area, the open questions, AGENTS.md's rules and crate table, [crates.md](../crates.md), and the project's skills. The `decision-log` skill in the agent kit walks through them; the `architect` agent records decisions with it.

## Options considered

- One table of long rows in one file: hard to read and to link.
- Records that are never edited, each later one amending or superseding the earlier: the current state must then be pieced together from a chain.
- Decisions recorded somewhere outside the repository: a contributor could not find them beside the code.

## Consequences

- A change that settles or changes a decision lands with its record in the same pull request.
- Code comments and docs cite a decision by its number, which never changes.
