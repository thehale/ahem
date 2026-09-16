<div align="center">

# deconfuse

Finds code patterns worth reconsidering, and says which principle each one
breaks.

<!-- BADGES -->
[![License: MPL-2.0](https://badgen.net/github/license/thehale/template)](https://github.com/thehale/template/blob/main/LICENSE)
[![Sponsor thehale on GitHub](https://badgen.net/badge/icon/Sponsor/pink?icon=github&label)](https://github.com/sponsors/thehale)
[![Joseph Hale's software engineering blog](https://jhale.dev/badges/website.svg)](https://jhale.dev)
[![Follow Joseph Hale on LinkedIn](https://jhale.dev/badges/follow.svg)](https://www.linkedin.com/comm/mynetwork/discovery-see-all?usecase=PEOPLE_FOLLOWS&followMember=thehale)
</div>

## Quickstart

Point `check` at the files you just touched. Each finding names the line, the
principle it breaks, and the rule that found it.

```console
$ deconfuse check src/session.js
src/session.js:2: Default to no comments. A well named helper is the comment, and rationale belongs in the commit message where it cannot drift from the code. [comment-earns-nothing]
src/session.js:3: A condition already is the value these branches return. Returning it directly says the same thing on one line. [condition-is-already-the-value]
```

A path can be a file or a directory, and several can be given at once. The exit
status is 1 while anything is reported, so a review step can gate on it.

```bash
deconfuse check src lib   # report findings under either path
deconfuse rules           # list rules and the languages they reach
deconfuse test            # run every rule against its snippets
```

## Installation

```bash
cargo build --release
install -m 755 target/release/deconfuse ~/.local/bin/
```

One binary, nothing installed alongside it. Rules and their tests are compiled
in, every tree-sitter grammar is linked at build time, and nothing is written
to disk at run time.

## Contributing

```bash
bin/setup     # Install the tools
bin/ci        # Run the checks
bin/ci --fix  # Fix what can be fixed automatically
```

Rules live in `rules/`, one file each, paired with snippets in `rule-tests/`.
A rule reaches a language only once that language has snippets proving it
fires and stays quiet, so adding a language means adding its snippets.

## License

Copyright (c) 2026 Joseph Hale, All Rights Reserved

Provided under the terms of the [Mozilla Public License, version 2.0](./LICENSE)

<details>

<summary><b>What does the MPL-2.0 license allow/require?</b></summary>

### TL;DR

You can use files from this project in both open source and proprietary
applications, provided you include the above attribution. However, if
you modify any code in this project, or copy blocks of it into your own
code, you must publicly share the resulting files (note, not your whole
program) under the MPL-2.0. The best way to do this is via a Pull
Request back into this project.

If you have any other questions, you may also find Mozilla's [official
FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) for the MPL-2.0 license
insightful.

If you dislike this license, you can contact me about negotiating a paid
contract with different terms.

**Disclaimer:** This TL;DR is just a summary. All legal questions
regarding usage of this project must be handled according to the
official terms specified in the `LICENSE` file.

### Why the MPL-2.0 license?

I believe that an open-source software license should ensure that code
can be used everywhere.

Strict copyleft licenses, like the GPL family of licenses, fail to
fulfill that vision because they only permit code to be used in other
GPL-licensed projects. Permissive licenses, like the MIT and Apache
licenses, allow code to be used everywhere but fail to prevent
proprietary or GPL-licensed projects from limiting access to any
improvements they make.

In contrast, the MPL-2.0 license allows code to be used in any software
project, while ensuring that any improvements remain available for
everyone.

</details>
