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

This stopped being hypothetical while the proposal was being written.
`rules/comment.yml` uses `any`, `kind`, `regex` and `not`.
`rules/composed-conditions.yml`, added in 20e6c78, uses `pattern` with
metavariables, `follows` nested inside `follows`, and `has` with
`stopBy: end`. The rule language also has `inside`, `precedes`, `matches` with
utility rules, `constraints`, `transform`, and four strictness levels, and the
rules are moving toward it rather than away.

Reimplementing that is not a distribution project. It is a second product.

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
- **jq:** `serde_yaml` too. As of 440cfe0 jq is no longer only scc's JSON
  reader; `src/lib/compile:body` uses it to compose the top-level `except`
  with a language's own. That is a fold over two `Option<SerializableRule>`
  values in Rust.
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

Written against `main` at 4059242, confirmed with the coordinator. Since
948e4bf: tests moved out of the rule file into `rule-tests/<same-basename>.yml`
keyed by ast-grep language name (872b4c6); the top-level `except` now always
composes, including where a language replaces the matcher (440cfe0); and the
rule set grew to four.

deconfuse is a generic code matcher, so **a rule compiles to every language
ast-grep supports**, not to the languages it happens to have tests for. Tests
are coverage. `languages:` is where a rule adapts. Neither one decides scope.
`src/lib/compile` still derives scope from the keys of the test file, so this
is a proposed change, and the section below costs it out.

`src/lib/compile` becomes a fold over typed values rather than yq expressions
over YAML text. `ast-grep-config` exports `SerializableRule` publicly, so
deconfuse's own format nests ast-grep's rule syntax as a typed field:

```rust
struct Deconfused {
    id: String, severity: Severity, message: String,
    rule: SerializableRule,
    except: Option<SerializableRule>,
    languages: HashMap<Lang, Override>,
}

struct Snippets {          // rule-tests/<name>.yml, keyed by language
    valid: Vec<String>,
    invalid: Vec<String>,
}
```

The split costs the Rust version nothing. `rule-tests/<name>.yml` is a
`HashMap<Lang, Snippets>` read from a second path, and the id the runner
reports comes from the rule file rather than the test file.

### The rules ship inside the binary, and users add to them

Today `src/deconfuse` reads `$ROOT/rules/*.yml` relative to its own path and
writes compiled output to `$ROOT/.cache/`. A single downloaded executable has
no `$ROOT`, no checkout beside it, and nowhere it should be writing. So the
built-in rules come along at build time:

```rust
static RULES: Dir = include_dir!("$CARGO_MANIFEST_DIR/rules");
static TESTS: Dir = include_dir!("$CARGO_MANIFEST_DIR/rule-tests");
```

Four rules and their tests are a few kilobytes against a 43 MB binary, so size
is not a consideration. Two consequences worth stating, before the section on
how a project overrides and extends this set:

- **The pairing invariant becomes a build-time check.** A rule whose
  `rule-tests/` file is missing is a failed build rather than a rule that
  silently compiles for no languages.
- **`.cache/` disappears entirely.** Compilation produces
  `HashMap<Lang, Vec<RuleConfig>>` in memory. Nothing is written to disk, so
  there is no cache to invalidate, no `sgconfig.yml` to generate, and no
  per-(rule, language) file to name.

### Shipped, toggleable, and extensible, like eslint

Embedding is what makes `curl` and run work with no config. It is not a
closed set. The eslint shape applies directly: built-in rules ship with the
tool, each one can be turned off or have its severity changed, and a project
can add its own on top.

```yaml
# deconfuse.yml, discovered upward from the working directory
rules:
  comment-earns-nothing: off
  branches-read-as-one-shape: warning
ruleDirs:
  - .deconfuse/rules
```

Three things make this cheap rather than a feature in its own right.

**Disabling is already in the model.** `ast_grep_config::Severity` has an
`Off` variant alongside `Hint`, `Info`, `Warning` and `Error`, and deconfuse
passes `severity` through verbatim today. Turning a rule off and promoting a
hint to a warning are the same operation on a field that already exists, not
a new concept.

**User rules are not second-class.** A rule read from `ruleDirs` goes through
the same `from_yaml_string::<Lang>` call as an embedded one, so it gets the
whole rule language, the custom `gotmpl` grammar, and the same `languages:`
override machinery. The spike loads a rule from a file at run time beside the
embedded set and matches with it. There is no plugin API to design, because
there is no plugin.

**Rule ids stay flat and unique.** The spike checks the merged set for
collisions. A user rule reusing a built-in id should be an error naming both
sources rather than a silent override, since silent shadowing is how people
lose a rule they thought was running.

Precedence: embedded rules load first, `ruleDirs` extend them, `rules:`
applies severity last, so a project can turn off a built-in and ship its own
stricter replacement.

### What a user cannot add

Languages. Grammars are linked at build time, which is the whole point of
dropping `dlopen`, so a rule targeting a language deconfuse did not compile in
has nothing to parse with. Shipping all 27 of ast-grep's languages plus
`gotmpl` keeps that surface wide, at 4.9 MB gzipped, and is the reason to
prefer the all-languages build over the slim one.

If someone genuinely needs a language deconfuse does not embed, the honest
answers are a pull request or an opt-in `customLanguages` escape hatch that
reintroduces `libraryPath` and `dlopen` for that user only. The second keeps
the default install clean while admitting the capability exists. Neither needs
deciding now, and neither blocks the rewrite.

Compilation keeps today's three decisions and changes only how they are
expressed:

- **Scope** is `Lang::all()`, not the test file's keys. This is the one
  proposed change, and the section below prices it.
