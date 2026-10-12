# Issue tracker: GitHub

Issues and specs live in GitHub Issues for `zantoku/OhMyLoop`.
Use the `gh` CLI with `--repo zantoku/OhMyLoop` for issue operations.

## Conventions

- Create: `gh issue create --repo zantoku/OhMyLoop --title "..." --body-file <file>`.
- Read: `gh issue view <number> --repo zantoku/OhMyLoop --json number,title,body,labels,comments`.
- List: `gh issue list --repo zantoku/OhMyLoop --state open --json number,title,body,labels`.
  Apply label and state filters as needed.
- Comment: `gh issue comment <number> --repo zantoku/OhMyLoop --body-file <file>`.
- Apply or remove labels: `gh issue edit <number> --repo zantoku/OhMyLoop --add-label "..."`
  or `--remove-label "..."`.
- Close: `gh issue close <number> --repo zantoku/OhMyLoop --comment "..."`.

Use a temporary UTF-8 file for multiline bodies, preserving actual newlines.

When a skill says "publish to the issue tracker", create a GitHub issue.
When it says "fetch the relevant ticket", read the referenced issue.

## Parent issues and dependencies

Use native GitHub sub-issues where available. Otherwise, put `Part of #<parent>`
at the top of each child body and add the child to the parent's task list.

Use native GitHub issue dependencies where available. API dependency operations
require the issue's numeric database ID, not its issue number or node ID.
Otherwise, put `Blocked by: #<number>` at the top of the child body.
A ticket is unblocked when every blocker is closed.

## Pull requests as a triage surface

**PRs as a request surface: no.**

GitHub shares issue and PR numbers. When a reference is ambiguous, resolve it
with `gh pr view` and fall back to `gh issue view`.

## Wayfinding operations

- Map: one issue labelled `wayfinder:map`, with Notes, Decisions-so-far, and Fog.
- Child ticket: linked to the map as a sub-issue or through the fallback above;
  labelled `wayfinder:<type>` (`research`, `prototype`, `grilling`, or `task`).
- Frontier: open map children with no open blockers and no assignee;
  first in map order wins.
- Claim: assign the ticket to `@me`.
- Resolve: comment with the answer, close the ticket, and append a context
  pointer containing a gist and link to the map's Decisions-so-far.
