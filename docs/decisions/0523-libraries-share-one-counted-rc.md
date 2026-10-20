# 0523. Libraries share one counted `Rc` representation

Status: Accepted. Amends [0320](0320-counted-rcs.md) and
[0494](0494-a-generic-rc-is-counted-where-it-may-drop.md), whose
representation analysis was local to a crate; extends
[0100](0100-separate-crates.md) and
[0414](0414-a-library-on-npm-as-rescripts-are.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): counts, identity,
weak references, and the last owner's destruction agree across crates.
Elision is A only where the whole program can't observe it.

## Context

`Counted` decides whether an `Rc<T>` is its value or a counted handle,
`{ value, strong, weak }`. A library and its consumer can't make that
choice apart for a value they share, so a library refused every counted
`Rc` ("a counted `Rc<T>` another crate may share"): itertools' `tee`, and
nineteen corpus cases as libraries. Generic functions, callbacks, trait
objects and fields make a list of public signatures insufficient.
Exporting counted pointee paths leaves absence ambiguous ("not used" or
"used uncounted"), can't name a generic instantiation, and reading a count
in a library only to change its ABI would shape valid Rust for the
compiler.

A trial of counting every `Rc` found four places a counted `Rc` dropped
what an uncounted one didn't, each closed first: a downcast beside one
([0520](0520-a-counted-rc-of-a-dyn-is-its-pair.md)), a `Box<dyn Fn>`
holding one ([0521](0521-a-dyn-fn-carries-its-drop.md)), and a closure
holding one, cloned ([0522](0522-a-closure-cloned-apart-is-made-again.md)).

## Decision

**A rust-js library counts every `Rc` and `Weak`, and so does a crate
using one.** `counted_rcs` answers it, given `shares_rcs`, a library or a
crate with one, before drop analysis; no consumer works out a second
answer for an imported value.

```text
library A ---- counted handle ----> application ----> library B
                     |
              one shared count
```

- **One library ABI, recorded once**: `library::VERSION` 3 requires counted
  `Rc`s. A library of 2 is refused ("unsupported library ABI 2; expected
  3"), a transitive one too, as each library's manifest is read; a missing
  contract is never read as an uncounted one. There's no per-type list,
  flag or option. `tooling/manifest.js`'s check is the Cargo tests'.
- **One handle crosses every boundary**: arguments, returns, fields,
  closures, dictionaries and generic instantiations pass it as it is,
  never wrapped at a call.
- **Drops stay the drop machinery's**: the last owner runs the pointee's
  drop once, whichever crate drops it.
- **A program with no rust-js library keeps the elision**: it sees the
  whole program. Native-only binding crates aren't libraries here.
- **The refusal goes**: `counted_here` and its message are removed.

Implementing it found three gaps, each closed here:

- **A `&mut` to a counted `Rc` is a cell** (`is_boxable`), as a number's
  is (ADR 0099): its owners share the object, so `*r = v` gives the
  place another `Rc`, `r.value = v`, and `Rc::make_mut` through a `&mut`
  argument writes the place, `rc.value = $makeMut(rc.value, ..)`. An
  `Rc`'s op given a `&mut` reads its box's `value`; `make_mut(&mut rc)`
  of a variable takes the variable itself. Writing an `Rc` or a `Weak`
  through a `&mut` was refused in any crate before.
- **A closure another crate gives back drops whole**: it carries its drop
  (ADR 0484), which its maker wrote with the same counts.
- **A library's trait's `dyn`** is the consumer's too:
  [0524](0524-a-librarys-dyn-in-its-consumer.md).
- **A generic function's fields are read in its own terms**: asking
  whether a call gives one a `&mut` inside a field normalized them in the
  caller's environment, where `I::Item` is the caller's `A`, and rustc
  panicked on itertools' `Tuple2Combination`, which `tee`'s refusal had
  hidden: `callee_field_own_terms`.

## Why

- **The same program across crates**: `three crates: rc_shared` clones an
  `Rc` in one crate, counts it in another and drops its last owner in
  each, with `Weak::upgrade` before and after; `rc_generic` gives a
  generic library's `Rc<T>` of a value with a destructor to a trait
  object, a callback and another `Rc`, with `make_mut` and `get_mut`
  through `&mut`; `rc_through_param` gives one to libraries with no `Rc`
  in sight, through their type parameters. Each runs against native Rust.
- **A library changed to read a count** keeps its consumers' behavior
  once rebuilt: "a library that starts reading a count".
- **A library's internal change can't break callers whose Rust is still
  valid**: the representation doesn't depend on what anyone reads.

## Consequences

- A library-backed app pays for counts its own code doesn't observe: the
  Cargo test "the app's crate is a program" shows `Rc::new(5)` as `5` in a
  program and `{ value: 5, strong: 1, weak: 0 }` once another crate uses
  it. A future elision has to keep this ABI; it can't add an option or ask
  for count reads.
- Twenty corpus cases lose `library-refused`.
- Mutations stop counting in a producer or a consumer, treat a `&mut Rc`
  as the object, read a box for its `Rc`, refuse a returned closure from
  another crate, and accept the old ABI.
- `Arc` follows the single-agent policy where it shares this machinery; no
  threads or cross-worker ABI are added.
