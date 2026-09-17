---
name: add-a-rule
description: >-
  Add a rule to ahem. Use when a pattern a reviewer keeps pointing at should
  become a finding the tool reports.
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

Choose `warning` when the finding is right nearly every time it fires, `hint`
when it is a judgement a reader may decline. There is no `error`, because
nothing here blocks.
