# 0864. A type equal only to itself keys a JS `Set` and `Map` by itself

Status: Accepted. Amended: `==` of two values of one is `===` too, and
its `PartialEq`'s dictionary is written only where it's reached; and a
library's contract lists its, which its consumers key by themselves;
and an `Rc` of one is equal only to itself too; and a `PartialEq` of
`ptr::eq` alone is `===`, whatever its `Hash`; and a library's contract
lists those too. Amended by
[1015](1015-what-holds-an-identity-is-never-eq-of-its-contents.md):
what holds one compares it by `===`, never by `$eq`.
Extends [0463](0463-hashing-is-stds.md) (`ptr::hash`
writes its pointer's address) and [0582](0582-ptr-eq-of-an-object-is-its-identity.md)
(`ptr::eq` of objects is `===`) to a set's and a map's keys, and
[0562](0562-a-pointer-is-a-place-and-an-address.md)'s address to a hash.

Case ([0262](0262-when-rust-and-js-disagree.md)): None, by the program's
own `Eq` and `Hash`: two of these are equal exactly when they're one
value, which is what JS's `Set` and `Map` ask of an object key, so the
set holds the keys Rust's does and finds each where Rust's does. Only
the order a `HashSet` iterates in differs, which is unspecified in Rust
and insertion order here, as of every `HashSet` (ADR 0059).

## Context

React's `Transition` keeps the fibers it updated, each by itself:

```js
currentTransition._updatedFibers = new Set();
..
if (updatedFibersCount > 10) { .. }
currentTransition._updatedFibers.clear();
```

A fiber is the reconciler's JS object, which the port's `react` package
names by a binding type, `Fiber(PhantomData<JsObject>)`. Rust's way to
key a set by identity is the type's own `Eq` and `Hash`, of its address:

```rust
impl PartialEq for Fiber {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Fiber {}

impl Hash for Fiber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}
```

rust-js refused it: a `HashSet<&'static Fiber>` was "values of type
`&'static Fiber`", a key JS's `Set` can't find by its own `eq`, and
`std::ptr::hash` was unknown.

## Decision

**A struct of the crate's whose `PartialEq::eq` is `ptr::eq(self, other)`
alone, whose `Hash::hash` is `ptr::hash(self, state)` alone, and whose
`ne` is std's, is equal only to itself: a set or map of it, or of
references to it, is a JS `Set` or `Map` keyed by each object, and `==`
of two is `===`.**

```rust
pub struct Transition {
    #[cfg_attr(rust_js, rust_js::name = "_updatedFibers")]
    pub updated_fibers: Option<RefCell<HashSet<&'static Fiber>>>,
}
..
current_transition.updated_fibers = Some(RefCell::new(HashSet::new()));
```

```js
currentTransition._updatedFibers = new Set();
```

- **This is the one way** to write a set or map of JS objects by
  identity: Rust's own, a type whose `Eq` and `Hash` say so. A struct
  that derives them compares by value, a `$KeyMap` (ADR 0121); one whose
  `eq` does anything else, or compares other values than its own two, is
  refused as a key JS's `Set` can't find by it.
- **`ptr::hash(p, state)` is what std's writes**, the pointer's address
  as a `*const T`'s `hash` writes it (ADR 0562's address), wherever it's
  called.
- **A copy of a `&&Fiber`, `iter().copied()`, is the reference**: the
  same key, not a copy of the fiber. `copied()` and `cloned()` of an
  iterator of references to references clone the reference, as std's do.

## Why

- **It's React's JS**, `new Set()`, `.size`, `.clear()`, and the Rust an
  engineer writes for a set of things by identity.
- **It's exactly JS's equality of an object**: no case where the `Set`
  finds a different key than Rust's would.
- **No new API**: the program's own impls say it, so no rust-js type or
  attribute is the identity set.

## Alternatives

- **A `js::Set` binding of objects**: JS's type in Rust's place, where
  std's `HashSet` says the same and native Rust runs it.
- **A wrapper key, `ByAddress<&Fiber>`**: a crate's type for what the
  type's own impls say, and a `$KeyMap` of wrappers in JS.
- **Recognize `ptr::eq` of any two values in `eq`**: an `eq` that compares
  another value's address isn't identity, which a JS `Set` would find
  wrongly.

## What holds it

- Corpus `identity_keys` (a set and a map of them, `entry`, a hash of
  one, `copied()` and `cloned()` of references to them) and
  `refcell_identity_set`.
- `test/lowering-collections.test.ts`, "a type equal only to itself keys
  a JS Set by itself": `new Set(fibers)`, and refusals of one with its
  own `ne`, one that compares `self` with `self`, and one that compares
  more.
- Mutations `identity-key-refused`, `identity-eq-unseen`,
  `ne-override-trusted`, `forwards-args-unchecked`, `ptr-hash-unrecognized`,
  `ptr-hash-of-pointee`, `copied-pointee-peeled`.

## Amendment: `==` of two is `===`, and its dictionary is written where it's read

