# 0939. An iterator that's an iterable in JS is made the JS iterator it steps where its `IntoIterator` dictionary gives it

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0552](0552-an-into-iterator-held-as-it-is-is-given-as-it-is.md).
Amended by [0971](0971-an-escape-is-the-array-of-what-it-gives.md): an
escape is the array of its `char`s, so the dictionary gives it as it is.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
With std's structs alone, `s.chars().flat_map(char::escape_unicode)` threw
`it.next is not a function`.

## Context

A char's `escape_unicode()`, `escape_debug()` and `escape_default()` are
the text they give, `$escapeUnicode(c)`, a JS iterable, which what steps
one makes a JS iterator, `Iterator.from(..)`. The `IntoIterator`
dictionary of an iterator gave it as it is, `(items) => items`, whose
`next` std's compiled `FlatMap` then called.

## Decision

- **The `IntoIterator` dictionary of an iterator that's an iterable in JS,
  a text escape (`is_text_escape`), makes it the JS iterator it steps**,
  `Iterator.from(items)`, as what steps one where it's known does
  (`js_iterator`); another is itself.

## Consequences

- ADR 0908's `differs` is 6, from 7: `escape_iterators.rs` runs as native
  Rust does with std's structs alone, and joins the suite's few.
- The normal build's JS doesn't change.
- Mutations: `iterable-into-iter-itself`.
