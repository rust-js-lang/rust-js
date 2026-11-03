# 0521. A `dyn Fn` carries its drop

Status: Accepted. Extends [0484](0484-a-returned-closure-carries-its-drop.md)
and [0098](0098-destructors.md). Amended: a type parameter's function is
one that may be, below; a borrowed function made a `dyn Fn` carries
no drop, below; and a capture holds a type parameter's value only where
it owns one, below (#497); and a type parameter's value drops only where a
caller gives it one whose drop runs, below (#580).

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

## Amendment: a `Fn` made a `dyn FnOnce` is consumed by its call (#409)

A closure rustc infers `Fn` or `FnMut`, made first and then a
`Box<dyn FnOnce()>`, `let f = move || { let n = &d; .. }; let task:
Box<dyn FnOnce()> = Box::new(f); task();`, carried its drop, which ran
only where the box was dropped uncalled: called, it never dropped what
it took, where std's `call_once` shim consumes the box and drops the
closure as the call returns. A closure made a `dyn FnOnce` where it's
written is inferred `FnOnce`, and its own body drops it, so it was
right. **A function made a `dyn FnOnce` that isn't an `FnOnce` closure,
nor a `dyn FnOnce` already, a `Fn`'s, an `FnMut`'s or a `dyn Fn`'s, is
`$droppingOnce(f, drop)`: its call runs `f`, then the drop, in a
`finally`, as the call returns or unwinds; dropped uncalled, the drop**
(`dyns.rs`, `recognition::is_fn_once_dyn`). A `dyn Fn`'s drop is its
`$dispose`, of one only where the program makes a `dyn Fn` of what has
something to drop (`fn_dyns_drop`, ADR 0626): else it's as it is,
`queueMicrotask(callback)`. Case C: Rust's drops, as it
runs them. Held by `test/corpus/fn_boxed_once_drops.rs` against native
Rust, `dyn_fn_once_drops_itself.rs` unchanged, and the mutations
`fn-made-dyn-fn-once-unconsumed`, `fn-once-closure-consumed-twice`,
`fn-once-dyn-consumed-again` (`lower/traits/dyns.ts`) and
`dropping-once-not-on-unwind` (`runtime/dropping.ts`).

## Amendment: a borrowed function made a `dyn Fn` carries no drop (rd495)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, Rust's drops, where
they were wrong.

A `dyn Fn` is the JS function itself, and its drop, `$dispose(f)`, runs
what that function carries. A borrowed function made one, `let shared:
Rc<dyn Fn(u32) -> u32 + '_> = Rc::new(&*greet);` of a `Box<dyn Fn>` whose
closure holds a `Noisy`, was `greet` itself: the `Rc`'s drop ran the drop
`greet` carries, printing `drop boxed` while `greet` was still held, and
again as `greet` was dropped. `Rc::new(&greet)`, of the `Box`, and
`Box::new(&count)`, of an `impl Fn` its maker gave back, did the same.
And a `Box<dyn Fn>` boxed again, `Box::new(greeter("twice"))` as a
`Box<dyn Fn>`, was given a drop of disposing itself, `$dropping(f, () => {
$dispose(f); })`, whose call never ended.

**A function made a `dyn Fn` that borrows one that may carry a drop, a
`dyn Fn`, a closure that carries its drop, a `Box` of one or a type
parameter's, is a function of its own that calls it, `$borrowed(f)`,
where the crate's `dyn Fn`s drop** (`fn_dyns_drop`): a reference drops
nothing. A counted `Rc`'s is in its cell, `{ value: $borrowed(greet),
strong: 1, weak: 0 }`. Where no `dyn Fn` drops, nothing runs a carried
drop, and the function is as it is, so an `Rc<dyn Fn>` of a bound
function is the function (rd231). **A `Box` of a function that carries
its drop is that function**, its drop the one it carries
(`dyns.rs`, `carries_own_drop`).

Found while asking whether React's Fizz server's `dyn Fn`s drop
(rd495): they do, a task's pings holding an `Rc<Request>` and a
`Weak<Task>` that are counted, as `Rc::new_cyclic` and `Weak::upgrade`
ask a `Task`'s count (ADR 0843), and a `Request` holds `Rc<Task>`s.

### What holds it

- `test/corpus/dyn_fn_of_a_borrowed_function.rs`, against native Rust:
  an `Rc` of a borrowed `dyn Fn`, cloned, and of a borrowed `Box<dyn Fn>`,
  a `Box<dyn FnOnce>` of one, a `Box<dyn Fn>` of a borrowed `impl Fn`, and
  a `Box<dyn Fn>` boxed again.
- Mutations `borrowed-fn-dyn-carries-its-drop`,
  `borrowed-box-fn-dyn-carries-its-drop`,
  `borrowed-closure-fn-dyn-carries-its-drop`,
  `borrowed-fn-dyn-in-counted-rc-unwrapped` and
  `boxed-fn-dyn-boxed-again-drops-itself` (`lower/traits/dyns.ts`), and
  `borrowed-is-the-function` (`runtime/dropping.ts`).

## Amendment: a capture holds a type parameter's value only where it owns one (#497)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: what's taken to drop drops nothing more than Rust's.

alien-signals' `createReactiveSystem<K>` keeps its callbacks in an
`Rc<System<K>>`, `struct System<K> { notify: Box<dyn Fn(&K)> }`, and
returns closures that capture it. A closure "captured a value of a type
parameter" where its capture's type named one at all, so once the crate
had any `Drop`, a guard restoring state in a `try`/`finally`, every `dyn
Fn` was taken to drop, `System<K>` to hold what drops, and its uncounted
`Rc` was refused: "a std type holding a value with a destructor,
`Rc<System<K>>`". A `K` that only a function's signature names, or a
reference, is never dropped by what holds it.

**A capture holds a type parameter's value where it owns one**
(`owns_a_param`, `analysis/closures.rs`): the parameter itself, or a
field, an item, a `Box`'s value or a `PhantomData`'s, by which std's own
types, an `Rc`, a `Vec`, a map, mark what they own, that does. A
reference, a raw pointer (`raw_pointee`'s, a `NonNull` too), a `fn` and a
`dyn` own none (a `dyn`'s own coercion answers for it, as before); an
associated type, or anything else, may.

### What holds it

- `test/corpus/rc_of_functions_of_a_param.rs`, against native Rust: the
  generic `Rc<System<K>>` beside a `Drop`, which compiled to nothing it
  drops.
- `test/corpus/dyn_fn_generic_owned_drops.rs`: a generic closure holding
  a `Vec<K>`, made a `dyn Fn` by generic code alone, given a `Noisy`,
  drops it where Rust does.
- Mutations `param-owned-through-a-fn-dyn`,
  `param-owned-through-a-raw-pointer` and
  `param-unowned-through-phantom-data` (`lower/analysis/closures.ts`).

## Amendment: a type parameter's value drops where a caller gives it one whose drop runs (#580)

alien-signals' generic `computed<T>` (design B4) boxes a closure of its
getter, `Box::new(move |previous: Value| js::any_of(Rc::new(getter(..))))`,
and the crate has a guard's `Drop`. A function holding a type parameter's
value, the closure or `F` itself, was taken to drop wherever the crate had
any destructor (`params_may_drop`), and `F` itself wherever any closure of
the crate's held one: so every `dyn Fn` dropped, every `Rc` holding one
was counted, `Rc<T>` too (ADR 0852), and `js::any_of` of a counted `Rc`
was refused (ADR 0844). Nothing gives `computed`'s getter a destructor:
JS calls it.

**A function made a `dyn Fn` that owns a type parameter's value drops it
where a caller gives that parameter one whose drop runs**, `drop_params`'
answer (ADR 0098, the one owner of what generic code is given to drop),
which `analysis.rs` keeps beside the counted `Rc`s it settles with
(`counted_with`) and gives `dropping_fn_dyn` as `given`. `owns_a_param`
asks it of each type parameter it finds, and a `Box::new(f)` of an `F`
is a function that owns one, as a closure holding it is. The crate-wide
guess, that any closure of the crate's may be what `F` is given, is gone:
`drop_params` sees each call's type arguments, a closure holding a
`Guard` among them, through the crate's generic functions and trait
impls, and what a library's function the crate reaches makes
(`foreign_drops`) may hold anything, as before. A library keeps the
guess (`given` is `None`): its callers are its consumers, which it can't
see, so a type parameter's function may be any closure it makes, and one
holding a type parameter's value drops wherever it has a destructor. Its
JS and its contract's `makes_dropping_fn_dyns` are as they were.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, what's dropped, and
when, is Rust's: a `dyn Fn` given a closure holding a `Guard` drops it as
the last `Rc` of what holds it is dropped.

### What holds it

- `test/corpus/dyn_fn_of_a_params_function.rs`, against native Rust: a
  closure holding `computed`'s `F`, made a `dyn Fn` beside a `Guard`,
  that no caller gives a destructor, is the node itself, no `Rc` counted;
  as a library, which keeps the guess, it's counted (`library-differs`).
- `test/corpus/dyn_fn_of_a_params_function_drops.rs`, against native
  Rust: one caller gives it a closure holding a `Guard`, dropped as the
  last `Rc` of its node is.
- `test/lowering-drop-guards.test.ts`, "a dyn Fn of a closure holding a
  type parameter's function drops what its callers give it": #580's
  `computed<T>`, `js::any_of` of its `Rc<T>`, compiled and run.
- Mutations `fn-dyn-param-given-unasked`, `fn-dyn-closure-drops-unseen`,
  `fn-dyn-generic-unseen` (`lower/analysis/closures.ts`) and
  `fn-dyn-library-params-ungiven` (`lower/analysis.ts`).
