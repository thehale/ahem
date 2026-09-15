# Spike: evidence, not product

This directory is throwaway. It exists to make
[the distribution proposal](../docs/single-file-executable.md) checkable, and
it should be deleted once the proposal is accepted or rejected.

It answers the two questions that decide the Rust option:

1. Can a Rust binary parse Go templates with **no `cc` at install or run time**?
2. Can it reuse **ast-grep's own rule semantics** rather than reimplementing
   them?

## Run it

```bash
mise install rust@latest
cd spike
mise x rust@latest -- cargo run --release
```

Expected output, seven cases, all `ok`:

```text
ok   comment-earns-nothing.gotmpl true  "{{- /* a thing */ -}}"
ok   comment-earns-nothing.gotmpl false "<p>{{ .Title }}</p>"
ok   comment-earns-nothing.Bash true  "# a thing"
ok   comment-earns-nothing.Bash false "#!/usr/bin/env bash"
ok   comment-earns-nothing.Bash false "# SPDX-License-Identifier: MPL-2.0"
ok   comment-earns-nothing.Rust true  "// a thing"
ok   comment-earns-nothing.Rust false "fn main() {}"
```

Those are `rules/comment.yml`'s own `valid`/`invalid` snippets, run against
rule bodies copied verbatim out of the compiled cache shape that `src/lib/compile`
produces today.

## What each answer rests on

**Grammar, statically linked.** `vendor/gotmpl/` holds `parser.c` from
`ngalaiko/tree-sitter-go-template` at `aa71f63`, stripped to the four files a
compile needs. `build.rs` runs `cc` at *build* time and links the object into
the binary; `src/main.rs` binds the `tree_sitter_gotmpl` symbol directly. The
released binary has no `libraryPath`, does no `dlopen`, downloads nothing, and
links only `libc` and `libgcc_s`.

**Rule semantics, reused.** `from_yaml_string::<Lang>` is ast-grep's own
deserializer from the `ast-grep-config` crate. Nothing here re-implements
`all`/`any`/`not`/`kind`/`regex`, and nothing would need to re-implement
`pattern`, `inside`, `has`, `matches` or `constraints` either. `Lang` is a
20-line enum over `SupportLang` plus `GoTmpl`; because it is generic over
`L: Language`, ast-grep treats the custom grammar as a first-class language,
not as a plugin.

## Measurements

Taken on this machine, `rustc 1.98.1`, `--release` with `strip = true`,
x86_64-unknown-linux-gnu.

| Build | Grammars | Binary | Clean build |
| --- | --- | --- | --- |
| `--features all-languages` (default) | 27 builtin + gotmpl | 43 MB | 15 s |
| `--no-default-features --features rule-languages` | 6 + gotmpl | 7.3 MB | 3 s |

`rule-languages` is the set `rules/comment.yml` actually targets: Bash, Css,
Go, JavaScript, Python, Rust. Both builds pass all seven cases.

For comparison, today's four mise-resolved dependencies are 84 MB on this
machine: ast-grep 50 MB, scc 18 MB, yq 14 MB, jq 2.2 MB.

## Known holes

Deliberate, because they do not change the decision:

- No file walking, no CLI, no language detection, no `ast-grep test` runner.
  The proposal costs those out; the spike does not build them.
- Rules are inlined in `main.rs` as post-compilation YAML. The spike does not
  reproduce `src/lib/compile`'s per-language expansion.
- Only linux-x64 was built. Cross-compilation is argued in the proposal from
  ast-grep's own release workflow, not measured here.
- `--no-default-features` makes an unselected language `unimplemented!()` at
  runtime rather than a compile error. A real build would gate that in the
  `Lang` enum.