Case ([0262](0262-when-rust-and-js-disagree.md)): none: `===` is what its
own `eq` does, and nothing reads a dictionary that isn't written.

React's `ReactStartTransition.js` began with what upstream hasn't:

```js
export const fiberPartialEq = { eq: fiberPartialEq_eq };

function fiberPartialEq_eq(fiber, other) {
  return fiber === other;
}
```

Its `Fiber`'s `PartialEq` is read by no code but a `HashSet`'s, a JS
`Set`. And `==` of two `Fiber`s was `===` only of references to them:
of two values, `a == b` resolved to the impl's own `eq` and called it,
`fiberPartialEq_eq(a, b)`, as the corpus's `identity_keys` printed.

- **`a == b` and `a != b` of two values of a type equal only to itself
  are `a === b` and `a !== b`**, as of references to them: a trait call
  of its `PartialEq`'s `eq` is `eq_values`', never resolved to the impl.
- **Its `PartialEq`'s impl is written only where it's reached**, as a
  `Hash`'s and a marker trait's are (ADRs 0307, 0463): where generic code
  is given it, `equal(a, b, fiberPartialEq)`, it's written, and its `eq`
  where a function value names it. The pipeline asks `std_impls.rs`'s
  `equal_only_to_itself`, the one answer, of each impl.
- **A library writes it for its consumers still**, as every impl it has
  (ADR 0100): another crate doesn't know the type is equal only to
  itself, whose `eq` and `hash` bodies it can't see, so its `==` calls
  `fiberPartialEq_eq` and its generic code is given `fiberPartialEq`; its
  `HashSet<&dep::Fiber>` is refused, "values of type `&dep::Fiber`". A
  manifest fact of it would let a consumer write `===` and a `Set`, and
  the library leave both out; until then, a library keeps them.

### What holds it

- `test/lowering-collections.test.ts`, "a type equal only to itself keys
  a JS Set by itself": `a === b` of references and `a !== b` of values,
  no `fiberPartialEq` written, and one written where generic code is
  given it; corpus `identity_keys` and `refcell_identity_set`.
- Mutations `identity-eq-impl-called` (`scripts/mutations/lower/traits/calls.ts`)
  and `identity-eq-written-unreached` (`lower/pipeline.ts`).

## Amendment: a library's contract lists its types equal only to themselves

Case ([0262](0262-when-rust-and-js-disagree.md)): None, as above: the
library's own `Eq` and `Hash` say so, and its contract only carries what
it found.

React's port moved the reconciler's `Fiber` into its own crate, and the
reconciler's functions read `react`'s `Transition`, whose
`_updatedFibers` is a `HashSet<&'static Fiber>` of `react`'s `Fiber`.
Each was refused, "values of type `&'static react_start_transition::Fiber`":
the consumer can't read the library's `eq` and `hash` bodies, which
`equal_only_to_itself` asked of the crate's own types alone. And a borrow
of the set held while it adds one was taken to run the library's code,
its `Hash`, which a library borrowing the set unchecked refused.

**A library's contract lists its types equal only to themselves,
`identities`, by their keys, of those its consumers can reach; a
consumer's `equal_only_to_itself` of a library's type is its contract's
answer.** So a consumer's set or map of one is a JS `Set` or `Map`, `==`
of two is `===`, and the `Eq` and `Hash` std's code runs of one, by its
address, run nothing:

```rust
fn schedule(transition: &Transition, fiber: &'static Fiber) {
    if let Some(fibers) = &transition.updated_fibers {
        fibers.borrow_mut().insert(fiber);
    }
}
```

```js
function schedule(transition, fiber) {
  if (transition.updatedFibers) {
    transition.updatedFibers.add(fiber);
  }
}
```

- **What std's code runs of a library's type by a trait it has no impl
  of, its own or a blanket one outside std's, is nothing**
  (`cell_borrows.rs`'s `bound_runs`), as of the crate's own: a `Fiber`'s
  missing `Debug`, which a `RefCell`'s own impls ask of what it holds,
  isn't the library's code. What it does implement may still run any.
- **A library still writes its `PartialEq` and `Hash` dictionaries**,
  `fiberPartialEq` and `fiberHash`: a consumer's generic code given one,
  `T: PartialEq` of a `Fiber`, imports it, as it does any impl's (ADR
  0100). A consumer writing its own dictionary of one would let the
  library leave them out; that's a decision of its own.

`test/crates/pairs/identity_keys` runs a consumer adding a library's
fibers to the library's `Transition` and to its own `HashSet`, and
comparing two, against native Rust; "a library's type equal only to
itself keys its consumer's Set by itself" holds its JS. Mutations
`library-identity-unread` (`lower/std_impls.ts`),
`library-identities-unlisted` (`lower/library.ts`),
`library-identity-runs-code` and `library-impl-always-there`
(`lower/analysis/cell_borrows.ts`).

## Amendment: an `Rc` of one is equal only to itself

