# 0584. A `#[repr(transparent)]` struct is its field

Status: Accepted. Amends [0448](0448-a-transparent-struct-is-its-field.md),
whose struct was `[field]` and a `transmute` made or read it. Extends
[0020](0020-structs-and-tuples.md)'s structs.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. Rust lays the
struct out as its field, and nothing a program does tells them apart.
ADR 0448's Case R, `ptr::eq` of a `&T` and the `&Wrapper` made from it, is
gone: they're one value.

## Context

React's scheduler's task holds its work as a function that gives what's
left of it, of its own type: `type Callback = boolean => ?Callback`.
Rust's type alias can't be of itself, so the Rust is a newtype, its
layout its field's:

```rust
#[repr(transparent)]
pub struct Callback(Box<dyn FnMut(bool) -> Option<Callback>>);
```

A tuple struct was its array (ADR 0020), so the task's callback was
`[f]`, called `callback[0](didTimeout)`, where upstream's is the function.
ADR 0448 made a `transmute` between such a struct and its field a struct
built, `[bytes]`, or a field read; one through a `&mut` was refused, and
two `&`s of one value weren't `ptr::eq`.

## Decision

**A `#[repr(transparent)]` struct of the crate's or a dependency's is its
field in JS**, as a `ManuallyDrop` is (`recognition::transparent_field`,
the one place that answers it): made, `Callback(f)`, it's `f`; read,
`callback.0`, it's `callback`; its other fields, of no size, a
`PhantomData`, are nothing. Whether a `&mut` to it is a cell is its
field's: a `&mut Uuid` of an array is the array.

**A `transmute` between it and its field is the value itself**, by value
or through either reference, so a `&mut`'s is no longer refused.

```js
const task = { id, callback, priorityLevel, .. };
const continuationCallback = callback(didUserCallbackTimeout);
```

A struct of std's that's `#[repr(transparent)]`, a `Cell` or a
`Wrapping`, keeps the JS value rust-js has for it.

## Why

- **It's what the attribute says**: the struct is laid out as its field,
  and in JS that's being it. A newtype that wants its own shape doesn't
  ask for this layout.
- **It reads as written**: upstream's is the function; a wrapper array no
  one wrote is noise at every call.
- **One answer**: whether a struct is its field is `transparent_field`'s,
  which making, reading, projecting, a constant's value and a `transmute`
  each ask.

## Alternatives

- **Every one-field tuple struct its field**: a newtype without the
  attribute is a type of its own in Rust's eyes too, and its JS, an array,
  is public, its declarations' `[T]`. Rejected.
- **A `#[rust_js::transparent]` attribute**: Rust's attribute already says
  it. Rejected.

## Consequences

- Its TypeScript declaration is its field's type.
- A `#[repr(transparent)]` struct's `impl`s take the field's JS value as
  `self`: uuid's `Uuid` is its array.

## What holds it

- The corpus's `transparent_transmute`: each `transmute` of ADR 0448's,
  a `&mut`'s written through, `ptr::eq`, a closure's newtype called, a
  struct beside a `PhantomData`, against native Rust, by value and as a
  library.
- Mutations: a transparent struct an array again, its marker taken as its
  field, a `&mut` to it a cell, a constant's an array, a `transmute`
  refused.

## Amendment: its constructor as a function

`adt_value` made a transparent struct its field, but its constructor
passed as a function, `.map(Self)`, went through `constructor_value`,
which built an object of the struct's shape, its field's: smallvec's
`NonNull::new(ptr).map(Self)` was `p ? { pointer: p } : undefined`, which
`as_ptr`, reading the struct as its field, gave `$ptrSlice` as a
pointer, and smallvec's probe threw (#122, bisected to this ADR's
commit). `constructor_value` makes it its field too: `.map(Meters)` gives
what it's given. Held by `test/corpus/transparent_constructor_value.rs`
and the mutation `transparent-constructor-value-shaped`.

## Amendment: its derived order

