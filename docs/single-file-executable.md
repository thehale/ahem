# Distributing deconfuse as one file

## The ask

Someone should be able to download one file, run it, and get findings. Today
they need mise, four binaries resolved through it, and a C compiler that runs
the first time a Go template is scanned. That is a fine way to develop
deconfuse and a bad way to use it.

## Recommendation

Rewrite deconfuse as a Rust binary that links ast-grep's library crates and
the Go template grammar directly. Keep `rules/*.yml` exactly as it is.

A working spike lives in [`spike/`](../spike/README.md). It compiles the
grammar at build time, links it into the binary, and matches
`rules/comment.yml`'s own test snippets using ast-grep's rule deserializer.
No `cc`, no `dlopen`, no download. The binary is 4.9 MB gzipped with all 27
of ast-grep's languages, 1.8 MB with the six the rules currently target.

The rest of this document is why the other three candidates lose.

## What the four dependencies are actually for

Naming them honestly shrinks the problem.

- **ast-grep** is the matching engine. It is the only one that is hard to
  replace, and the only one worth keeping.
- **yq** reads `rules/*.yml` and rewrites each rule into one ast-grep rule per
  language. Any language with a YAML parser replaces it.
- **jq** parses scc's JSON. It exists only because scc exists.
- **scc** tells `src/lib/classify` what language each file is. It is 18 MB of
  line-counting tool used for one field of its output.

So the real question is ast-grep, plus one Go template grammar. Everything
else is a library call in any candidate.

## The two questions that decide it

### Can the grammar be embedded?

ast-grep's CLI loads custom languages one way: `libraryPath` in
`customLanguages`, which `ast-grep-dynamic` resolves with `libloading`, that
is, `dlopen` on a platform shared object. The Python and Node bindings
inherit the same constraint. `ast_grep_py`'s `CustomLang` TypedDict has
exactly one required field, `library_path`.

That matters more than it sounds. Any candidate that drives ast-grep as a
subprocess or through a published binding cannot embed the Go template
grammar. It can at best ship a prebuilt `.so`/`.dylib`/`.dll` per platform
next to the executable, which is a second file and the same
cross-compilation work, bought without the benefits.

Only a candidate that links `ast-grep-core` as a library escapes this,
because the `Language` trait takes a `TSLanguage` from anywhere, including a
symbol statically linked from `parser.c`. The spike does this in twenty
lines.

### Can ast-grep's rule semantics be reused?

`rules/comment.yml` today uses `any`, `kind`, `regex` and `not`. The rule
language it is written in also has `pattern` with metavariables, `inside`,
`has`, `precedes`, `follows` with `stopBy`, `matches` with utility rules,
`constraints`, `transform`, and four strictness levels. Reimplementing that
is not a distribution project. It is a second product.

**Any candidate that cannot reuse ast-grep's rule engine is disqualified.**
That is the whole evaluation in one sentence, and it eliminates Go, Ruby and
every other language without Rust FFI.

## Candidates

### Bash with vendored binaries

Ship a self-extracting archive containing the four binaries plus the current
scripts.

- **Grammar:** no. Needs a prebuilt shared object per platform alongside, or
  keeps `cc` at run time.
- **Rule semantics:** reused, unchanged.
- **yq, jq, scc:** still there, still shipped.
- **Release:** 83 MB of binaries, 21 MB compressed, per platform, each pinned
  and updated by hand. Every dependency's platform matrix becomes deconfuse's
  platform matrix, and deconfuse supports the intersection.
- **Migration:** trivial. This is the cheap option and it stays cheap.

It fails the ask on the detail that matters. "Download one file, run it" is
not a 21 MB shell script that unpacks four binaries into a cache directory
and still needs a compiler for Go templates. It also freezes today's
accidental complexity, the ast-grep quirks the current code works around,
into the shipped product.

Worth keeping as a fallback if the rewrite is rejected. Not worth doing
first.

### Rust

- **Grammar:** yes, proven. `build.rs` runs `cc` at build time on a vendored
  `parser.c`; the released binary links only libc.
- **Rule semantics:** reused verbatim. `ast_grep_config::from_yaml_string` is
  ast-grep's own deserializer, generic over the caller's language type.
  Nothing is reimplemented, and future ast-grep rule features arrive with a
  version bump.
- **yq:** `serde_yaml`, already a transitive dependency.
- **jq:** gone with scc.
- **scc:** `SupportLang::from_path` covers extensions. Shebang detection is
  the one thing genuinely lost, and it matters here, because `bin/ci` and
  `src/deconfuse` have no extension and scc classifies them by `#!` today.
  Reading a first line and matching an interpreter is roughly twenty lines.
  Count it as work, not as free.
