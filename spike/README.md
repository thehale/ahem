# Spike: evidence, not product

This directory is throwaway. It exists to make
[the distribution proposal](../docs/single-file-executable.md) checkable, and
it should be deleted once the proposal is accepted or rejected.

It answers the four questions that decide the Rust option:

1. Can a Rust binary parse Go templates with **no `cc` at install or run time**?
2. Can it reuse **ast-grep's rule semantics** rather than reimplementing them?
3. When a rule is compiled to **every** language, do the languages it does not
   fit **fail loudly**?
4. Does a rule that does not apply to a language need a **waiver marker**?
5. Can a project **add its own rules** to a binary that ships with rules
   built in?

Questions 3 and 4 are answered against `main` at 4059242, which split tests
into `rule-tests/` and grew the rule set to four.

## Run it

```bash
mise install rust@latest
cd spike
mise x rust@latest -- cargo run --release
```

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
small enum over `SupportLang` plus `GoTmpl`; because ast-grep is generic over
`L: Language`, the custom grammar is a first-class language, not a plugin.

The seven matched snippets are `rules/comment.yml`'s own `valid`/`invalid`
cases, run against rule bodies in the shape `src/lib/compile` produces today.

**Gaps fail loudly, and cost more the more structure a rule names.**
`coverage()` compiles all four of `main`'s rule bodies against all 29
languages:

```text
 4 of 29 languages need an override: comment-earns-nothing
 7 of 29 languages need an override: name-carries-no-content
18 of 29 languages need an override: conditions-compose-into-a-value
26 of 29 languages need an override: branches-read-as-one-shape
```

`RuleConfig::try_from` returns `MissingPotentialKinds` when a rule's `kind`
resolves to no node in that grammar, and a `pattern` that does not parse is
rejected outright. `branches-read-as-one-shape` names four node kinds at once
and only JavaScript, TypeScript and Tsx have all four under those names. All
four bodies are copied from `main` at 4059242.

**Loud failure has one hole.** `false_positives()` shows Markdown and Yaml
accepting the pattern rule and then matching nothing in real documents. The
rule is inert rather than wrong, but nothing complains, so the checklist calls
those languages covered when the rule is meaningless there.

**A waiver marker is not settled.** `waiver()` compiles four candidate no-ops
against Json. A rule that does not apply often still compiles and matches
nothing, which is why Json needs no waiver for the comment rule: it has a
`comment` node. A rule that belongs to one language family, like the new one,
is the case that may need one. ast-grep offers no conventional no-op: `any: []`
compiles and matches nothing as an emergent property of an empty kind union,
while `all: []` and `not: {any: []}` are both rejected.

**User rules are not second-class.** `user_rules()` writes a rule to a file,
reads it back at run time, and loads it beside the embedded set through the
same `from_yaml_string::<Lang>` call. It matches, and the merged set is
checked for id collisions. Nothing about a rule loaded from disk is weaker
than one compiled in, so there is no plugin API to design. Adding a
*language* is the thing a user cannot do, because grammars link at build time.

## Measurements

Taken on this machine, `rustc 1.98.1`, `--release` with `strip = true`,
x86_64-unknown-linux-gnu.

| Build | Grammars | Binary | gzip -9 | Clean |
| --- | --- | --- | --- | --- |
| `all-languages` (default) | 27 builtin + gotmpl | 43 MB | 4.9 MB | 15 s |
| `rule-languages` | 6 + gotmpl | 7.3 MB | 1.8 MB | 3 s |

Select the second with
`cargo build --release --no-default-features --features rule-languages`.
Parser tables compress well, so the download is a fraction of the file on
disk. Compiling to every language argues for the first build regardless.

For comparison, today's four mise-resolved dependencies are 83 MB on this
machine, 21 MB as a `tar czf` bundle: ast-grep 50 MB, scc 18 MB, yq 14 MB,
jq 2.2 MB.

## Resolved: the abort belongs to tree-sitter-haskell

An earlier draft recorded an unexplained abort and blamed the spike's use of
`ast-grep-core`, and ruled out the vendored grammar on a test that turned out
to have reused a cached build artifact. Both conclusions were wrong.

`src/bin/haskell-abort.rs` is the reproducer. It compiles a `kind: comment`
rule for every language, matches a handful of snippets, and takes a
`PERTURB` count of live heap blocks to allocate first:

```bash
cargo run --release --bin haskell-abort              # aborts
SKIP=Haskell cargo run --release --bin haskell-abort # clean
```

Swept over 21 heap layouts:

```text
all languages:  20/21 heap layouts aborted
SKIP=Haskell:    0/21 heap layouts aborted
```

`tree-sitter-haskell` 0.23.1, the version `ast-grep-language` 0.45.3 pulls and
the newest published, overflows the heap while parsing. The damage is latent
and surfaces later as `corrupted size vs. prev_size` in whatever allocates
next, which is why it first looked like a `gotmpl` or an `ast-grep` problem.
The narrowest input that triggers it is a Haskell source consisting only of a
comment, though whether a given build aborts depends on heap layout, so a
standalone one-grammar reproducer is not reliable.

Nothing here is deconfuse's to fix and there is no fixed release to upgrade
to. deconfuse ships every language except Haskell until upstream has one.

## Known holes

Deliberate, because they do not change the decision:

- No file walking, no CLI, no language detection, no test runner. The proposal
  costs those out; the spike does not build them.
- Rules are inlined in `main.rs` as post-compilation YAML. The spike does not
  reproduce `src/lib/compile`'s per-language expansion.
- Only linux-x64 was built. Cross-compilation is argued in the proposal from
  ast-grep's own release workflow, not measured here.
- `--no-default-features` makes an unselected language `unimplemented!()` at
  runtime rather than a compile error. A real build would gate that in `Lang`.
