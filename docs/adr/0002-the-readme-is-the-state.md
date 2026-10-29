# 2. The README is the only state

Date: 2026-10-28. Status: accepted.

## Context

readme-style must be idempotent, must let users switch themes, and must never confuse its own
output with what the user wrote. That needs to know, on every run, which lines it generated.

## Decision

Everything generated is delimited by HTML comments in the README: regions
(`<!-- readme-style:NAME theme=THEME -->` ... `<!-- /readme-style:NAME -->`) for generated content,
and wrapper lines (a generated line ending in a marker) for layout around user content. There is
no cache, lock file or side file.

Regions are always placed where the theme puts them; wrapper lines are removed first and rebuilt
last. The theme is recorded in the markers, so later runs and `--check` reuse it.

## Consequences

- The output is a function of the user's content, the project and the theme: idempotency and theme
  switching follow from it, and both are tested on every run and on a corpus.
- Edits inside a region are overwritten. The README says so, and the markers are visible in source.
- A blank line added to space out a region cannot be told apart from the user's own once the region
  is gone, so theme switches are equal up to blank lines between blocks, which do not change the
  rendering.
