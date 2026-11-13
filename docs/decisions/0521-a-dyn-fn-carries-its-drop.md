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

## Amendment: a `dyn Fn` drops by what it takes (#954)

Whether a `dyn Fn` drops what it holds was one answer for the crate:
once a function holding a value with a destructor was made any `dyn
Fn`, every `dyn Fn` dropped. Beside such a neighbour, a generic struct
holding a `Box<dyn Fn(&K)>` in an `Rc` was refused as a std type holding a
value with a destructor (the neighbour shuffle, #954), and a `pub`
thread-local whose initializer calls a `dyn Fn` was refused as one that
may run what rust-js can't follow (#956). Now a `dyn Fn`
drops where it may take what one such is made takes (`FnDynDrops`), the
tuple of its arguments compared with `may_be`. A type parameter in it may
be any type, and its kind, `Fn`, `FnMut` or `FnOnce`, isn't asked, as one
upcasts to another. What it gives isn't asked either, which only counts
more as dropping. Every `dyn Fn` still drops where a library's contract
says its own may. The library contract's `makes_dropping_fn_dyns` is
whether any does. Two corpus cases lose their `$dispose`s of `dyn Fn`s
none of whose kind drops, which disposed of nothing:
`dyn_fn_generic_closure_drops` and `dyn_fn_of_a_borrowed_function`.
Corpus case `dyn_fn_drops_by_its_signature`, mutation `fn-dyn-drops-any`.
The readability baseline is blessed for it: the two cases' lines fewer,
each other shape's share of them grows a little, not one more of it.

## Amendment: a boxed function boxed again is another allocation (2026-11-10, #1221)

Three shapes made a `dyn Fn` of a function that was one already, and
dropped it wrongly (found by #1204's `dyn-fn` rewrite, five corpus cases):

- **Generic code's `boxed<F: Fn>(f: F) -> Box<dyn Fn>` given a `Box<dyn
  Fn>`** made it `$dropping(f, () => dropF?.(f))`, which set the drop on
  `f` in place of the one `f` carried, so `dropF`, `$dispose(f)`, called
  itself: a stack overflow. `$dropping` of a function that carries a drop
  already gives a function of its own that calls it, carrying the new
  drop, which drops the one it holds, as the box is another allocation
  holding the first (`src/runtime/dropping.js`). A function that carries
  none is given its drop in place, as before; no JS a compile writes
  changes.
- **A `Box<Box<dyn FnOnce>>` made a `dyn FnOnce`** was taken for a `Fn`
  made one, `$droppingOnce(f, () => $dispose(f))`, so the inner's drop ran
  after its call had consumed it: twice. Whether a function is called by
  `FnOnce` alone is asked of what its boxes hold.
- **Generic code's `once<F: FnOnce>(f: F) -> Box<dyn FnOnce>`** was taken
  for a `Fn` made one too, though its callers give it a function called by
  `FnOnce` alone, which drops what it holds as its body ends (ADR 0470): a
  `Fn`-kind closure given it dropped twice. A type parameter is asked its
  kind as a closure is, `function_kind`, its strongest bound's.

Results changed ([0262](0262-when-rust-and-js-disagree.md)), so Rust's are
kept. Corpus case `boxed_function_boxed_again`, both builds; mutations
`dropping-again-in-place`, `fn-once-dyn-of-a-box-unpeeled` and
`fn-once-dyn-of-a-parameter-unasked`.

## Amendment: a `Box<dyn Fn>` boxed again is held by its snapshot (#1109)

Since #1221, `$dropping(f, drop)` of a function that carries a drop
already wraps it in a function of its own, which carries the new drop, so
a `Box<dyn Fn>` boxed again and taken for one that doesn't carry its own
drop, `$dropping(f, () => { $dispose(f); })`, drops it once, as the
function itself, its drop the one it carries, does. Which of the two it is
is how the JS reads, not what it does: mutation
`boxed-fn-dyn-boxed-again-drops-itself` survived main's check, and is
held by `dyn_fn_of_a_borrowed_function.rs`'s snapshot now (`snapshots:
true`). ADR 0262's case: none, no JS changes.

## Amendment: a `Box<dyn Iterator>` carries its drop (2026-11-12, #1420)

A `dyn Iterator` is the JS iterator itself (ADR 0022), so, like a `dyn Fn`,
it had nothing to carry the drop of what it owns. Boxing a value with a
destructor as one was refused: "a `dyn` of a value with a destructor".
quickcheck 0.9's shrinkers are `Box<dyn Iterator<Item = A>>` of a type
parameter's `A`, `Box::new(iter::once(value))` and `VecShrinker`'s, so
nothing that uses quickcheck compiled. That included itertools' 284
quickcheck tests.

**An iterator boxed as a `dyn Iterator` carries its drop, `$droppingIterator(it,
drop)`, and a `Box<dyn Iterator>` is dropped by `$disposeIterator(it)`.** This
is the same form a `dyn Fn` has. The drop is the boxed value's own:
- a std iterator's is its closing, `it.return()`, which drops what it has
  left (ADR 0422);
- the crate's own iterator's is its drop glue, of the fields it has as it is
  stepped.

- **The drop is the iterator's own property** under `$disposeKey`, read with
  `Object.hasOwn`. Node's `Iterator.prototype` has a `Symbol.dispose` of its
  own, which calls `return()`. Read through the prototype, it would close an
  iterator Rust never drops, a generator given as one, or close one twice.
- **It's dropped once**, wherever that is first: by its `Box`'s drop, or
  by a consumer that took it by value, `count()`, `collect()` or a `for`
  that breaks, closing it as it ends, `return()`, as std drops the `Box`
  it was given. A drop that panics isn't run again.
- **One that carries a drop already**, a `Box<dyn Iterator>` that generic
  code boxes again, is another allocation holding it. A `$lent` of it carries
  the new drop, which disposes the one it holds.
- **A `Box<dyn Iterator>` drops only where one may own what drops**
  (`ClosureFacts::iterator_dyns_drop`), found as a `dyn Any`'s is:
  - from the crate's owning coercions, not a `&dyn Iterator`'s or a
    `&mut dyn Iterator`'s, of what drops or, where a type parameter's value
    may, of a type parameter's;
  - or from a library that boxes one so, as its contract's
    `makes_dropping_iterators` says.
  A crate with neither writes what it did.
- **An `Rc<dyn Iterator>` of what drops stays refused**: its drop runs where
  its last owner drops, which this doesn't follow.

Case C (ADR 0262): what's dropped, and when, as std's.
Held by:
- the corpus case `dyn_iterator_drops`, against native Rust in both builds:
  unstepped, part stepped, replaced, popped, after a `for` that breaks,
  counted, collected, a `Vec`'s `IntoIter`, generic code's own box of a type
  parameter's iterator, one boxed again, `iter::once` of a type parameter's
  value, and a drop that panics;
- `test/runtime-package.test.ts`: a generator given as one is never closed
  by its drop, and one that carries a drop is closed once;
- the corpus case `dyn_iterator_drop_panics`: a drop that panics as the
  `Box` drops;
- the mutations `iterator-dyn-drop-lost`, `iterator-dyn-drops-nothing`,
  `iterator-dyn-disposed-as-a-pair`, `iterator-dyn-carries-no-drop`,
  `iterator-dyns-drop-unfound`, `iterator-dyn-drop-inherited`,
  `iterator-dyn-reboxed-in-place` and `iterator-dyn-unclosed-by-its-consumer`.

## Amendment: an adapter of a boxed iterator closes it (2026-11-12, #1420)

quickcheck 0.9's `char` shrinker boxes
`(*self as u32).shrink().filter_map(char::from_u32)`, an adapter whose
source is a `Box<dyn Iterator>` that carries a drop. The adapter's drop
didn't know how to close its source, so boxing it was refused: "a `dyn`
of a value with a destructor". **A `Box<dyn Iterator>` is a source an
adapter closes** (`closing_source`). Closing the adapter, `return()`,
reaches the box's `$droppingIterator`, whose dispose runs the drop once,
as std drops the box.

Case C of ADR 0262. Crate pair `library_filter_map_of_a_boxed_iterator`
checks it against native Rust: dropped part stepped, taken, broken out of,
counted and collected. Mutation `boxed-dyn-iterator-source-unclosed` is
caught.

## Amendment: a `dyn Iterator` carrying a drop steps as Rust's (2026-11-13, #1690)

A JS iterator made a `dyn Iterator` of a value with a destructor carried
its drop on its own `return` (`$droppingIterator`), but kept JS's own
iterator helpers. By the JS spec, `map`, `filter`, `take`, `drop`,
`flatMap` and `reduce` close what they step as their function throws. That
ran the box's drop of what it still holds while the throw was still on
its way: before the fold's accumulator, which std's `fold` moves into each
step, and before a closure's capture. And a panic of one of those drops was
dropped by `IteratorClose` instead of aborting.

**It's now a `$Steps`** (`$stepsAround`), whose own adapters are std's:
they pass a panic on and leave what they step to the fold that consumes
it, which drops the accumulator, then closes it, as cleanup that aborts on
a second panic. A binding's JS iterator, which carries no drop, keeps JS's
helpers. Case (ADR 0262): results change (drop order, and an abort), so
Rust's are kept. The iterator tables' `boxed dyn` source (ADR 1136) passes
494 of its 502 listed case-builds now. The 8 left, `step_by.step_by` over
the box, want a `Vec`'s `into_iter()` made a `dyn Iterator` to keep its
slice steps, its `nth` dropping what it passes as one slice; they stay
under #1690. Corpus cases
`boxed_iterator_summed_drops_its_rest_after_the_total` and
`boxed_iterator_drop_panicking_as_it_unwinds_aborts`, each failing before;
mutation `dropping-iterator-js-helpers`, caught.

## Amendment: a box forwards only what `Box<dyn Iterator>` forwards, and a `Vec`'s keeps its slice steps (2026-11-14, #1690)

The box's `$Steps` (`$stepsAround`) owns, of what it holds, only what std's
`Box<I>` forwards for an unsized `I`, a `dyn`'s: `nth`, and of a
double-ended one `next_back` and `nth_back`, of an exact-size one `len`.
Not `last`, `count` or a fold, which only a sized `I`'s box forwards: a
`dyn`'s are std's defaults over `next`. Forwarding the inner's `last` too
broke 20 table cases, which shows the rule is needed.

A `Vec`'s iterator made a `Box<dyn Iterator>` of what drops is now
`$closing($iter(items), $dropEach(drop))`, the form a `Vec`'s iterator has
in a chain, whose `nth` drops what it passes as one slice. The box's drop
closes it. So `step_by` over the box, which skips by `nth`, drops a skipped
slice's rest past a panic before the fold's accumulator, as native does:
the last 8 of #1690's table rows. It applies only to what drops each of a
slice's items (`$dropEach`). The crate's own iterator boxed is as before.
Each box is now a new `$Steps`, so re-boxing needs no `$lent`. Corpus case
`boxed_vec_iterator_skips_as_one_slice`; mutations
`boxed-vec-iterator-unsliced` and `dyn-box-nth-not-forwarded`, caught;
`test/runtime-package.test.ts`' "an iterator carrying a Rust drop never
reaches JS's own iterator helpers" holds the rule in the runtime. #1690's
rows are all off `test/iterator-matrix-known.txt`.
