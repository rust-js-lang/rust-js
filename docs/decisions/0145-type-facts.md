# 0145. A generic function is given the size and name of a type parameter it asks for

Status: Accepted. Extends [0049](0049-traits-and-generics.md), [0090](0090-wasm32-front-end.md)
and [0098](0098-destructors.md). Amended:
`mem::needs_drop` is a fact too (below); and a trait impl's method asks
its impl what it asks of the impl's type parameters: the impl's
dictionary is made of them, `wrapFootprint(TSize)`, one for each set,
keyed in a `Map`, and gives its methods each, which a direct call of one
is given too. A trait's default method and an impl's method of its own
type parameters ask none still (`test/corpus/impl_type_facts.rs`,
`impl_const_size.rs`). And a library's function or impl its consumers can
reach asks too: its manifest says what, `type_facts`, its consumers' calls
give it, and a consumer's generic function that passes its own type
parameter on asks it too, arrayvec's `extend_from_slice` of its public
`try_from` (`test/crates.test.ts`). A trait's method called of a type
naming the caller's type parameter is its impl's, and asks what that
asks: arrayvec's `from_iter` calling `array.extend(iter)` of an
`ArrayVec<T, CAP>` asks `T`'s size, and passes it on; so does one that
gives an impl's dictionary for a callee's bound, a `C: Push<T>` of a
`Stack<T>`, or calls a std method that only calls its bound's,
`collect()` that's `FromIterator::from_iter` (ADR 0160), arrayvec's
`Clone` (`test/corpus/impl_type_facts_through_trait.rs`). Which impl
gives the dictionary is `traits::evidence_impl`'s alone. And a fact of an
associated type of the crate's trait, `size_of::<A::Item>()` of smallvec's
`A: Array`, asked directly or of a callee given it, is its trait's
dictionary's, `AArray.ItemSize`: each impl's dictionary carries it of its
type for it, `T`'s of `type Item = T`, which its factory is given
(`test/corpus/associated_type_facts.rs`). A type's layout fact is
`type_fact_value`'s alone, `size_of_val` of a value's bytes apart.

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`size_of::<u32>()` is `4`, rustc's answer for the wasm32 target rust-js
checks programs for (ADR 0090), and `type_name::<u32>()` is `"u32"`. Of a
type parameter, they were errors: a generic function is one JS function
for every type, so its `T` has no one answer.

```rust
fn describe<T>(_: &T) -> String {
    format!("{} {}", type_name::<T>(), size_of::<T>())
}
```

Each caller knows its type, as each caller of a function with a destructor
to run knows its drop (ADR 0098).

## Decision

**A generic function that asks `size_of`, `align_of` or `type_name` of a
type parameter takes the answer as a parameter, after its dictionaries,
and each caller gives it for its type:** (Amended by ADR 0331: and its
`TypeId`, `TId`, unless a `T: Any` bound gives its dictionary.)

```js
function describe(_, TSize, TName) {
  return `${TName} ${TSize}`;
}

describe(1n, 8, "u64");
```

- **Asked directly, in a closure, or through a function it calls:** a
  generic function of the crate's that passes its own `U` on as a callee's
  `T` asks what the callee asks, `relay(u, UClone, USize, UName)` giving
  `describe(u, USize, UName)`. The analysis runs backwards from each ask,
  as `drop_params` runs forwards from each value with a destructor.
- **A fact is one value:** `size_of` and `size_of_val` of a sized type ask
  its size, `align_of` its alignment, `type_name` and `type_name_of_val` its
  name, each once, in that order, after the dictionaries and before any
  drop function.
- **A caller's concrete type answers it,** as rustc computes it for
  wasm32; a caller's own type parameter passes on what it was given.
- **`size_of::<T>` as a value is a call's arrow**, `let size =
  size_of::<T>` `const size = () => TSize`, and of a concrete type, `() =>
  2`. (Amended: a value of it was its own case, of a concrete type only.)
- **Still errors:**
  - through a trait's dictionary, a trait method's or an impl's, whose
    callers can't see what it asks, and a `Drop` impl's;
  - of an associated type, `T::Native`, or of a type holding a parameter,
    `type_name::<Cow<'a, T>>()`;
  - `size_of_val` of an unsized `T`, which is the value's size;
  - of a library's function its consumers reach (ADR 0100), which never
    see what it asks.

## Why

- **It's exact:** the value native Rust's monomorphized code has, for the
  wasm32 target, compared with native Rust by the `type_facts` corpus case
  on types whose layout is the same on both.
- **It's the JS a person writes:** a number or a string passed in, and a
  function that asks nothing takes nothing.

## Alternatives

- **One object per type parameter, `{ size, align, name }`,** given to
  every generic function. Simpler to pass on, but every generic function
  would take one, most for nothing.
- **A dictionary entry,** as a bound's. A size has no bound to hang on:
  every `T` is `Sized`.

## Consequences

- 5 rustc tests pass that stopped at a size or a name of a type parameter.

## Amendment: `needs_drop`

`mem::needs_drop::<T>()` (2026-10-15) is a fact of `T` too, `TNeedsDrop`
where it's a type parameter's: rustc's answer, `true` or `false`, which is
what the program sees, as native Rust's. It was refused. The facts are one
list, `TypeFact::ALL`, each named by its `word`, which the analysis that
passes them and the parameters that take them both ask. The corpus's
`needs_drop.rs`, and rustc's `union/union-manuallydrop-rpass.rs`, pass.

## Amendment: a name of an unsized type parameter, and of a type that holds one

`type_name::<T>()` of a `T: ?Sized` asked nothing of its caller, as a size
of one isn't one size; a name is, so it's asked, and given, as of any
(a size or an alignment, of an unsized one, still isn't). And of a type
that holds a type parameter, `type_name::<Cow<'_, T>>()`, it's rustc's name
with the one its caller gives of each parameter in its place:

```js
function names(TClone, TName) {
  return [`alloc::borrow::Cow<'_, ${TName}>`, `alloc::vec::Vec<(${TName}, u8)>`];
}
```

rustc names a parameter `_`, so each is put in as a type no program names,
`[!; 4000000000]` and up, by its index, and its given name put where that
is. **A type's fact of a `dyn` given to generic code is refused:** over a
`T: ?Sized`, it resolves a blanket impl's method where rustc calls the
`dyn`'s own, its value's type's, rustc's `dont-propagate-generic-instance`.
The `type_names_generic` corpus case names types holding a parameter, an
unsized one, one passed on, and a blanket impl's, directly and through a
`dyn`, with native Rust; rustc's `type-name-basic` passes.

## Amendment: an impl's facts reach a caller one way

**What an impl asks of its type parameters reaches a caller one way:
through the dictionary the caller gives for a bound**, a trait method's
`Self: Trait` among them, `stack.extend(iter)`'s `Stack<T>: Extend`, and
`collect()`'s `B: FromIterator`. The analysis had two more ways to the same
facts, resolving a trait's method call to its impl's and `collect()` to
`from_iter`, which since `Extend` and `FromIterator` are given as
dictionaries (ADR 0445) found nothing the bound didn't: an impl's method's
own type parameters ask none (above). They're gone, and the mutations that
held them, which survived.
