# 1006. A `pub` thread-local is made as its module loads

Status: Accepted. The user's decision (2026-10-28, React port gap rd51).
Amended (rd542): a `dyn Fn` its call is given is what that call gave it,
and a clone of a closure runs none of it. Amended (rd621): `may_run` is
its own rule, and a drop runs the destructors its fields and `dyn`s hold.
Amended (#554, the user's decision, 2026-11-04): one whose making as its
module loads would show is made where it's first used, a `pub` one refused.
Amended (#616): what a `with` lends is told apart by its struct's fields.
Amends [0037](0037-thread-locals.md), whose amendment refusing a `pub`
thread-local made where it's first used it replaces.

Case ([0262](0262-when-rust-and-js-disagree.md)): B, an accepted
difference the user decided: what a `pub` thread-local's initializer does
is done as its module loads, where Rust does it at its first use. D,
refused, where its module's load can't make it before what it reads.

## Context

React DOM's `DOMEventNames` exports its event names, each made by
`getVendorPrefixedEventName`, which reads and writes that module's state:

```js
export const ANIMATION_END = getVendorPrefixedEventName('animationend');
```

SimpleEventPlugin's JS, upstream's, imports `ANIMATION_END` and compares a
DOM event's type with it in a `switch`. The natural Rust is a `pub`
thread-local:

```rust
use crate::get_vendor_prefixed_event_name::get_vendor_prefixed_event_name;

thread_local! {
    pub static ANIMATION_END: DOMEventName = get_vendor_prefixed_event_name("animationend");
}
```

Its initializer runs the crate's code, so ADR 0037 made it where it's first
used, `export const ANIMATION_END = $local(() => ..)`: a JS module
importing it read a function where its `.d.ts` said `string`. The last
amendment of 0037 refused it, and the port kept a JS-path binding of its
own function.

## Decision

**A `pub` thread-local is made as its module loads, after the modules it
imports, as JS's `export const X = f()` is, whatever its initializer
runs**: `export const ANIMATION_END = getVendorPrefixedEventName("animationend");`,
read by its name by the crate's modules and JS's alike, as its `.d.ts`
says. `analysis/lazy_locals.rs` makes no `pub` one lazy.

- **What its initializer may run and read there is asked**
  (`lazy_locals::loaded_by`): the crate's bodies it calls, of each call's
  arguments; what std's calls run of what they're given, by their bounds
  (`CellBorrows::std_runs`, ADR 0328's owner); what a library's or a
  binding's call is given of the crate's, a closure, a function, or what a
  fn pointer among them may be (ADR 0907); each impl of the crate's a call
  through a trait may be; destructors a drop runs; each thread-local,
  `static` and `const` read by its name; and what the initializer of a
  thread-local read runs, which reading it may make there.
- **Refused where its module's load can't make it** (the pipeline's
  `load_cycles::refuse_exported_out_of_order`, once the modules' imports are
  known):
  - one that runs or reads code of a module in an import cycle with its
    own, which may not have loaded yet;
  - one whose initializer reads itself, through another thread-local or a
    function, which it would read before it's made;
  - one that may run what rust-js can't follow: a `dyn Fn`'s call, a
    fn pointer that may be any function, or what a type parameter's `Fn`
    is given.
  Each says to move what it runs out of the cycle, or keep it `pub(crate)`
  and export a `pub fn` that reads it.
- **Its own module's items it reads are made before it**: `js::in_load_order`
  places each `const` and thread-local after what it reads, through the
  functions it calls (ADR 0306), so `export const FIRST = firstLength();`
  follows the `const lengths` written after it in Rust.
- **A binding that runs nothing, `js::unknown_of(f)`'s cast, runs nothing of
  what it's given**, React's `export const jsx = __DEV__ ? .. : jsxProd`:
  making a value of a function doesn't run it.

## Why

- **It's upstream's JS**: React's modules export values made as they load,
  and their JS importers read them by name. A getter, or a `pub fn`, isn't
  what upstream's JS imports.
- **The difference is only when the initializer runs.** What it computes,
  and what it does, are the same; JS does them as its module loads, before
  any of the program's other code that imports it runs.
- **What would read what isn't made yet is refused**, never a
  `ReferenceError` at load.

## Alternatives

- **Keep the refusal** (0037's last amendment): correct, but the port
  couldn't write upstream's export, and kept a JS-path binding of its own
  crate's function, which AGENTS.md rules out.
- **Made lazily, read by a getter**: what JS imports would be a function or
  an accessor, not upstream's `export const`.
- **Made as its module loads only where nothing could tell**: the refusal
  again for React's case, whose initializer writes its module's state.

## Consequences

- **What a `pub` thread-local's initializer prints, panics with, and
  changes is done as its module loads**, before `main`'s first line and
  before what imports it runs, where Rust does it at its first use: a
  panic there is its module's load failing; a value it reads is what's been
  set by then; `LocalKey::set` before its first use finds it made already.
  Listed in ADR 0262's Consequences and the guide's differences
  (`semantics.md`, 30). A `pub(crate)` one is made where it's first used,
  as before.
- **More is refused than JS would mind**: a `dyn Fn` call whose value is
  the crate's own function of a module that has loaded is refused, as
  rust-js can't say so.

### What holds it

- Corpus `thread_local_exported_at_load`, against native Rust: two `pub`
  thread-locals of another module's function, which writes that module's
  state and makes a lazy thread-local of it, read in the order they're
  made, and what they did in that order; one of its own module's function
  that reads a thread-local written after it.
- `test/lowering-module-variables.test.ts`, "a pub thread-local is made as
  its module loads, which a JS importer reads": its `export const`, its
  `.d.ts`, a JS module of the program's reading it, and its initializer's
  print at the import.
- `test/diagnostics.test.ts`: one in an import cycle, through a closure
  given std; one reading itself through another thread-local; one of a
  `dyn Fn`'s call. `docs/guide/limitations.md`'s example.
- Mutations `exported-local-*` (`lazy_locals.rs`, `load_cycles.rs`).

## Amendment: a private one too, where its module's load can make it (rd120)

The user's decision (2026-10-29, React port gap rd120). A private
thread-local whose initializer ran the crate's code, printed, may have
panicked, or read what may change was made where it's first used,
`const isScrollEndEventSupported = $local(() => isEventSupported("scrollend"))`,
read `isScrollEndEventSupported()`, where React DOM's ScrollEndEventPlugin
writes `const isScrollEndEventSupported = isEventSupported('scrollend');`:
two forms of one item, by its visibility.

**Every thread-local is made as its module loads, whatever its initializer
runs, where its module's load can make it first**: `const x = f();`, read
by its name. `analysis/lazy_locals.rs` asks of each what it may run and
read there (`LazyLocals::at_load`): of one that runs none of the crate's
code (`Initializer::runs_the_crates`: no call of the crate's, a closure's,
a fn pointer's or a `dyn`'s, no print, which may run the crate's
`Display`, no std call given the crate's closure or function, no drop of
the crate's destructor), the thread-locals and `static`s it reads, and
what a thread-local among them reaches (a `const` it reads by its name is
read as its value where its module may load first, ADR 0031); else all `loaded_by` finds. What made one
lazy before but runs none of that, a check that may panic, a read of a
`static mut` or an atomic, no longer sends it to `loaded_by`, and "a
function of the crate's that only makes a value" (`Makers`) is gone: a call
of one is followed by `loaded_by`, whose answer is the same, as it calls
nothing else. A binding's call there is taken to run nothing of the crate's,
0037's rule, which React's `memo(Badge)` and the scheduler's
`port1.onmessage = ..` rely on; `loaded_by` follows what a binding is given.
Making one rule of the two is left to the user (asking `loaded_by` alone
makes those two `$local`). **Where its module's load can't make
it first, a private one is made where it's first used, as before, and not
refused**: the three tests above, now one predicate,
`load_cycles::out_of_order`, which a `pub` one's refusal asks too: it runs
what rust-js can't follow, it reads itself, or it runs or reads code of a
module in an import cycle with its own. The crate is lowered again with
those made where they're first used (`Lowered::MadeWhereUsed`), as one
whose maker was of a module in an import cycle was. A `pub` one keeps
this decision's rule; its refusals no longer advise `pub(crate)`, which
is made as its module loads now too.

This replaces 0037's amendments that made one where it's first used for
what its initializer runs or reads, #376's among them: one that reads
what may change after its module loads, a `Cell` thread-local, a `static
mut` or an atomic, is made as its module loads and reads what they hold
then. Its own module's items it reads are made first (`js::in_load_order`),
another module's, of no cycle, have loaded. A `$local` stays for those
made where they're first used.

Case ([0262](0262-when-rust-and-js-disagree.md)): B, this decision's
accepted difference, the user's, extended to private ones: what the
initializer prints, panics with, reads and changes is done as its module
loads, where Rust does it at its first use, or never, of one never used or
`set` first. Where nothing can tell, none.

- Corpus cases that showed the difference read what they make first, in
  the order their modules make them, so they still run as native Rust
  does: `thread_local_exported_at_load` (`prefixed` reads `VENDOR` before
  it records its name), `thread_local_exported_made`,
  `thread_local_made_by_constructor`, `thread_local_made_where_used`,
  `thread_local_reads_another_set`, and the crate pair
  `thread_local_reads_library`. Those of lazy ones, made of what `set`
  gives, a panic caught and made again at the next use, are of a module in
  an import cycle: `thread_local_set_before_use`, `thread_local_refcell_set`,
  `thread_local_made_where_used`'s `FLAKY`.
- `test/lowering-module-variables.test.ts`, "a private thread-local is
  made as its module loads, where its module's load can make it":
  ScrollEndEventPlugin's `const`, its print at the import, and one of an
  import cycle made where it's first used. `test/lowering-dictionaries.test.ts`:
  one of a dictionary, after it.
- Corpus `thread_local_runs_a_cycle`, against native Rust: one of each
  way an initializer runs a module in an import cycle with its own, a drop,
  a fn pointer, a print's `Display`, `Option::map` given its function, a
  call and a `static`'s read, each made where it's first used.
  `test/diagnostics.test.ts`: a `pub` one reading itself through another's
  initializer, and one reading one of a `dyn Fn`'s call.
- Mutations `at-load-*` and `out-of-order-*` (`lazy_locals.rs`,
  `load_cycles.rs`); 0037's that held what made one lazy, gone with it.

## Amendment: one rule of what a binding's call runs there (rd120)

The user's decision (2026-10-29). The amendment above left two rules of
what an initializer runs as its module loads: one that ran none of the
crate's code took a binding's call to run nothing, whatever it was given
(`Initializer::runs_the_crates`, 0037's assumption); any other was
followed by `loaded_by`, which follows what a binding is given. Two
answers to one question, which disagreed of a binding given a closure.

**What an initializer runs and reads as its module loads is
`loaded_by`'s alone** (`analysis/lazy_locals.rs`): `runs_the_crates`,
its `Initializer` and its helpers are gone. A binding's call runs what
it's given, a closure, a function or a `dyn Fn`, which `loaded_by`
follows or, of a `dyn Fn`, can't, but where it runs nothing,
`cell_borrows::call_runs_nothing`, the one answer a borrow held across a
call asks too: a binding marked `#[rust_js::runs_nothing]` (ADR 0867), and
a JS object type's `Deref`, which is the object. So a binding that makes a
value of a function, run later, says so:

- webapi's event handler setters, generated so, `port1.onmessage = ..`,
  which keep the function they're given;
- React's `memo`, `lazy` and `forwardRef`, and Next.js's `dynamic`, its
  options' and `noSSR`, which make a component rendered later:
  `memo(Badge, Some(Box::new(|a, b| ..)))` in `test/components.rs` is made
  as its module loads, `const LooseBadge = memo(Badge, (a, b) => ..)`;
- the js crate's `function::bind`, which makes a function called later:
  React's `callDestroyInDEV`, a bound fn pointer.

A `const` the initializer reads itself is read as its value where its
module may load after (`load_cycles::load_cycles`, ADR 0031, amended), so
it isn't what the initializer reaches, as of one that ran none of the
crate's code before; one a body it calls or a closure it makes reads by
its name is, as before. So `thread_local_made_where_used`'s `FROM_LIMITS`
and `FLAKY`, which read `limits::LIMIT` and were made where they're first
used for it, call `limits::limit_plus_one()` now, a function of the
module in an import cycle with theirs, to stay so, as native Rust's
output of them needs.

Case ([0262](0262-when-rust-and-js-disagree.md)): as the amendment above,
B: each made as its module loads where it was.

Held by `test/react.test.ts`'s components and APIs (`memo`, `lazy`), the
scheduler's port and `test/lowering-names.test.ts`'s `onmessage`,
`test/lowering-module-variables.test.ts`'s `callDestroyInDEV`,
`test/js-builtins.test.ts`'s `WeakMap` of a `Map`'s `Deref`,
`docs/guide/react/nextjs.md`'s `dynamic`, corpus `load_cycle_values`;
the mutations `at-load-binding-runs-nothing-followed` and
`js-object-deref-runs-code`, `at-load-own-const-reached` and `at-load-key-read-as-const`. The fast path's mutations, `at-load-*` of
what `runs_the_crates` asked, are gone with it.

## Amendment: every binding that keeps what it's given says so

The amendment above marked `memo`, `lazy`, `forwardRef`, `dynamic` and
`noSSR`, but not the other bindings that keep what they're given and never
call it, so `pub static Shown: ComponentValue<..> =
next::error::catch_error(..)` in `test/next.test.ts`'s app was refused, its
initializer taken to run its fallback as its module loads. Each binding
that only keeps what it's given is marked `#[rust_js::runs_nothing]` now:

- Next.js's `catchError` and `withRouter`, which make a component of a
  fallback or a component, rendered later, as `memo` does;
- React's `createContext`, which keeps its default, a function too, never
  calling nor reading it, and `cache`, which makes a function of one,
  called later;
- the js crate's `finalization_registry::new`, which keeps its `cleanup`
  for JS to call as targets are collected, so React's ReactTaint's
  registry is made as its module loads, `const finalizationRegistry =
  typeof FinalizationRegistry !== "undefined" ? new
  FinalizationRegistry(cleanup) : undefined`, not `$local(..)`.

A binding that calls what it's given, `startTransition`, `act`,
`flushSync`, or a hook's, isn't: those run as they're called.

Case ([0262](0262-when-rust-and-js-disagree.md)): B, as above: each made
as its module loads, as React's and Next.js's own `export const` are.

Held by `test/next.test.ts`'s app (`catchError`, `withRouter`) and
`test/react.test.ts`'s "React's createContext and cache keep what they're
given, made at load", and `test/js-collections.test.ts`'s "whether the host
has a FinalizationRegistry is is_defined of its constructor".

## Amendment: a `dyn` named only as a type argument runs nothing (rd281)

React's componentFrameCache, written as upstream's, `let
possibly_weak_map = if js::is_function(js::class::<WeakMap<Key, String>>)
{ .. } else { js::class::<js::Map<Key, String>>() };
Some(possibly_weak_map.call_static(js::weak_map::new, ()))` of `type Key
= dyn Fn() -> Option<&'static Unknown>`, was made where it's first used,
`$local(() => ..)`, each read a call: `loaded_by` took the `dyn Fn` in
the type arguments of a binding's call, `js::class::<WeakMap<Key, ..>>`,
and of a function item given one, `js::weak_map::new::<Key, String>`, for
a `dyn Fn` the binding may call, which rust-js can't follow. Neither is a
value: a type argument names a type, and a function item holds none.
**What a call of a library's or a binding's is given is the values of
its inputs, not a function item's type arguments (`values_in`); of its
own type arguments, the closures and functions they name, and the drops
of their types, but no `dyn`, of which it's given no value.** The crate's
function given is followed of its arguments where it's called, as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: nothing of the crate's runs that didn't, and it's made as its
module loads, as each one that runs only what rust-js follows is.

Held by `test/lowering-module-variables.test.ts`' "a thread-local that
names a dyn Fn only as a type argument is made as its module loads", and
mutations `type-argument-dyn-followed` and `fn-item-arguments-walked`
(`analysis/lazy_locals.ts`).

## Amendment: a `dyn Fn` its call is given is what that call gave it (rd542)

Case ([0262](0262-when-rust-and-js-disagree.md)): B and D, as above: what
it runs is followed, and refused where it can't be.

React's noop renderer exports what `createReactNoop(ReactFiberReconciler,
true)` makes as its module loads (rd293). Its Rust passes the reconciler's
factory as a value, as upstream does, since React Test Renderer and ART
pass their own:

```rust
thread_local! {
    #[cfg_attr(rust_js, rust_js::namespace)]
    pub static REACT_NOOP: ReactNoop = create_react_noop(&react_fiber_reconciler, true);
}

fn create_react_noop(reconciler: &dyn Fn(&JsObject) -> &'static ReactFiberReconciler, ..) -> ReactNoop {
    let noop_renderer = reconciler(host_config);
    ..
}
```

`loaded_by` took any `dyn Fn`'s call for what it can't follow, so the
initializer was refused. **A `dyn Fn` a body calls is followed to what
its call was given**: where the body's MIR says what it is, a closure or
a function coerced to it there, or a parameter its call gave one so, each
through copies and reborrows of a variable written once, each such
function is what the call runs, followed as a direct call of it is: a
body of the crate's, or a binding's call, here `react-reconciler#default`
of a module that has loaded (ADR 1006's one rule of what a binding runs).
A body reached from two calls giving it two such is followed once for each
of them. Any other `dyn Fn`, of a field, a collection or a closure's
capture, is still what rust-js can't follow.

The other way, a namespace thread-local (rd293) made as its module loads
whatever it runs, since only JS reads it, was not taken: its initializer
runs as its module loads either way, so what it runs may still be a
module of an import cycle that hasn't loaded; following it keeps that
guarantee.

Held by `test/lowering-module-variables.test.ts`'s "a pub thread-local's
dyn Fn is what its call gave it", against native Rust, and
`test/diagnostics.test.ts`'s refusals of a `dyn Fn` read from an array,
of one lent mutably, of one std calls, and of a closure given that reads
the thread-local;
mutations in `scripts/mutations/lower/analysis/lazy_locals.ts`. A function
made that nothing reads, a binding's value MIR holds in a variable of its
function's type whose `&` is read by its type, is dropped (`unused lets`),
so the initializer is `export const NAMED = make(String);`, not an IIFE.

**A clone of a closure runs none of it** (rd542, the porter's
createReactNoop): std's clone of a closure clones what it captured, so
it runs only those clones, each `Clone` of the crate's among the types of
what it captured, a derived one's too, and no function it holds. Its
`get_or_create_root_container.clone()`, boxed into what it returns, was
followed as a call of the closure, which calls the reconciler's
`createContainer` given a `dyn Fn`, so the initializer was refused. Held
by `test/lowering-constants.test.ts`'s "a pub thread-local whose
initializer clones a closure doesn't run it" and `diagnostics.test.ts`'s
"a pub thread-local whose closure's clone reads it"; mutations
`closure-clone-runs-it` and `closure-clone-runs-nothing`.

## Amendment: what a body may run is its own rule (rd621)

`loaded_by` is `analysis/may_run.rs`'s `may_run`, the one answer of what
a body of the crate's may run, which a function whose return alone owns
a callback asks too, of what it runs before it returns (ADR 0317,
amended). Its drop rule was a walk of a type's arguments for the crate's
destructors, which missed one of a field of the crate's struct, a
library's destructor calling a closure it holds, and whatever a `dyn`
holds. A drop now runs nothing where it runs no destructor
(`drops::drop_runs`) and holds no `dyn Fn`; else each destructor of the
crate's through its types' fields and what std's and a library's hold, a
library's destructor's closures, and, of a `dyn Fn`, what each closure
the crate makes one of only to hand it to JS holds, which `drop_runs`
leaves out; a `dyn` whose drop may run one is what rust-js can't follow. An initializer that drops such a `dyn` is made
where it's first used, a `pub` one refused, as one calling a `dyn Fn`
is. Mutation `may-run-field-destructor`.

## Amendment: made where it's first used where its module's load would show (#554)

The user's decision (2026-11-04, option (b) of #554). Every thread-local
was made as its module loaded unless its module's load couldn't make it
first, so one whose initializer read what the program set later was made
of what was there at the load:

```rust
fn made() -> Option<u32> { println!("made"); none_of(LEN.with(|l| l.get())) }
thread_local! {
    static LEN: Cell<u32> = const { Cell::new(1) };
    static LAZY: RefCell<Option<u32>> = RefCell::new(made());
}
pub fn main() { println!("start"); LEN.set(5); println!("{:?}", LAZY.with_borrow(Clone::clone)); }
```

Native Rust prints `start`, `made`, `Some(5)`; the JS, `let lazy = made()
?? null` at its module's top, printed `made` first and gave `None`: a
wrong answer, which the difference accepted.

**A thread-local is made where it's first used, `$local(() => ..)`, where
making it as its module loads would show** (`analysis/lazy_locals.rs`, the
one owner of where each is made): of all its initializer runs and reads,
`may_run`'s answer,

- **a body that prints or panics**, which Rust orders: a `println!`'s
  `_print`, or a call of what never returns, a `panic!`, an `assert!`'s
  failure, `unreachable!()`, `process::exit`. A std call's own panic,
  `expect` of `None`, and what a binding's call does, stay at the module's
  load (the guide's difference 25): React's `ReactSharedInternalsServer`,
  a binding's `expect`, still fails its module's load, as upstream's
  `throw` does;
- **state it reads that a body that may run after the modules load
  writes**: a thread-local of what has a cell (through the fields of the
  crate's types, the type arguments of std's, and what a reference points
  to; not a function's, a closure's or a `dyn`'s, which name no state, nor
  a binding's type's type arguments, React's `&'static
  Class<fn(.., Option<Rc<Fiber>>, ..)>`, a JS class), a `static mut`, a
  `static` of interior mutability, each written by
  a method that sets it or lends its cell, `set`, `take`, `replace`,
  `with_borrow_mut`, `with`, or by any other use but a read, `get` and
  `with_borrow`; a `static mut` where it's assigned or a pointer is taken;
  any use of another `static` of a cell;
- **state it writes that such a body reads or writes**, which would tell
  whether it's been made.

**A body runs only as the modules load** where it's a thread-local's
initializer made then, a `js::on_load!`'s, a function only calls reach
(ADR 0577's `only_called`) each of whose calls is in one of those, or a
closure one of those gives std's call to run there, keeping none of it (in
neither what it gives back nor what else it's given, a `with(|x| ..)`'s,
not an iterator's `map`). Any other body may run after: `main`, a function
JS may call, a trait's method, a closure kept. Each made where it's first
used runs after the load too, so what only it ran may make another show:
to a fixed point. One whose initializer only makes a value, calls a
binding, or reads and writes what only the loading code touches is made as
its module loads, as before: React DOM's `const isScrollEndEventSupported =
isEventSupported("scrollend")`, and DOMEventNames' `export const
ANIMATION_END = getVendorPrefixedEventName("animationend")`, which reads
`style`, a `Cell` only its module's `js::on_load!` sets, and its caches,
JS objects held by reference that bindings change (difference 25).

**A `pub` one that would show is refused**, as before rd51: JS's importers
read an `export const`, made as its module loads. "a `pub` thread-local
whose initializer prints, in `made`", ".. may panic, in `checked`", "..
reads `LEN`, which `grow` may change after the modules load", ".. changes
`COUNT`, which `count` may read after the modules load": make it
`pub(crate)`, made where it's first used, and export a `pub fn` that reads
it.

A lazy `#[rust_js::nullable]` one is a cell that stores `null` for `None`
however it's written, `$local(() => $nullableCell(..))` (#541's
`nullable_cell`), and so is the cell `set` makes of one not made yet,
`initWith($nullableCell(4))`, not `{ value: 4 }`.

Case ([0262](0262-when-rust-and-js-disagree.md)): C where it would show,
results changed: Rust's behavior is kept, `$local`. B, the accepted
difference, narrowed to what the program can't tell from Rust's order but
by what bindings do and std's own panics, and by what the loading
initializers do among themselves: two that share state only they touch,
a cache only they fill, are made in their modules' order, where Rust makes
each at its first use. N where nothing can tell.

Over-cautious, for now: a destructor std may run of what it's given, of a
type whose `Drop` prints, and a `with` of a thread-local of an `Rc` of a
cell, which may change what it points to, make one lazy that can't show
(`thread_local_refcell_set`'s `NOISY`, `refcell_flow_thread_local_init`'s
`SECOND`): correct, read by its call. And what's read of state isn't told
apart by its field: React Flight's `const TaintRegistryObjects =
$local(() => ReactSharedInternals.TaintRegistryObjects)`, and its three
siblings, read a `Cell` of the server internals, whose other cells, the
dispatcher's, functions set after the load, so they're made where they're
first used, where upstream destructures them as its module loads (told
apart since #616, the amendment below).

The ports, built on this (2026-11-04): React's (react-in-rust 06d6b3f9),
whose only thread-locals made where they're first used now are those four,
private; no `pub` one refused. alien-signals' JS is unchanged.

- Corpus `thread_local_lazy_where_observable`, against native Rust: the
  issue's, one that prints, one that may panic, two that read `LEN` set
  before their first use, one that writes `COUNT` that `main` reads, and a
  nullable one written `None` by `set`, `replace` and `with_borrow_mut` and
  `set` before its first use, each `$local`; a `const { .. }` one, a
  binding's `Math.max` and two of a cache only they touch, filled in a
  `with`'s closure, made as their module loads. `thread_local_exported_at_load`
  is of such a cache; the cases rd120 ordered to dodge the difference,
  `thread_local_made_where_used`, `thread_local_made_by_constructor`,
  `thread_local_reads_another_set`, `thread_local_exported_made`, are
  `$local` again.
- `test/lowering-module-variables.test.ts`: a `pub` one reading what a
  `js::on_load!` sets, an `export const`; a private one of what only makes
  a value at load, one that prints, `$local`, printing at its first use,
  and one of an import cycle, `$local` for that alone (the mutation
  `out-of-order-private-at-load`, whose corpus case's initializers print
  now). `test/diagnostics.test.ts`: the four refusals.
- Mutations in `scripts/mutations/lower/analysis/lazy_locals.ts`, and
  `nullable-local-set-plain` (`std_types/cell.rs`).

## Amendment: a namespace thread-local is first used as its module loads (#554)

The amendment above asked of every thread-local whether its making as its
module loads would show, a namespace one's too (rd293, ADR 1061). So
`test/namespace-values.test.ts`'s noop renderer, React's noop entries'
`export const { render, .. } = createReactNoop(..)`, was refused: its
`pub static NOOP: Renderer = create(3)`, whose `create` counts what it
makes in `MADE`, which `made`, a `pub fn`, reads, "reads `noop::MADE`,
which `noop::create` may change after the modules load". Valid Rust that
compiled before, and a refusal of what can't be made lazy: a namespace is
its module's exports, `export const { .. }`, never a `$local`.

**A namespace thread-local isn't asked whether its making would show**
(`lazy_locals`, by `classes::is_value_namespace`): Rust never reads it, a
read refused (ADR 1061), and JS reads it as its module's exports, which
are there from its module's load, so its first use is that load. Where
its module's load can't make it first, `load_cycles::out_of_order`'s
refusal holds as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: Rust makes it at its first use, and its only use, JS's, is
its module's load.

Held by `test/namespace-values.test.ts`, whose fixture's `create` writes
`MADE` and whose JS reads `made()` after the import, and the mutation
`at-load-namespace-shows` (`analysis/lazy_locals.ts`).

`test/es-classes.test.ts`'s ES class, whose thread-local `MADE` is made
by `Plugin::new`, which prints, is made where it's first used now, as
this amendment's rule says: Rust prints `made loud` before `made early`,
as `run.mjs` makes its `Loud` before it calls `early()`, which the
`const made = new Plugin("early")` it printed before reversed (case C).
Its class before what its module makes of it as it loads is held by a
second, `SHOUTED`, `Plugin::shout("hi")`, `const SHOUTED` after the class.

## Amendment: what a `with` lends is told apart by its fields (#616)

The amendment above took any use of a thread-local, but a read, for a write
of all of it. React Flight's four private `TaintRegistry*` thread-locals
each read one `Cell` of the server internals, which nothing writes after
the load, but Flight's `performWork` and `createRequest` set the
dispatchers' cells, `internals.h.set(..)`, so all four were `$local(() =>
ReactSharedInternals.TaintRegistryObjects)`, where upstream writes `const {
TaintRegistryObjects, .. } = ReactSharedInternals;` (the user accepted the
`$local` form for now, 2026-11-04, and asked for this).

**A thread-local's `with` given a closure that reads only fields of the
struct it lends, `INTERNALS.with(|internals| internals.registry.get())`,
reads and writes those fields alone** (`lazy_locals::fields_lent`, its
`Place`s): each use of the closure's parameter, `|internals|` or
`|&internals|`, is a field of what it points to, through its references
alone. An initializer that reads field X is made as its module loads where
nothing that may run after the load writes X, or the state whole; one
whose field is written then, `DISPATCHER` of `main`'s
`internals.dispatcher.set(5)`, is made where it's first used, as before.

- **Any other use of what it's lent writes every field**, as before: the
  struct handed to a function, a method, a binding (`Object.assign(target,
  ..)`), or another crate's function, which may write any field; a
  closure that captures it, which rustc's capture truncates to the struct
  at a shared reference's deref; a reference to it copied out, `*held`; a
  field of a union, whose fields aren't apart; and a `with` given anything
  but a closure. A thread-local's own `get`, `set`, `with_borrow` and
  `with_borrow_mut` are of the whole of it, as before.
- **A field may point to what another holds or points to**: two fields'
  `Rc`s of one cell, or references to one, or a reference to the struct
  itself. So a field read or written with what it points to is one place
  with another field where what one may point to (`pointees`: each type
  not `Freeze` it reaches through a reference, an `Rc`, an `Arc` or a
  `Weak`, through structs' fields and std's type arguments) is the other's
  type or meets what the other may point to; a raw pointer, a `dyn`, a
  closure, or a type rustc doesn't say may point to anything.
- **What it points to is read or written by any use but a `Cell`'s**: a
  `Cell`'s `set` writes the field alone; its `get` and `replace` read it
  alone where what they give out points to nothing that may change, a JS
  object's `PhantomData` among them, React's `Option<&'static WeakMap<..>>`
  and its dispatchers'. One that gives out what points to such state, a
  `Cell<Option<&'static Cell<u32>>>`'s `get`, reads and writes what it
  points to, as what's given out may be written through.

Case ([0262](0262-when-rust-and-js-disagree.md)): C where a field read is
written after the load, results changed: made where it's first used, as
Rust makes it. N where nothing writes the fields it reads, nor the state
whole: what's read is the same at the load as at its first use.

Not told apart, as before: a `static`'s fields, and a field of a field.
Writes through a reference copied out of the state by a read, a `Cell`
thread-local's `get` of a `&'static`, or held by another thread-local made
of it, aren't seen as writes of the state, of its fields or of the whole
of it alike: a gap of their own, a wrong answer reported with #616, which
this amendment neither opens nor closes: `static READ: u32 =
PTR.get().unwrap().n.get()` is made as its module loads though `main`
writes `PTR.get().unwrap().n` first.

The ports, built on this: React's ReactFlightServer destructures the four
again, `const { TaintRegistryObjects, TaintRegistryValues,
TaintRegistryByteLengths, TaintRegistryPendingRequests } =
ReactSharedInternals;`, upstream's.

- Corpus `thread_local_lazy_by_field`, against native Rust: `REGISTRY`,
  of a field nothing writes after the load, made as its module loads;
  `DISPATCHER`, of one `main` sets, where it's first used; and one of each
  way a field is written whole or through another, each where it's first
  used: a function given the struct, its method, a binding's
  `Object.assign`, a closure that captures it, a reference copied out,
  `Rc`s of one cell, and references to one given out by `get`.
- Mutations `field-*` in `scripts/mutations/lower/analysis/lazy_locals.ts`.
