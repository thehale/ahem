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

## Open risk: an abort when matching Haskell

Matching, rather than compiling, a rule against Haskell source aborts the
process with `corrupted size vs. prev_size`. It reproduces every run.

What is known:

- Compiling rules for all 29 languages is clean. Only matching aborts.
- It is not the vendored grammar. The abort happens with the `gotmpl` object
  unlinked.
- It is not the grammar alone. `tree_sitter::Parser` with the same
  `tree-sitter-haskell` 0.23.1 parses all the same snippets without aborting,
  including with one parser reused across parses.
- It is not ast-grep. `ast-grep run --lang haskell` on the same snippets is
  clean, and `src/deconfuse check` over a `.hs` file exits 0.

So it sits somewhere in this spike's use of `ast-grep-core` as a library, most
likely in how `Lang::get_ts_language` hands out a `TSLanguage` per call. Not
root-caused. It is stage-1 work for the migration, and the proposal lists it
as the one thing to resolve before committing to the rewrite.

It is reached by scanning a Haskell file. Compiling rules for every language
resolves `kind` names against static tables and never parses anything, which
is why `coverage()` touches all 29 grammars and stays clean.

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
