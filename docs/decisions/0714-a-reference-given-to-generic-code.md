# 0714. A reference given to generic code is what it refers to

Status: Accepted. Amends [0162](0162-generic-as-ref.md), whose `AsRef`
dictionaries were of text and sequences alone.

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change:
each was refused, or threw.

## Context

Giving a reference where a generic parameter is, the everyday
`fn shout<S: AsRef<str>>(s: S)` called as `shout(&text)`, found three
gaps:

- **`&T` of the crate's own `AsRef`** was refused: "implementation evidence
  for `<&Text as AsRef<str>>`". std's impl for it, `impl<T: ?Sized +
  AsRef<U>, U: ?Sized> AsRef<U> for &T`, passes it on to `T`'s, and no
  dictionary was made of that.
- **`&PathBuf` as an `AsRef<Path>`** was refused too: at a call, `as_ref()`
  of a path as a `Path` was the value (ADR 0173), but a dictionary asked
  another question, of text and sequences alone. Two answers to one.
- **`take2(&mut range)`**, a `&mut` to a range given as an `I: Iterator`,
  threw: the range object was given, which JS can't step.

## Decision

- **A reference's dictionary, by std's impl that passes the trait on, is
  what it refers to's**: an impl of std's whose type is `&T` or `&mut T`
  of its own parameter `T`, and which asks `T` the same trait
  (`forwarded_to_referent`). A reference is the value it refers to in
  JS. It's the last thing `dictionary` tries, so nothing it made before
  changes.
- **Whether std's `AsRef<U>` of a `T` is the same value is
  `as_ref_is_itself`'s**, of a call of `as_ref` and of a dictionary alike:
  text as a `str`, a sequence as a slice, what a `Box` or an `Rc` holds, and
  text or a path as a `Path`.
- **A `&mut` to a range given as an iterator is the range, stepped where
  it is**, `$iterator(it, $rangeNext)`, so its lender goes on from where
  it's left, as Rust's does (`as_js_iterator`).

## Consequences

- Held by `test/corpus/references_given_to_generics.rs` against native
  Rust: `Display`, `Debug`, `PartialEq`, `Ord`, `Hash`, `Fn`, `Write` and
  `AsRef` through references, and `&mut` ranges, inclusive too; and by the
  mutations `reference-dictionary-unforwarded`, `as-ref-path-not-itself`
  and `mut-range-given-raw`.
- That the impl asks `T` the same trait is a check no program here
  reaches: std's impls for `&T` that don't, `Clone`'s, are made before.
