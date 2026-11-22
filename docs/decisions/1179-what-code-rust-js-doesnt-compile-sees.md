# 1179. What code rust-js doesn't compile sees

Status: Accepted (2026-11-21, the lead's decision for #2015 and #2024).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. The answers are the
ones each analysis gave by its own check, now asked of one owner, and the
output is byte-identical where each moved.

## Context

A closed-world analysis decides from the crate's own bodies: nothing
clones this, nothing writes these items, nothing reads this key from
elsewhere. That holds only of what no code rust-js doesn't compile sees:
another crate, where this one is a library, and JS. Six checks asked it
apart and disagreed (#2015): rustc's `effective_visibilities()` and
`is_exported`, `library::reachable`, `js_facing`, `instances::seen_whole`
and an item's `visibility(..).is_public()`. A `pub fn` in a private module
was narrowed to the crate's own calls (#2025, a wrong answer), as rustc's
crate-API answer leaves out what a module's JS exports.

## Decision

Two questions, each with one owner.

**A function**, whether code outside may call it: `library::seen_outside`,
another crate's reach where the crate is a library (`library::escapes`,
`library::reachable`'s) or JS's, its module's export (`exported_function`:
a `pub` function or inherent method, private module or not, a program's as
a library's). Each analysis that narrows what a function is given asks it
(#2015).

**An item**, whether JS reaches it, in two honest answers
(`crates/rust-js-lower/src/declarations.rs`, #2024):

- `js_names(id)`: JS can name it, as its module's JS exports it, private
  module or not: a `pub` type, a `pub` field of a type it names, a `pub`
  static or thread-local key, a `pub` trait's impl through its exported
  dictionary, a function as `exported_function` says. Another crate's reach
  of an item is within it, as what it reaches is `pub`, so an item's
  `seen_outside` is `js_names` alone.
- `js_holds(ty)`: JS may hold a value of it and give it back, `js_names`'s
  and each type an export hands JS, transitively: the type in a named
  function's signature, a named field's or a static's. Asked where a value's
  identity or sharing matters: what a consumer can name of an impl's type
  (fn_pointers), whether a generic owner's values are shared or counted
  (`instances::seen_whole`).

**Fields are asked of `js_names`, not `js_holds`.** A struct only the crate
constructs is plain data: JS gives it back as it was given, and gives no
object of its own in its place (difference 60, the lead's decision 29). A
JS caller writes only what its `.d.ts` names, and a type it can't name has
no declared field. So a private struct a `pub fn` returns keeps its fields'
facts, `wholes`' whole numbers among them; taking it as JS's would lose them
for no answer that changes.

The export rules themselves (pipeline.rs, classes.rs, bindings.rs and the
like) are the owners' sources. Where one decides an export by the same
facts, it asks the same predicate, so what's exported and what's taken as
reached can't disagree.

## Consequences

- `test/architecture-escapes.test.ts` keeps rustc's crate-API answer in
  library.rs and the sites it names, and ratchets each analysis's item
  `is_public()` down as each moves onto `js_names` or `js_holds`.
- Moved onto `js_names`, each probed over the corpus, the React port and
  the metamorphic `library` rewrite, byte-identical: wholes' fields and keys,
  cell_flow's and cell_borrows' keys, read_alike's, lazy_locals', js_nones'
  field types, scopes' types, owned_by_return's fields.
- Part 2 (2026-11-21): `js_holds` is found once a session, the crate's
  types each item JS names hands it, memoized in the session's caches.
  Its one consumer is fn_pointers' type, where rustc's `is_reachable` stood:
  a type a `pub fn` of a private module hands out is JS's to hold, so a JS
  function given in its field is called without the holder as its `this`,
  `(0, own.lower)(left)`, four corpus files' output moved, as ADR 0262's
  "results change" case keeps Rust's behaviour. `instances::seen_whole`'s
  `Shared` and `Counted` ask of a generic function, and `method_family` of
  a trait: `js_names` questions, which another crate's reach is within;
  none moved output. collected's exported constant stays an export rule,
  its ratchet row saying so. The React port and the metamorphic `library`
  rewrite over the corpus as before.

## Rejected

- **rustc's `effective_visibilities()` for JS's reach.** It answers what
  another crate reaches from the crate's root, and leaves out a `pub` item
  in a private module, which its module's JS exports (#2025).
- **One answer for naming and holding.** A field JS can't name would be
  taken as JS's to write, losing facts no JS caller can break, by
  difference 60.
