# 0436. Every `dyn` that's a pair keeps its value's drop, `dyn Send` too

Status: Accepted. Extends [0049](0049-traits-and-generics.md),
[0098](0098-destructors.md) and [0141](0141-std-trait-objects.md). Amended
by [0691](0691-a-dyn-any-drops-where-the-crate-makes-one-that-does.md): a
`dyn Any` drops where the crate makes one of a value that drops.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): when a value's
destructor runs is what a program prints; and least surprise (AGENTS.md).

## Context

A `dyn` of the crate's trait is a pair, `{ value, impl }`, whose
dictionary has its value's drop, `$drop`, as Rust's vtable does (ADR
0098); dropping one is `d.impl.$drop?.(d.value)`. Two `dyn`s Rust writes
had nowhere to keep it:

- **One of std's pairs**, `Box<dyn Any>`, `Box<dyn Display>`, `Box<dyn
  Error>`: its dictionary has a place for a `$drop`, but a value with a
  destructor made one was refused (amended in 0141).
- **One of no trait but auto traits**, `Box<dyn Send>`, `Rc<dyn Send +
  Sync>`: no value at all, refused as `dyn std::marker::Send`. rustc's
  `drop/rc-trait-object-double-drop.rs` drops one.

Whether a `dyn` is a pair was answered three times, by `dynamic_trait`,
the drops' walk and a supertrait's route, each its own copy of "a trait
rust-js compiled, or `Display`, `Error` or `Any`".

## Decision

**A `dyn` is a pair where its trait has a dictionary, and where it has no
trait, of its drop alone.** Its dictionary keeps its value's drop
whatever the trait, so every pair is dropped the one way.

```rust
let boxed: Box<dyn Send> = Box::new(Noisy("boxed"));
let many: Vec<Box<dyn Send + Sync>> = vec![Box::new(Noisy("a")), Box::new(5u8)];
let unnamed: Box<dyn Send> = named;   // a `Box<dyn Named + Send>`
```

```js
const boxed = { value: ["boxed"], impl: { $drop: noisyDrop_drop } };
const many = [
  { value: ["a"], impl: { $drop: noisyDrop_drop } },
  { value: 5, impl: {} },
];
const unnamed = named;
```

- **One owner each**: `Recognition::is_pair_trait`, a trait rust-js
  compiled or std's `Display`, `ToString`, `Error`, `io::Write`, `Any`;
  and `is_pair_dyn`, a `dyn` of one or of none. `dynamic_trait`, the drops'
  walk, `dyn_drop_lost`, a supertrait's route, which unsizing is refused
  (`check_unsize`) and what's an object in JS each ask them.
- **A `dyn` of no trait's dictionary** is its drop, `{ $drop }`, or
  nothing, `{}`: what a vtable of no methods holds. An upcast to one, a
  `dyn Named + Send` as a `dyn Send`, is the same pair, whose dictionary
  has the drop already.
- **A struct whose last field is a `dyn` that's a pair**, of a trait or
  of none, is the struct given its `impl` (ADR 0409), behind a reference
  or now a `Box`, which drops that field through it, `fat.impl.$drop?.(
  fat.inner)`, where Rust drops it. A struct that holds the `dyn` itself is
  given its `impl` as it's made, `$dynCell({ size: 1 }, sevenShow())`. A
  reference to the field is its pair, read in place as its parts,
  `shown.impl.fmt(shown.inner)`, as `Expr::member` reads a field of any
  object made of what does nothing, and tidy asks it again of what's read
  in place.
- **Still refused**: a `dyn` that isn't a pair, a `dyn Debug`, the string
  it shows (ADR 0060), of a value with a destructor, or as a struct's last
  field; an `Rc` of a struct whose last field is a `dyn`, which may be
  counted; and `size_of_val` of any `dyn`, whose dictionary holds no size.

## Why

- **Rust's order of drops**, for every `dyn` Rust writes: the
  `dyn_auto_traits` corpus case drops a `Box<dyn Send>`, a `Vec` of them,
  an upcast, an `Rc<dyn Send>` cloned and one of a borrow, and
  `dyn_std_destructors` std's pairs, each against native Rust.
- **One rule, no exception**: a `dyn` is its value and its vtable, as in
  Rust, whatever its trait; what drops it is the same code for each.

## Consequences

- A `dyn Send` of a value with no destructor is `{ value, impl: {} }`, an
  empty dictionary where the value alone would do. Such a `dyn` is rare,
  and one form for every pair keeps its drop and upcasts one way.
- rustc's `drop/rc-trait-object-double-drop.rs` passes, and
  `drop/dst-coerce-unsized-drop.rs` still does, now dropping its last
  field; `traits/principal-less-objects.rs` stops at `size_of_val`. The
  `unsized_struct_dyn` corpus case, once a compile error, drops a
  `Box<Fat<dyn Show>>`, a `Box<Fat<dyn Send>>` and a struct with a
  destructor of its own, and shows a `&Fat<dyn Display>`'s field.
