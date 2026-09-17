---
name: add-a-check
description: >-
  Add a check to bin/ci. Use when a linter, formatter, test runner, or
  scanner should run as part of this repository's CI gate.
license: MPL-2.0
---

# Add a check

`bin/ci` is the gate. A check belongs here when it should block a change, not
when it is something a person runs by hand.

## Steps

1. Add the tool to `mise.toml`.
2. Add a `check` line to each list in `bin/ci`: the gating command under the
   `else` branch, and the fixing command under `--fix` if the tool can fix
   what it finds. Name it `"Category: Action"`, matching the ones already
   there.
3. Run `bin/ci` and read the summary.

A one-line command can sit inline in `bin/ci`. Anything that needs a config
file, a file list, or more than one pipeline gets its own script under `bin/`.

## A check with its own script

Name the script for the job, not for the tool it calls. A script that shares
its tool's name can take that name on PATH and call itself instead, and the
symptom is a hang rather than an error.

Follow `bin/markdownlint` for a check that carries configuration. It writes
the config to a temporary directory, passes it to the tool, and removes it on
exit, so no dotfile lands in the repository.

Take the file list from `filelike`, sourced from `bin/lib/filelike`, rather
than globbing. It reads what git tracks, drops deleted files, and skips
`vendor/`.

Accept `--fix` when the tool can fix what it reports.

## Tests

A script with logic of its own gets a bats suite at `bin/.tests/<script>.bats`.
A wrapper that writes a config and hands off to its tool does not, since the
suite would be testing the tool. The suites run from `BATS_TEST_TMPDIR`, so a
test builds whatever files or repository it needs and leaves this one alone.
`bin/ci` runs them as the `Bash: Test` check.