- **Matcher selection.** A `languages[L]` body replaces the top-level `rule`.
- **`except` composition** keeps 440cfe0's semantics. The top-level `except`
  is always ANDed in as `not:`, including for a language that replaced the
  matcher, and a language's own `except` composes with it rather than
  replacing it.

Output is `SerializableRuleConfig<Lang>` through `RuleConfig::try_from`.

Those last two are one expression over two `Option<SerializableRule>` values
and an `Option<Override>`, which is worth doing in types because the current
jq drops a case. `body` branches on whether `languages[L]` has an `except`
key, so a language block carrying both a matcher and an `except` loses its
matcher and silently falls back to the top-level `rule`. No rule does this
today, and the coordinator documents the combination as supported, so it is a
latent bug rather than a live one. A struct with two fields cannot express it.

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

### What compiling to every language actually costs

One rule made it look nearly free. Four do not. The spike surveys each rule's
default body against all 29 languages:

| Rule | Body is built on | Gaps |
| --- | --- | --- |
| `comment-earns-nothing` | `kind: comment` | 4 of 29 |
| `name-carries-no-content` | `kind: identifier` + `regex` | 7 of 29 |
| `conditions-compose-into-a-value` | `pattern` + `follows` | 18 of 29 |
| `branches-read-as-one-shape` | `if_statement` + `has`/`precedes` | 26 of 29 |

The pattern is clear and it is not about `kind` versus `pattern`. It is how
much structure the rule names. `kind: comment` names one node most grammars
share. `branches-read-as-one-shape` names `if_statement`, `else_clause`,
`return_statement` and `lexical_declaration` in one breath, and only
JavaScript, TypeScript and Tsx have all four under those names. It passes
three languages and needs an override for the rest.

Averaged over the four rules, **about half of all languages need bespoke work
per rule**, and the current four rules would owe roughly 55 overrides between
them.

This is the number to decide on, and it is a judgement call rather than a
technical obstacle:

- **Accept it**, and a rule is not finished until it has been thought about in
  29 languages. Thorough, and slow enough to discourage new rules.
- **Let a rule declare a family**, so `branches-read-as-one-shape` claims the
  languages with C-like statement structure and stays silent elsewhere. Keeps
  the checklist meaningful without pretending every rule is universal.

Whichever way it goes, nothing about the Rust recommendation changes. Both are
a predicate over `Lang::all()`.

### This conflicts with how `main` works today

`src/lib/compile:languages()` reads the top-level keys of
`rule-tests/<name>.yml`, so a language with no test snippets is never
compiled, even when `languages:` has a block for it. Scope comes from the
tests. That is the opposite of compiling to every language, and the table
above is why the two have not been reconciled by accident.

The proposal describes the target. `main` describes today. The gap between
them is a decision someone has to make, not something the packaging work
should quietly settle in either direction.

### Loud failure has one hole

Markdown and Yaml accept `conditions-compose-into-a-value`. They compile it
and then match nothing in real Markdown or YAML, verified in the spike, so the
rule is inert rather than wrong. But nothing complains, which means the
checklist reports a language as covered when the rule is meaningless there.
Silence is not the same as fit, and only tests close that gap. That is an
argument for keeping tests central even once they stop deciding scope.

### A waiver marker, reconsidered

An earlier draft concluded a waiver was unnecessary, on the evidence of
`comment-earns-nothing` alone. That was too narrow. A rule that does not apply
often still compiles and matches nothing, which is why Json needs no waiver
for the comment rule: `tree-sitter-json` has a `comment` node. But a rule that
genuinely belongs to one language family, like the new one, makes a waiver or
a family declaration look necessary rather than optional.

ast-grep offers nothing here. `SerializableRule` is atomic, relational and
composite fields only, with no no-op. `any: []` compiles and matches nothing,
an emergent property of an empty kind union rather than a documented idiom,
while `all: []` and `not: {any: []}` are both rejected. Treat `any: []` as
available and unblessed. The waiver belongs in deconfuse's own format, where
it can say something meaningful, rather than being smuggled in as a rule body
that does nothing.

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
- Shipping every grammar means deconfuse owns every grammar's bugs for the
  languages it scans. The spike found one immediately.

### Compiling to every language is not scanning with every grammar

Two separate things, worth keeping apart.

**Compilation** happens once at startup and builds a `Lang -> Vec<RuleConfig>`
map. It touches every grammar, but only to resolve `kind` names to node ids,
which is a lookup against static tables. The spike does this for all 29
languages with no trouble.

**Scanning** detects each file's language once and matches it against that
language's rules only. A `.py` file meets the Python grammar and nothing else.
That does not change.

## The one grammar deconfuse cannot ship

`tree-sitter-haskell` 0.23.1 overflows the heap while parsing. The corruption
is latent and aborts the process later, in whatever allocates next. The
spike's `haskell-abort` reproducer aborts in 20 of 21 heap layouts with
Haskell in the language set and 0 of 21 without it.

0.23.1 is both what `ast-grep-language` 0.45.3 depends on and the newest
published version, so there is nothing to upgrade to. deconfuse ships the
other 26 languages plus `gotmpl`, and adds Haskell when upstream fixes it.

This is an argument for the approach rather than against it. Linking grammars
means a bad one is a build-time decision deconfuse gets to make. Shelling out
to ast-grep would have left the same bug reachable with no way to exclude it.

## Decision

Adopt Rust. Start with stage 1, which is the spike plus rule expansion, and
keep the bash implementation until stage 4 passes.

If the rewrite is rejected, the bash-with-vendored-binaries option is the
fallback, and the first thing it should do is ship prebuilt Go template
shared objects so that `cc` stops being an install requirement.
