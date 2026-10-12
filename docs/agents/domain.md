# Domain Docs

This repo uses a single-context layout:

- `GLOSSARY.md` at the repo root.
- Architecture decision records under `docs/adr/`.

## Before exploring

Read `GLOSSARY.md` and ADRs relevant to the area being explored.

If these files do not exist, proceed silently. Domain-modeling creates them
lazily as terminology and decisions are resolved.

If the repo later introduces `GLOSSARY-MAP.md`, follow its pointers to the
glossaries relevant to the topic, and read relevant context-specific ADRs.

## Vocabulary

Use glossary terms in issue titles, proposals, hypotheses, and test names.
If a needed concept is missing, reconsider whether it belongs to the domain;
record real vocabulary gaps for domain-modeling.

## ADR conflicts

Explicitly flag proposals that contradict an existing ADR. Name the ADR and
explain why its decision should be reopened.
