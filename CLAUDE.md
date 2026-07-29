# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Working on a referenced PR or branch

If the user asks you to work on, fix, or diagnose a specific PR or branch
they've named (e.g. "look at PR #1" or "fix the CI on branch X"), make your
changes directly on that PR's existing branch and push there. Do not create
a new branch or open a separate PR for the fix unless the user asks for
that explicitly. The goal is for the fix to land in the PR/branch the user
actually referenced, not in a new one.

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
