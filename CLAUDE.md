# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Working on a referenced PR or branch

If the user asks you to work on, fix, or diagnose a specific PR or branch
they've named (e.g. "look at PR #1" or "fix the CI on branch X"), make your
changes directly on that PR's existing branch and push there. Do not create
a new branch or open a separate PR for the fix unless the user asks for
that explicitly. The goal is for the fix to land in the PR/branch the user
actually referenced, not in a new one.

## Working from an issue

Issues are the primary way changes to this repository get started, and they
are filled out from the forms in `.github/ISSUE_TEMPLATE/`: `[BUG]` for
behaviour that is wrong, `[REQ]` for behaviour that is missing, `[TSK]` for
work on the repository itself. The issue body is the specification. Read
every heading in it before touching code.

**Done when** is the acceptance contract. Satisfy it literally. If it cannot
be satisfied as written — it contradicts the code, or it turns out to describe
a different problem — say so in the pull request or the issue thread rather
than quietly substituting a criterion you can meet.

**Notes for the implementer** is direction, not decoration. Where it conflicts
with what you would have done unprompted, follow it. If you deviate anyway,
the deviation and its reason belong under Design decisions in the pull
request.

A skipped optional field renders as `_No response_` in the issue body. That
means the filer did not specify, not that there are no constraints. Make the
call yourself rather than stalling, then record it under Assumptions in the
pull request — that section exists precisely to catch the guesses that an issue
left open.

On a `[REQ]` marked **Exploratory — discuss before implementing**, do not open
a pull request. Comment on the issue with a proposed approach and wait for a
reply.

If the issue contradicts the code — a bug report describing behaviour the
source cannot produce, a request for something that already exists — the code
wins. Say so in the issue thread before implementing anything.

Close the loop when you open the pull request: `Fixes #N` under Summary.

## Opening a pull request

Use `.github/pull_request_template.md` for every PR. GitHub does not apply
the template to pull requests created through the API, so paste it in and
fill it out yourself rather than assuming it appears.

Complete every section. Explain intent and the choices you made — a reviewer
can read the diff, so restating it wastes the one place where your reasoning
can be recorded. Never tick a checkbox for something you did not do; leave it
unchecked with a one-line reason instead. Name yourself as the agent under
Provenance and leave the accountable human blank for the repository owner to
fill.