- **Release:** a GitHub Actions matrix, one native runner per target. See
  below.
- **Migration:** a rewrite, but a stageable one.

Three things get *simpler*, not just faster, which is the strongest argument
for the rewrite:

1. **The per-language scan dance disappears.** `check` currently sorts files
   by language, then runs ast-grep once per language with
   `languageGlobs: {lang: ["*"]}`, because `languageGlobs` matches the file
   name rather than the path. A library caller detects the language itself
   and matches each file against the rules for it. One pass, no globs.
2. **Rule id mangling disappears.** Ids are suffixed with `.{language}`
   because ast-grep demands they be globally unique across a rule directory.
   A library caller keys rules by language in a map and reports the id the
   author wrote.
3. **The test runner becomes deconfuse's own.** `ast-grep test` needs a
   parallel `rule-tests/` tree, a `testConfigs` block, and
   `--skip-snapshot-tests` to suppress a feature deconfuse does not use.
   Running `valid` and `invalid` snippets directly is the twelve-line
   `report` function in the spike.

The cost: a 43 MB binary on disk in the all-languages build, and a
contributor toolchain that is Rust rather than bash. Both are acceptable.
Nobody notices 43 MB on disk when the download is 4.9 MB, and ast-grep's own
binary is 50 MB for the same reason.

### Go

- **Grammar:** yes in principle. `go-tree-sitter` compiles grammars through
  cgo at build time. But cgo forfeits Go's cross-compilation, which is the
  main reason to pick Go here, so the release story ends up no better than
  Rust's.
- **Rule semantics:** **reimplemented.** There is no Go port of ast-grep's
  rule engine and no cgo binding to one. This disqualifies Go, and it is not
  close.
- **scc:** the one real upside, since scc is Go and its detection could be
  imported rather than shelled out to. Worth a fraction of the rule engine.

### Ruby, Python, and other scripting languages

- **Grammar:** no. `ast_grep_py` registers custom languages by
  `library_path`, verified above, so the shared object problem survives.
- **Rule semantics:** reusable through `ast_grep_py`, which is the one thing
  this family has over Go. Ruby has no equivalent binding.
- **Release:** the binding is a 43 MB compiled wheel, so a single file still
  means bundling an interpreter plus a native extension with PyInstaller or
  Tebako, built per platform. That is Rust's release matrix with an
  interpreter stapled on.

Reusing ast-grep through Python buys a worse version of what linking it
buys.

## Rule compilation

deconfuse is a generic code matcher, so **a rule compiles to every language
ast-grep supports**, not to the languages it happens to have tests for.
`tests:` is coverage. `languages:` is where a rule adapts. Neither one decides
scope.

`src/lib/compile` becomes a fold over typed values rather than yq expressions
over YAML text. `ast-grep-config` exports `SerializableRule` publicly, so
deconfuse's own format nests ast-grep's rule syntax as a typed field:

```rust
struct Deconfused {
    id: String, severity: Severity, message: String,
    rule: SerializableRule,
    except: Option<SerializableRule>,
    languages: HashMap<Lang, Override>,
    tests: HashMap<Lang, Snippets>,
}
```

Compilation keeps today's three decisions and changes only how they are
expressed:

- **Scope** is `Lang::all()`, not `tests` keys.
- **`except` folding** builds `all: [rule, not: except]` as a value. Today it
  is assembled as JSON text and re-parsed through yq's `from_json`. That
  round-trip disappears.
- **Per-language override** keeps the current semantics exactly. A
  `languages[L]` body replaces the default; a `languages[L].except` narrows
  it.

Output is `SerializableRuleConfig<Lang>` through `RuleConfig::try_from`.

### Gaps are compile errors, not silent holes

Compiling to every language means rules meet grammars they do not fit.
`comment` is not a node kind in every language, which is why
`rules/comment.yml` already overrides Rust with `line_comment`/`block_comment`.

ast-grep already complains loudly about this, which is the best news in the
evaluation. `RuleConfig::try_from` returns `MissingPotentialKinds` when a
rule's `kind` resolves to no node in a grammar, so an ill-fitting rule fails
to build rather than matching nothing. The spike compiles a bare
`kind: comment` rule against all 29 languages and gets:

```text
4 of 29 languages need an override: Java, Kotlin, Markdown, Rust
```

That output is a coverage checklist. deconfuse should surface it as one: `test`
fails when a rule does not compile for a language, and the error names the
language, so adding a rule tells you immediately which `languages:` overrides
it still owes.

### A waiver marker is probably not needed

The obvious worry is a rule that genuinely does not apply to a language, which
would make the checklist noise rather than work. It turns out to be rarer than
it sounds, because **a rule that does not apply still compiles and matches
nothing**. Only a rule whose `kind` names no node at all is rejected.

