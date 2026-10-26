# 0521. A `dyn Fn` carries its drop

Status: Accepted. Extends [0484](0484-a-returned-closure-carries-its-drop.md)
and [0098](0098-destructors.md). Amended: a type parameter's function is
one that may be, below.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

A `dyn Fn` is the JS function itself (ADR 0022), with no dictionary: unlike
a `dyn T`'s pair, nothing carried its drop. A function holding a value with
a destructor made one, `Box<dyn Fn(&str)>` of a closure holding a `Guard`,
was refused where `Box::new` kept it ("a function that holds a value with a
destructor, kept by what it's given to"), and a type parameter's was
dropped as a function that drops nothing. Rust drops what it holds as the
`Box` is dropped: a scope's end, a `Vec`'s `pop`, a map's replaced value.
`subscriptions`, a handler `Box<dyn Fn>` holding an `Rc`, was one of the
four gaps the [counted-`Rc`s-across-crates
draft](0523-libraries-share-one-counted-rc.md)'s trial found.

## Decision

**A function made a `dyn Fn` carries its drop, `$dropping(f, drop)` (ADR
0484), and a `dyn Fn` is dropped by `$dispose(f)`.**

- **A closure made one carries it from where it's made**, as one its
  maker gives back does: `ClosureFacts::carries_its_drop` is the one answer for
  both, a crate fact of the closures the crate makes `dyn Fn`s of and the
  types `impl Fn`s hide.
- **Any other function holding a value with a destructor, a type
  parameter's, carries it as it's made one**: `$dropping(f, () =>
  dropF?.(f))` at the coercion.
- **`$dispose(f)` runs what `f` carries, if anything**: a `dyn Fn` of a
  plain function carries nothing.
- **A `dyn Fn` drops only where the crate makes one of a function holding
  a value with a destructor** (`ClosureFacts::drops`), found from the coercions:
  of a type parameter's, where a closure the crate makes holds one, or a
  library it uses may. A library's may always be: its callers' closures
  are unseen. A captured `Rc` counted can make one drop, and a `dyn Fn`
  that drops can count an `Rc`; both only grow, so a second pass settles
  them.
- **A function `Box::new` keeps drops with the `Box`** where its drop runs
  wherever it's dropped: a type parameter's, a `dyn`'s, or one that carries
  it. What a lazy adapter keeps stays refused.

## Why

- **It's the same program**: `dyn_fn_drops` drops `Box<dyn Fn>`s at a
  scope's end, by `pop`, `remove`, a map's replaced value and the program's
  end, and `dyn_fn_generic_drops` ones made by generic code, against native
  Rust.
- **Nothing changes where nothing drops**: a crate that makes no `dyn Fn` of
  a function holding a value with a destructor writes what it did;
  `subscriptions` keeps its output.

## Consequences

- `$dispose` tolerates a function that carries nothing.
- `recognition::carries_its_drop` is `given_back`, the half `ClosureFacts` asks.
- Mutations stop finding a `dyn Fn` that drops, a closure that carries its
  drop, the coercion's `$dropping`, the `dyn Fn`'s `$dispose`, and refuse
  what `Box::new` keeps.

## Amendment: a function handed to JS (ADR 0873)

A closure boxed where it's made and made a `dyn Fn` straight into a
binding's argument, which gives no function back, nothing after it
before the call unwinding, carries no drop: JS keeps it, and nothing in
Rust drops it again. And a closure that carries its drop, called by
`FnOnce`, drops what it took in its own body, where it was `$dispose` of
a function its body has no name for.

## Amendment: what a library hands a crate (ADR 0626)

A crate using a library takes it to hand what the items of its the
crate uses reach, its `impl Fn`, a `dyn Fn` or an `Rc` in what it exports,
not anything: a library of a `bool` static hands none.

## Amendment: a type parameter's function is one that may be (ADR 0626, amended)

A function made a `dyn Fn` is taken to be a type parameter's, which may
be any closure the crate makes, only where it may be: a type parameter's
own, `F`, or a closure that captures a value of one. A closure of a
generic function's that captures none, React's `chain_thenable_value<T>`'s
`move |resolve| listeners.borrow_mut().push(resolve)`, drops what it
captures, which the drop query answers; a `dyn Fn` made another is a
function already, whose own coercion answers. Held by the corpus's
`dyn_fn_generic_closure_drops` against native Rust, and ADR 0626's
amendment's tests and mutations.
