# Architecture Specifications (LeanSpec)

> Specifications precede implementation. If a system's invariants cannot be
> articulated in plain technical English, compiling it is merely executing an
> unvetted guess.

This directory is the LeanSpec registry of record. Every architectural decision,
boundary rule, and verification gate is specified as an individual, versioned
contract. Methodology: LeanSpec (`vendor/leanspec/`).


| Spec | Title | Status | Priority |
|---|---|---|---|
| [SPEC-0001](SPEC-0001-process-supervision.md) | Process supervision | complete | critical |
| [0001](0001-template-architecture.md) | Template architecture | in-progress | high |
| [0002](0002-verification-gates-and-evidence.md) | Verification gates and evidence | in-progress | high |
| [0003](0003-branch-model-and-promotion.md) | Branch model and promotion | in-progress | high |
| [0004](0004-dependency-and-licence-policy.md) | Dependency and licence policy | in-progress | high |
| [0005](0005-ci-cache-and-runners.md) | CI, cache and runners | in-progress | high |

## How to add a spec

1. Take the next free number. Use four digits. Example: `0006`.
2. Copy the template from `../vendor/leanspec/spec-template.md`.
3. Name the file `NNNN-kebab-slug.md`. Make the H1 read `# NNNN-kebab-slug`.
4. Keep the sections in this order: Overview, Design, Plan, Test, Notes.
5. Put one runnable command in a fenced block under Test.
6. Keep the file under 300 lines. Split a long spec into sub-spec files.
   See `../vendor/leanspec/sub-spec-files.md`.
7. Add one row to the table above, with a relative link.
8. Run `make spec-check`. It must pass.
