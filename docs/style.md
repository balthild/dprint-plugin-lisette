# Formatting style

The formatter builds dprint print items and lets the printer decide where the lines go.
This document records the style decisions it makes and the reasons behind them.

## Breaking

A construct either fits on one line or every one of its parts goes on its own line. There
is no partial grouping: if one argument of a call has to break, they all do.

- Comma-separated lists take a trailing comma when they break, except the lists between
  angle brackets. `List<int, string>` and `call<int, string>(x)` put one type per line and
  no comma after the last one, because a trailing comma there is not valid Lisette.
- The operands of a binary chain break *before* the operator, with one level of
  continuation indent.
- Interfaces and `select` statements separate members with line breaks rather than commas,
  one member per line.
- The methods of an `impl` block are always separated by a blank line.

## The author's line breaks

Inside an array literal, a struct literal, or the field list of a struct definition, a
line break the author wrote is kept, so the list stays one field per line. Nothing else
about an expression's layout is the author's to decide.

## Hugging

A value that opens a brace of its own does not force the list around it to break. This is
what keeps the idiomatic

```lisette
do_work(arg1, arg2, |value| {
  ...
})
```

on one opening line. The values that hug are lambdas, blocks, matches, tuples, struct
literals and array literals.

## Imports

Imports are hoisted to the top of the file and sorted, because their order carries no
meaning in Lisette. `go:` imports come first, then the rest, with a blank line between the
two groups. Within a group the sort key is the alias when the import has one (`_` sorts as
`_`), otherwise the path with its `go:` prefix removed. A comment written above an import
travels with it.

## Comments

Comments belong to the author; the formatter only decides where they sit.

- A comment on the same line as the previous item trails it. The first comment that starts
  a new line, and everything after it, leads the next item.
- A comment detached from the first item of a file by a blank line stays at the top as a
  file header rather than travelling with that item.
- Two or more blank lines between two items become one. Blank lines inside a comment block
  are kept, one for one.

Anything emitted straight from the source must mark its comments as handled, or the same
comment is emitted a second time.

## Limits of the syntax tree

Some things cannot be formatted because `lisette-syntax` 0.12.5 does not carry them:

- **Statement semicolons are dropped.** The parser consumes them with `advance_if`, so
  nothing downstream can know whether one was written. The upstream formatter behaves the
  same way.
- **`@rawgo` has no span.** It is rendered from its own text. As a statement in a block it
  also cannot take part in the comment bookkeeping, so a comment written above it is
  emitted above the statement after it instead.
- **An interpolation in an f-string** is printed on one line, since a break would be a
  break inside a string. One written over several lines is kept as it was written, because
  flattening it could swallow a line break that belongs to a nested string.
