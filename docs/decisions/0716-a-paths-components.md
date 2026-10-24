# 0716. A path's components are std's `Component`, as any enum is

Status: Accepted. Extends [0405](0405-a-paths-methods.md),
whose components were its runtime's alone, and
[0033](0033-enums-with-fields.md), to std's enums whose `PartialEq` or order is
derived.

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results
change: each was refused, or, given as a path, threw.

## Context

`Path::components()` (#185) was refused, and `OsStr::new` with it, though
`path.js` goes over a path's components as std's `Components` does. Lifting
it found what `Component`'s values needed of std's enums:

- `{:?}` of one reached `Component::Prefix`'s `PrefixComponent`, a
  Windows path's prefix, which rust-js refused to show;
- `==` of one, and of `Bound`, was refused: std's enums other than
  `Option`, `Result` and `Ordering` weren't compared;
- sorting them, or keeping them in a set, was refused, and so was ordering
  the `OsStr` a `Normal` holds;
- one given where a path is, `buf.push(c)`, `Path::new(&c)`, or to `P:
  AsRef<Path>`, was given its object, which `path.js` threw on.

## Decision

- **`p.components()` is `$pathComponents(p)`**, an array of std's
  `Component` as any enum is (ADR 0033): `"RootDir"`, `"CurDir"`,
  `"ParentDir"`, `{ TAG: "Normal", _0: "usr" }`, made of `$pathParts`'s.
  A path's iterators, `iter()`'s, `components()`'s and `ancestors()`'s, step
  an array (`is_path_items`), so `next_back` and `rev` are an array's.
- **A component's text**, `c.as_os_str()` and std's `AsRef<Path>` and
  `AsRef<OsStr>` of it, is `$componentText(c)`. `as_ref_is_component_text`
  answers it, of a call and of a dictionary alike; what a path's method is
  given as a path, `path_given`, is it too. `Path::new` and `OsStr::new`
  are a path's methods, given what they're given as a path.
- **A variant no value is ever made of, `Component::Prefix`**, is nothing
  to show or order: `never_made`. A Unix path has no prefix.
- **std's enums whose `PartialEq` is derived compare as any enum**,
  `$eq` (`std_enum_derives_eq`), as their derived `{:?}` shows them; one
  whose order is derived orders by variant, then fields, as the crate's
  own; and keys a map by value where its fields do.
- **An `OsStr`'s `==` and order are its text's**, by code point, which is
  its bytes' order (`is_os_text`).
- **A unit variant written in place is no tagged one**: its tag test is
  `false`, not `"RootDir".TAG === "Normal"`.

## Consequences

- Held by `test/corpus/path_components.rs` against native Rust, and the
  mutations of `path.js`, `path.rs`, `ordering.rs`, `std_impls.rs`,
  `map.rs` and the recognition questions.
- Still refused: `Components::as_path()` and `Iter::as_path()`, of what's
  left, which an array doesn't keep; `PathBuf::from(&c)` and collecting
  components into a `PathBuf`.
