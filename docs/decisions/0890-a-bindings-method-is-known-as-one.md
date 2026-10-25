# 0890. A binding's method is known as one, whatever its name

Status: Accepted. The review's P2: a fact of Rust's on the JS tree in
place of a JS shape read back.

Case ([0262](0262-when-rust-and-js-disagree.md)): results could change, so
Rust's order is kept. No program of the corpus changes; the guard is for
the one that would.

## Context

`Expr::only_reads` and `runs_program_code`, which tidy asks before it moves
what's evaluated past a call (`first_applied`) and which a format's order
asks (`formats_out_of_order`), took a call of a method named `map`,
`join`, `includes` and the like as JS's own, which only reads: they read
the name. A binding's method of the same name, `store.join()` of a JS class
the program declares, runs what it likes, and was taken as JS's own too.
Only a method of a JS value of any shape was marked, `Known::Getter`.

## Decision

- **Each binding's method is marked `Known::Binding`** where the binding's
  call is lowered (`binding_calls.rs`), beside `Known::Getter` where a
  getter may answer it: facts are a set (ADR 0889), so
  `answered_by_a_getter()` adds its fact rather than replacing them.
- **`calls_only` never counts a call of one as JS's own**, whatever its
  name; JS's own methods of what Rust's types say is text or an array are
  as before.

## Consequences

- Held by a unit test, `a_bindings_method_runs_what_it_may`: no program's
  JS changes, so no mutation's corpus test could catch it.