Case ([0262](0262-when-rust-and-js-disagree.md)): None. Std's `Rc<T>`
compares and hashes what it points at, `T`'s `eq` and `hash`, here its
address's; each `Rc` of one allocation is one JS value, so `===` and a
`Set` tell them apart as Rust does.

React's `FiberRoot` keeps the fibers that scheduled updates,
`memoizedUpdaters`, a `Set` upstream, which the port holds as a
`RefCell<HashSet<Rc<Fiber>>>`, the `Rc`s the tree owns its fibers by.
Every function taking a `FiberRoot` was refused, "values of type
`Rc<Fiber>`": an `Rc` wasn't a type equal only to itself, and wasn't a
key JS finds by its `===`.

**An `Rc` or `Arc` of a type equal only to itself is equal only to
itself** (`equal_only_to_itself`): its JS value is what it points at,
an uncounted one's (ADR 0844), or the `{ value, strong, weak }` every
clone of a counted one shares (ADR 0320), one allocation's either way.

```rust
fn add_updater(root: &FiberRoot, fiber: &Rc<Fiber>) {
    root.memoized_updaters.borrow_mut().insert(Rc::clone(fiber));
}
```

```js
function addUpdater(root, fiber) {
  root.memoizedUpdaters.add(fiber);
}
```

Corpus `rc_identity_set` (uncounted, a root's set) and
`rc_identity_set_counted` (counted: inserting one already there drops
the clone, as std's does) run it against native Rust; mutation
`rc-identity-unread` (`lower/std_impls.ts`).

## Amendment: `==` of a `PartialEq` of `ptr::eq` is `===`, whatever its `Hash`

React's update queue compares two updates, each an `Rc<Update>`, by
identity, upstream's `a !== b`. Its `Update` has the `PartialEq` this
decision names, `ptr::eq(self, other)`, and no `Hash`, as no set or map
keys it: so it wasn't equal only to itself, `a != b` of two
`Option<Rc<Update>>` was `!(a == null || b == null ? a == b :
updatePartialEq_eq(a, b))`, its dictionary was written, and, the queue's
module in an import cycle with one whose load runs code, refused (ADR
0592).

**`==` of a type whose `PartialEq` is `ptr::eq(self, other)` alone, its
`ne` std's, is `===`, whatever its `Hash`** (`std_impls::eq_by_address`):
`a == b` and `a != b` of two values, of references, of `Rc`s and of
`Option`s of them, `a != b` of two `Option<Rc<Update>>`, as of any
`Option` of what `===` compares; and its `PartialEq`'s dictionary is
written only where it's reached, as an identity key's is. Generic code
given it is given `===`, `{ eq: (a, b) => a === b }`. Its `Hash` matters
only to a set or a map, which still ask `equal_only_to_itself`, both
impls, to key a JS `Set` or `Map` by each object; and a library's
contract lists those alone.

Case: none ([0262](0262-when-rust-and-js-disagree.md)): `===` is what
its own `eq` does.

Held by `test/lowering-dictionaries.test.ts`, "an identity PartialEq is
===, and its dictionary no load cycle reaches", and the mutations
`identity-eq-value-called`, `identity-eq-option-called`,
`identity-eq-needs-hash`, `identity-eq-dictionary-always-written` and
`identity-eq-trait-call-resolved`. Corpus `unknown_option_equality`'s
`Value`, whose `PartialEq` is `ptr::eq`, no longer writes the dictionary
nothing read.

## Amendment: a library's contract lists its `PartialEq`s of `ptr::eq` alone

Case ([0262](0262-when-rust-and-js-disagree.md)): none: `===` is what
the library's own `eq` does, as in its crate.

React's update queue is the reconciler's, and its consumers compare two
`Rc<Update>`s. The amendment above made `==` of a `PartialEq` of
`ptr::eq` alone `===` in its own crate, but a library's contract listed
only its types equal only to themselves, both impls, so its consumer
asked its own `eq_by_address` of none: `Update`'s `eq`, which the
library doesn't export, was compiled again in the consumer (ADR 0914),
`function updatePartialEq_eq(update, other) { return update === other; }`,
and each `==` called it.

**A library's contract lists, `eq_by_address`, its types whose `==` is
their address's, whatever their `Hash`, its `identities` among them; a
consumer's `eq_by_address` of a library's type is its contract's
answer**, as its `equal_only_to_itself` is `identities`'. Each list is
one function's answer of the crate's own types (`pipeline/reached.rs`),
written by `library.rs` and read by `Foreign`. So a consumer's `==` of
two `Rc<Update>`s, or of what holds them, is `===`, and the impl isn't
compiled where nothing else reaches it.

`test/crates.test.ts`, "a consumer's == of what holds a library's type
equal only to itself is never $eq", holds the contract's lists and that
the consumer writes no `updatePartialEq_eq`; mutations
`library-eq-by-address-unlisted` (`lower/library.ts`) and
`library-eq-by-address-unread` (`lower/std_impls.ts`).
