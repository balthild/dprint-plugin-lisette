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
  continuation indent. A chain is a run of operators that all bind equally tightly, so it
  stops where an operator binds differently and that tighter pair keeps a line of its own:

  ```lisette
  let y = a * b
    + c * d
  ```

  Breaking every operator at once would lose the shape of the expression.
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

An operand of a chain hugs more widely: anything but a group of its own, i.e. a nested chain
or a parenthesized expression. Letting one of those wrap while the operator around it stays
put would read as if that operator bound more tightly than it does, so it breaks the chain
instead.

## Imports

Imports are hoisted to the top of the file and sorted, because their order carries no
meaning in Lisette. They fall into three groups separated by a blank line: packages of the
Go standard library, Go packages from anywhere else, and the rest. A `go:` path counts as
the standard library until its first segment contains a dot, which is how Go itself tells a
module path from a standard library one. Within a group the sort key is the alias when the
import has one (`_` sorts as `_`), otherwise the path with its `go:` prefix removed. A
comment written above an import travels with it.

## Comments

Comments belong to the author; the formatter only decides where they sit.

- A comment on the same line as the previous item trails it. The first comment that starts
  a new line, and everything after it, leads the next item. In a chain of operands this is
  what keeps an operator together with the operand it belongs to: a comment written after
  `map(f)` stays on that step's line, and one written above a step goes above the operator.
- A comment anywhere between two operands forces the whole chain to break. On one line the
  comment would land between an operator and its operand.
- A comment detached from the first item of a file by a blank line stays at the top as a
  file header rather than travelling with that item.
- Two or more blank lines between two items become one. Blank lines inside a comment block
  are kept, one for one.

Anything emitted straight from the source must mark its comments as handled, or the same
comment is emitted a second time.

## Limits of the syntax tree

Some things cannot be formatted because `lisette-syntax` 0.13.0 does not carry them:

- **Statement semicolons are dropped.** The parser consumes them with `advance_if`, so
  nothing downstream can know whether one was written. The upstream formatter behaves the
  same way.
- **`@rawgo` has no span.** It is rendered from its own text. As a statement in a block it
  also cannot take part in the comment bookkeeping, so a comment written above it is
  emitted above the statement after it instead.
- **An interpolation in an f-string** is printed on one line, since a break would be a
  break inside a string. One written over several lines is kept as it was written, because
  flattening it could swallow a line break that belongs to a nested string.