A derived `PartialOrd` or `Ord` of a transparent struct was refused,
"comparing `Uuid`s": `cmp_value`'s derived order reads a struct's fields,
and this one's value is its field. Its order is its field's now, as its
`==` was. uuid 1.28's `Uuid` is one, and the crate corpus's probe, which
compares two, was refused (#122). Held by `test/corpus/transparent_ord.rs`
and the mutation `transparent-order-refused`.

## Amendment: its derived `<` is its field's

React's ReactEventPriorities compares `EventPriority`s, a transparent
`i32` whose order is derived, `a < b`, as upstream compares its numbers;
the port printed `$cmp(a, b) < 0`: `ordering_call` wrote JS's own `<`
only of a type JS's `<` orders, a number's, and the struct's order went
through `cmp_value`, which orders it as its field already. **A
`<`, `<=`, `>` or `>=` of a transparent struct whose order is derived is
its field's** (`ordered_as`): JS's `<` of a number, `bool` or numbered
enum, as a string's of a literal below U+D800. Case N: the struct is its
field in JS, and the derive compares it as its field's `partial_cmp`
does, an `f64`'s `NaN` false either way. A hand-written `PartialOrd` or
`Ord` is still called. Held by `test/lowering-numbers.test.ts`, "a
transparent struct's derived order is its number's <", `lane_bits.rs`
against native Rust, and the mutations `transparent-order-by-cmp` and
`transparent-order-of-its-own-impl-too` (`ordering.ts`).

## Amendment: its clone, of what's no object

A derived `Clone` of a transparent struct of a function, React's
`RendererTask(&'static dyn Fn(bool) -> Option<RendererTask>)`, was
refused, "cloning `RendererTask`", where the program writes one over
another where it's kept, `queue.borrow_mut()[i] = callback`: a type
written through a `&mut` is "changed in place" (ADR 0020), so its clone
was asked for, of a struct whose value is its field, which has no
fields to copy. What's written over is where the value is kept, its
array; the value, a function, isn't changed. **A transparent struct of
the program's whose field is no object is told apart from its clone as
its field is**, unless its own `clone` runs code (`needs_clone`): its
clone is the function, `let callback = queue[i];`. A container of one
written through a `&mut` is still copied, as `contains_mutated` says. Case
A: a function is the same function however it's copied. Held by
`test/corpus/refcell_clone_through_rc.rs`'s `Task` and the mutation
`transparent-of-no-object-cloned` (`std_impls.ts`).

## Amendment (2026-10-29, the user's decision): its `From` is the value it's given

React's reconciler keeps upstream's `opaque type EventPriority = Lane` as
`#[repr(transparent)] pub struct EventPriority(Lane)`, made of a lane by
`impl From<Lane> for EventPriority { fn from(lane) -> Self {
EventPriority(lane) } }`. React DOM's getCurrentUpdatePriority, upstream's
`return ReactDOMSharedInternals.p`, read
`eventPriorityFromI32_from(ReactDOMSharedInternals.p)` and wrote a copy
of the `from` that gives back its `lane` (gap rd62).

The user's decision: **a single-field newtype's construction, its `From`
and its field read print as the value itself.** Construction and field
read did already (above); now:

- **A `From` between a `#[repr(transparent)]` struct and its field whose
  body makes the struct of its parameter, `EventPriority(lane)`, or reads
  its field, `priority.0`, is the value it's given**, wherever it's
  called: `EventPriority::from(lane)`, `lane.into()`,
  `i32::from(priority)`, and `lane.map(EventPriority::from)`, which is
  `lane`. It's ADR 0264's rule, a function each call of which is what
  it's given (`patterns::gives_its_argument`, which `gives_own_name` was),
  asked of the impl a trait's method resolves to or the `From` an
  `into()` does (`callee`), so one nothing reads as a value is written
  nowhere.
- **A library's is too**, as its contract lists (`gives_argument`, by key,
  `Foreign::gives_its_argument`): a consumer has no body of it to read.
- Only a `From`, the conversion a newtype names: a crate's own function of
  the same body, React's `eventPriorityToLane(priority)`, is called, as
  upstream calls it. A `From` that does more, `Doubled(lane * 2)`, is
  called; a newtype without `#[repr(transparent)]` is an array of one, and
  its `From` makes one.

Case ([0262](0262-when-rust-and-js-disagree.md)): none, as the call gives
what it's given, and runs nothing else.

Consequences: a `From` taken as a value where it's not `map`'s,
`lane.map(EventPriority::from)` aside, is the function still, and is
written. `lane.map(EventPriority::from)` names it before `map` sees it's
the value, so its `from` is written though no call reads it (a gap).

Held by `test/corpus/newtype_conversions.rs` (run against Rust) and
`test/cross-crate-items.test.ts`'s "a library's transparent struct's From
is the value it's given", and the mutations `newtype-from-called`,
`newtype-from-of-anything`, `newtype-from-unresolved` and
`gives-argument-unlisted`.