Json is the case that looks like it needs a waiver and does not.
`tree-sitter-json` has a `comment` node, so `kind: comment` compiles there and
finds comments in the files that have them. All four real failures, Java,
Kotlin, Markdown and Rust, are languages that *do* have comments under a
different node name. Every one of them is work, not a waiver.

ast-grep has no conventional no-op marker; `SerializableRule` is atomic,
relational and composite fields only. If one is ever needed, `any: []`
compiles and matches nothing, an emergent property of an empty kind union
rather than a documented idiom. `all: []` and `not: {any: []}` are both
rejected. Treat `any: []` as available but unblessed, and do not build the
format around it until a rule actually needs it.

## Releasing

Five targets, each on a runner that builds natively, so nothing
cross-compiles and `cc` always matches the host.

| Target | Runner |
| --- | --- |
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` |
| `aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` |
| `x86_64-apple-darwin` | `macos-13` |
| `aarch64-apple-darwin` | `macos-latest` |
| `x86_64-pc-windows-msvc` | `windows-latest` |

A musl build for Alpine is the one target needing more than a native runner;
`cross` or a musl toolchain on the Ubuntu runner covers it, and it can wait
for someone to ask.

Clean release builds took 15 seconds on this machine, so release CI is not a
consideration. ast-grep runs this exact shape of matrix for a binary with 27
C grammars in it, which is the best evidence that the approach holds up.

Install is a `curl | sh` script that reads the platform and fetches the right
release asset, plus the GitHub release page for people who would rather not
pipe. Because the result is a plain static binary, mise's `github:` backend
can install it too. deconfuse becomes installable *by* mise without requiring
mise.

## Migration

The rewrite is not incremental, but it is stageable, because `rules/*.yml` is
the interface and it does not change. Each stage ships behind `bin/ci`
running both implementations, so divergence shows up as a failing check
rather than a surprise.

1. **`deconfuse test`.** Rule self-testing only: read `rules/*.yml`, expand to
   every language, fail on any that does not compile, then run `valid` and
   `invalid`. No file walking, no detection. This stage also has to resolve
   the Haskell abort described below. `bin/ci` runs the Rust one and the bash
   one and compares.
2. **`deconfuse check PATH...`** on explicit paths, extension-based detection.
   Diff its output against the bash version over this repo and a Go template
   project.
3. **Directory walking and shebang detection.** The `ignore` crate, already a
   transitive dependency of `ast-grep-language`, gives `.gitignore`-aware
   traversal. Shebang sniffing closes the scc gap.
4. **`rules` and `compile`.** `rules` lists what is loaded. `compile` becomes
   a dump of the deserialized rule, which is a better debugging tool than the
   current cache dump because it shows what will actually match.
5. **Delete `src/`, delete the four tools from `mise.toml`, add the release
   workflow.** `mise.toml` keeps the development tools and gains Rust.

Stages 1 through 4 leave the repo working at every commit. Stage 5 is the
only irreversible one, and by then both implementations have agreed on real
input for four stages.

## What this costs

An honest tally, since the recommendation is a rewrite of a tool that is one
day old.

- 88 lines of `src/deconfuse` plus 161 lines of `src/lib/*` get thrown away.
  That is the good news about doing this now rather than in six months.
- Shebang-based language detection has to be written rather than inherited.
- Contributors need a Rust toolchain instead of bash. The rules, which are
  where contribution actually happens, stay YAML.
- deconfuse takes on ast-grep's library crates as a versioned dependency
  rather than a pinned binary. They are published, versioned together with
  the CLI, and currently at 0.45.3, matching the pin in `mise.toml`.
- Compiling to every language means deconfuse owns every grammar's bugs. The
  spike found one immediately.

## The one open risk

Matching a rule against Haskell aborts the spike with
`corrupted size vs. prev_size`, reproducibly. It is not the vendored grammar,
not `tree-sitter-haskell` on its own, and not ast-grep, all of which the
spike's README rules out individually. It is somewhere in using
`ast-grep-core` as a library, and it is not root-caused.

This matters more under compile-to-every-language than it would have under
compile-to-tested-languages, because every scanned file now meets every
grammar. Resolve it in stage 1, before the rewrite is committed to. It is
plausibly a small fix, a `TSLanguage` handed out per call where ast-grep's own
CLI hands out one, but that is a guess until someone runs it under a sanitizer.

## Decision

Adopt Rust. Start with stage 1, which is the spike plus rule expansion, and
keep the bash implementation until stage 4 passes.

If the rewrite is rejected, the bash-with-vendored-binaries option is the
fallback, and the first thing it should do is ship prebuilt Go template
shared objects so that `cc` stops being an install requirement.
