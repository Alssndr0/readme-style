# 3. Verify every result with an independent guard

Date: 2026-10-28. Status: accepted.

## Context

"Keep my words" is the product. A transform bug that drops a flag from an install command would be
worse than no tool at all, and no amount of transform tests covers every README.

## Decision

Every result is compared with its input by a guard that shares no logic with the transforms:
strip what readme-style manages, render both as GitHub would, and compare words, exact code, link
targets, images, heading levels and block boundaries. Then run the pipeline again on the output and
require no change. Both checks run in production, not only in tests; a failure leaves the file
untouched and exits with code 3.

## Consequences

- Bugs show up as refusals, never as damage. Property tests and the corpus treat a refusal of
  well-formed input as a failure, which is how most transform edge cases were found.
- The guard defines what "the same content" means. Two deliberate allowances: an alert's title
  stands in for the "Note:" lead-in it replaced, and emoji are not words.
- Styling costs a second pipeline run and two extra renders: still a few milliseconds.
