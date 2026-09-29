---
name: add-a-rule
description: >-
  Add a rule to ahem. Use when a pattern a reviewer keeps pointing at should
  become a finding the tool reports, or when deciding whether it should.
license: MPL-2.0
---

# Add a rule

A rule is `rules/<id>.yml` and `rule-tests/<id>.yml`, both named for the `id`
the rule declares, which the build enforces.
Any pair already in those directories shows the shape: an `id`, a `severity`,
a `message`, the `rule` itself, an optional `except`, and a `languages` block
carrying whatever a grammar needs said differently. What follows is what
reading one does not say.

Scope a rule to what it can get right. A matcher that reaches a few languages,
or only the clearest shape of the problem, is finished work; widening it later
is a small edit, and a rule that fires where it should not is one somebody
turns off.

## When to recommend no rule

A request for a rule can be declined, and declining it with the evidence is
finished work. Weigh it before writing a matcher, and bring a recommendation
rather than a half-built engine change.

Recommend against the rule when any of these holds:

- The judgement needs more than the file in front of it. The engine reads one
  file at a time. A pattern judged by its callers, its types, or its history
  needs a resolver for each language, and resolving calls is a type checker's
  work.
- The only form a matcher can see leans on convention rather than syntax. A
  name that signals privacy does not stop a caller elsewhere, so the rule
  misfires wherever the convention is broken.
- The pattern turned up in one codebase, and a reviewer already caught it by
  hand. The rule would guard against a mistake that is rare and cheap to fix.
- A compiler, a linter, or the language's own tooling already reports it.

The cost is the engine code and the misfires a rule adds, against the reviews
it spares. When the cost is the larger, say so and name what the rule would
have caught, so the decision rests on the evidence.

## Steps

1. Write the generic matcher under `rule:` in `rules/<name>.yml`.
2. Write `rule-tests/<name>.yml`: per language, one snippet the rule flags and
   one it leaves alone. A language reaches the rule from here and nowhere
   else, so a `languages:` block for a language with no snippets fails the
   build, and a language with snippets and no block gets the generic matcher.
3. `cargo build --release`, then `target/release/ahem test`, then `bin/ci`.

## The matcher

Read the tree before writing it: `cargo run --example ast -- LANGUAGE PATH`.
Node names belong to the grammar, not the language. A function body is a
`block` in Rust, a `statement_list` inside a `block` in Go, and `statements`
in Kotlin, so a rule that counts children misses a level and matches nothing.

A `kind:` rule reaches most grammars unchanged, which is why
`comment-earns-nothing` overrides six of its twenty-eight languages. A
`pattern:` rule is written in one language's syntax and holds almost nowhere
else: `conditions-compose-into-a-value` overrides sixteen of seventeen. A
pattern that is not a whole file on its own needs `context:` and `selector:`,
as the C and Java overrides in `rules/condition-is-already-the-value.yml` do.

Compiling is not matching. A matcher naming a field or kind the grammar does
not have compiles and then flags nothing, which reads as a quiet pass rather
than an error. Where that is the failure mode, find the boundary by hand:
`function-holds-more-than-a-story` was confirmed by generating functions of
five to ten statements in each language and watching it flip at exactly eight.

## Nested code

A file can hold code in a language other than its own, and the engine hands
each such chunk to the grammar that owns it. Only that grammar's rules see the
chunk, so a rule reaches nested code through the nested language's snippets and
nothing else. A snippet written in the outer language tests the handoff, and an
`invalid:` entry written as a `line` and a `source` pins the line of the outer
file the finding lands on. Keying a snippet under the outer language reaches
that language too, so it needs a matcher that compiles against its grammar.

## Narrowing

Narrow inside the matcher when the false positive is a shape. `Ok(false)` is a
constructor rather than a flag, so the Rust block walks up to the
`call_expression` and excludes `Ok`, `Err` and `Some`. That narrowing does not
carry to another language, because Kotlin reaches its callee through a
`navigation_expression`. Every narrowing earns a `valid:` snippet, or the next
edit quietly undoes it.

`except:` answers the other case, where the match reads the syntax correctly
and the intent wrongly. Its entries are lint directives — `shellcheck`,
`noqa`, `eslint` — because a comment another tool reads is configuration, and
deleting it changes what that tool does. A top-level `except` composes into
every language, including the ones that replace `rule` outright.

## Message and severity

The message is the whole finding. It is guidance and not an order: it explains
why the code reads badly and where what the code is carrying would sit better,
and the agent receiving it makes the call. That agent takes it as a general
preference, so a message wider than its matcher causes edits the rule never
asked for. Keep it
inside what the matcher can see. `branches-read-as-one-shape` reaches
statements alone, and its message argued for an expanded if/else without
saying so, which pushed an agent twice into expanding one-line conditionals it
never matched. The message now names where it stops.

Keep the message true in every language the rule reaches, and put advice that
holds in one language alone under that language in the `languages:` block. An
`addendum:` there is appended to the message, for a recommendation only that
language can act on. A `message:` there replaces it, for a language whose
reason differs. Neither changes the matcher.

Choose `warning` when the finding is right nearly every time it fires, `hint`
when it is a judgement a reader may decline. There is no `error`, because
nothing here blocks.
