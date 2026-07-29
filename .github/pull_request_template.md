<!--
Fill in every section. Guidance comments stay in place — they don't render, and
they make an unfilled section obvious. Replace the prompts with real content:
never leave placeholder text, and never claim something you did not do.
"None" and "N/A" are acceptable answers where a section allows them, and are
always better than a guess dressed up as a fact.
-->

## Summary

<!-- One imperative sentence: what this PR does. Then two or three sentences on
why it is needed — the problem being solved, not a restatement of the title.
Delete the line below if this PR does not close an issue. -->

Fixes #

## Scope

<!-- What a reviewer might reasonably expect to find here but won't, and why it
was left out. Adjacent cleanups not done, cases not handled, follow-on work
deferred. -->

**In scope:**

**Out of scope:**

## Design decisions

<!-- Every choice a reviewer could reasonably have made differently: what was
chosen, what was rejected, and why. This is the highest-value section for
reviewing code you did not write — do not skip a decision because it seemed
obvious while making it. -->

## Assumptions

<!-- Where the request was ambiguous and you resolved it by guessing: state the
guess and what breaks if it is wrong. Write "None" if nothing was ambiguous. -->

## Verification

<!-- What was actually run, and what "correct" means for this change.
These boxes are self-reported; see #2 for replacing them with CI-checked
status once the crate and a build workflow exist. -->

- [ ] Builds cleanly (`cargo build`)
- [ ] `cargo fmt` applied, `cargo clippy` clean
- [ ] Tests added or updated for the behaviour this PR changes
- [ ] Test suite run locally (`cargo test`)
- [ ] Public API changes carry doc comments
- [ ] No commented-out code, dead code, or leftover debug output

<!-- Leave a box unchecked if it does not apply or was not done, and say why in
one line underneath. An unchecked box with a reason is fine. A checked box that
is not true is not. -->

**How a reviewer can see it work:**

## Risk

<!-- What could this break, and how would the breakage show up? Call it out
explicitly if the change touches: public API, `unsafe`, per-frame or hot-path
code, allocation behaviour, concurrency, serialization or asset formats, or
dependencies. "Low — <reason>" is a fine answer for contained changes. -->

## Where to focus review

<!-- Order the diff for the reviewer: what to read first, what is mechanical and
can be skimmed, and which part you are least confident in and why. Naming your
weakest spot is expected — it is the most useful sentence in the PR. -->

## Follow-ups

<!-- Known gaps left deliberately, linked to an issue where one exists.
"None" if the change is complete as it stands. -->

## Provenance

<!-- The human named here is accountable for what merges, regardless of who or
what typed the diff. -->

- Authored by: <!-- agent / tool, e.g. Claude Code -->
- Accountable human:
- [ ] A human has read the full diff and can explain what it does.
