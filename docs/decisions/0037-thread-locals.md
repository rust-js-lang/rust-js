# 0037. `thread_local!` is a variable of its module

Status: Accepted. Amended, below; last: a `pub` thread-local is made as its module loads ([1006](1006-a-pub-thread-local-is-made-as-its-module-loads.md)), where an amendment had refused one made where it's first used.

Case: A, B, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A web program keeps state between events: how many times something ran,
the element currently on screen, a log. The playground keeps three such
values for its Result frame. In JS they're variables at a module's top level.

Rust keeps such state in a `static`. A `static` must be `Sync`, and JS values
aren't: an `&'static Element` can't go in a `Mutex`. So Rust programs on
the web keep JS values in `thread_local!`, as wasm-bindgen's documentation
recommends:

```rust
thread_local! {
    static COUNT: Cell<u32> = Cell::new(0);
}
```

`static` items were an error in rust-js (ADR 0016). ReScript has top-level
`let`s holding `ref`s, and Scala.js has `object`s with `var`s.

## Decision

**A thread-local is a `const` of its module, holding its value**, made from
the macro's initializer:

```rust
thread_local! {
    static COUNT: Cell<u32> = Cell::new(0);
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}
pub fn bump() -> u32 { COUNT.set(COUNT.get() + 1); COUNT.get() }
```

```js
const COUNT = { value: 0 };
const LOG = { value: [] };
export function bump() {
  COUNT.value = COUNT.value + 1 >>> 0;
  return COUNT.value;
}
```

- **JS runs a module on one thread**, so "one per thread" is "one", the
  module's own variable. It goes at the top of the file with the `const`
  items (ADR 0031), and is exported if another module uses it.
- **`thread_local!` expands to `const NAME: LocalKey<T>`**, whose block holds
  `fn __rust_std_internal_init_fn() -> T { init }` and std's storage for it,
  a `static` of each target's kind. rust-js lowers the `init` function, uses
  its body as the value, and leaves the storage out.
- **The methods**, on the `Cell` or `RefCell` it holds (ADR 0023):

  | Rust | JS |
  |---|---|
  | `KEY.get()`, `KEY.set(v)` | `KEY.value`, `KEY.value = v` |
  | `KEY.with(f)` | `f(KEY)` |
  | `KEY.with_borrow(f)`, `KEY.with_borrow_mut(f)` | `f(KEY.value)` |

  A closure that only returns is put in place, on the key or its value:
  `START.with(|s| s.get())` is `START.value`, not `((s) => s.value)(START)`.

## Why

- **It's how Rust programs for the web already keep state**, and rustc
  checks every use of it.
- **The JS is the variable a JS programmer would write**, with nothing of
  std's storage left.

## Alternatives

- **`static mut`**: plain, but `unsafe` at every use, and Rust 2024 warns
  about references to one.
- **`static` of a `Sync` type** (atomics, `Mutex`, `OnceLock`): right for
  numbers, but JS values aren't `Send` or `Sync`, so it can't hold an
  element.
- **Pass the state around**: works, but a web page's event handlers all need
  the same state, and that's what a module's variables are for.

## Consequences

- **A thread-local is made when its module loads**, not at first use as in
  Rust. An initializer with side effects, or one reading another module's
  thread-locals, runs at load. Initializers like `Cell::new(0)` and
  `RefCell::new(Vec::new())` can't tell.
- `with` and `with_borrow_mut` call their closure right there:
  `((log) => log.push(line))(LOG.value)`.
- Plain `static`s were still an error, until ADR 0096, and so are `take`
  and `replace` on a thread-local.

## Amendment: a `with` of one statement

`KEY.with(|it| { it.f(); })`, of a closure of one statement, is that
statement, the thread-local in its parameter's place: `KEY.f();`, as
react.dev's runESLint writes `linter.defineRules({..})`. It was the
closure called in place, `((it) => { it.f(); })(KEY)`.

## Amendment: a constant holds a thread-local as its variable

A constant's `&'static LocalKey<T>` is the thread-local's variable,
which every use of the constant shares as every use of the key does:
react.dev's MDXComponents table holds `pre: CodeBlock`, a memo
component, `export const MDXComponents = { .., pre: CodeBlock, .. }`.
It was refused as a constant of a type rust-js can't share. A JSX test
renders a table's memo; a mutation refuses it again.

## Amended by

- [0270](0270-module-variables.md): A thread-local only read and set is its module's variable
- [0306](0306-item-order.md): A module's items are where its Rust has them
- [1006](1006-a-pub-thread-local-is-made-as-its-module-loads.md): A `pub` thread-local is made as its module loads

## Amendment: one whose initializer runs the crate's code is made where it's first used

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what a thread-local's
initializer prints, a panic in it, and what it reads of another module
were where its module loaded; they're where Rust has them now. B for what
a binding's call does in one, which the user decided (2026-10-25).

A thread-local was made as its module loaded, its initializer the
module's code. Rust makes each at its first use. `thread_local!{ static
X: Cell<u32> = Cell::new(noisy()); }` printed `init` before `main`'s first
line, where Rust prints it at the first `X.with`; and in an import cycle,
one whose initializer called a function of a module not yet loaded threw
a `ReferenceError` reading that module's `const` (#207). React's contexts
are thread-locals too, `pub static TocContext: Context<Toc> =
create_context(Vec::new())`, which React writes as `export const
TocContext = createContext([])`, made as the module loads.

- **A thread-local whose initializer runs the crate's code, or may
  panic, is made where it's first used** (`analysis/lazy_locals.rs`): its
  initializer's MIR calls a body of the crate's, a function, a method of
  its impl or a closure, or of what rust-js can't see the body of; drops a
  value of the crate's with a destructor; checks what may fail; or calls
  what never returns. It's `$local(() => init)`, whose call makes it the
  first time and gives it after, read by its call, `source()`, the
  crate's or a library's (`Calling.lazy`, ABI 5); one whose initializer
  panics stays unmade, and its next use runs it again, as Rust's does.
  Its `with(|x| ..)` holds it first, `const x$1 = x();`, and of a closure
  that only reads a path of it, `|e| e.clone()`, is that path of its
  call, `source()`.
- **One that only makes a value, or calls std's or a binding's
  functions, is made as its module loads**, as JS writes it, React's
  `createContext([])`: the user's choice of the JS idiom where only JS's
  or std's code runs. What a binding's call does then, or a std call's
  panic, is at the module's load, the guide's difference 25.
- **A lazy one runs nothing as its module loads**, so a dictionary in an
  import cycle with its module isn't refused for it (ADR 0592): `counts`'
  thread-local of `shapes::total` compiles, as Rust's does.
- Held by `test/corpus/thread_local_made_where_used.rs`, against native
  Rust: an initializer's print after `main`'s, one's drop, one that
  panics made again, an import cycle's (#207), and another module's
  `const` read by its name; `test/lowering-structs-enums.test.ts`'s
  dictionary cycle, which compiles, and one of a binding's, refused; and
  the mutations `lazy-local-*`, `local-made-*`.

## Amendment: made after what its call is given, and `set` before its first use (#252)

A thread-local made where it's first used was read where its reference
was, `key()`, before what the call that uses it is given:
`COUNT.replace(made(9))` ran its initializer, then `made(9)`, where Rust
runs `made(9)`, then the initializer, inside `replace`. And `LocalKey::set`
of one not made yet read it, so its initializer ran and the `set` dropped
what it made, where std's `set` makes it of the value given and never runs
the initializer (#252).

**Its read is `Known::LocalRead`, the same cell wherever it's read, read
where its call uses it:** `reads_same`, and `reads_same_anywhere`, so a
`flush` keeps it in place, and nothing a call's arguments are ordered
against, `has_effects`; a `Cell`'s or a `RefCell`'s call of it makes what
it's given first, then reads it. **`set` of one is `if
(!key.initWith({ value })) { .. }`**: `$local`'s `initWith` makes it of the
cell given where it isn't made, and says so; one made already is set as
before, what it held dropped. `FnCx::lazy_key_getter` is the one answer to
what a read of one calls. Tested by `thread_local_set_before_use.rs`,
against native Rust, a `Cell`'s, a `RefCell`'s and one of a value with a
destructor, set before and after it's made, and one given what `vec!`
makes; a `RefCell` of an array, which `set` writes as a `{ value }`, is
#249's.

## Amendment: a fn pointer's call runs what it may be

React's scheduler (`packages/scheduler/src/forks/Scheduler.rs`,
react-in-rust) picks its clock once, as upstream's `Scheduler.js` does:

```rust
thread_local! {
    static GET_CURRENT_TIME: Box<dyn Fn() -> f64> = if js::is_defined(webapi::Performance::now) {
        let local_performance = webapi::performance;
        Box::new(move || local_performance.now())
    } else {
        let local_date_now: fn() -> f64 = js::date::now;
        let initial_time = local_date_now();
        Box::new(move || local_date_now() - initial_time)
    };
}
```

Upstream, and the port's JS before the amendment above, is `let
getCurrentTime; if (typeof performance !== 'undefined' && ..) { ..
getCurrentTime = () => localPerformance.now(); } else { .. }`. Since it,
the port read `const GET_CURRENT_TIME = $local(() => { .. }); function
getCurrentTime() { return GET_CURRENT_TIME()(); }`: `local_date_now()` is
a fn pointer's call, which `lazy_locals` took as one that may be the
crate's, though the only function the crate makes a `fn() -> f64` of is
`js::date::now`, a binding's.

**A fn pointer's call runs what it may be** (ADR 0907's
`FnPointers::may_be`, which `CellBorrows` holds and gives to
`lazy_locals`): the crate's code where any function or closure it may be
is the crate's, by the rule a direct call of it has, or where it may be
anything; a binding's or std's alone, none. So the scheduler's clock is
made as its module loads, upstream's `let` and `if` again (ADRs 0601,
0610), and one whose fn pointer may be the crate's function is made where
it's first used, as before. Case B, as this amendment's first was decided:
what `Date.now` does, at the module's load.

- `test/corpus/thread_local_made_where_used.rs`'s `POINTED`, a
  thread-local of a fn pointer of the crate's `noisy`, prints after `main`'s
  first line, against native Rust; `test/port-scheduler.test.ts`'s
  snapshot holds the scheduler's `let getCurrentTime;`.
- Mutations `lazy-local-pointer-unseen`, `lazy-local-pointer-any`.

## Amendment: a function of the crate's that only makes a value

Case: A ([0262](0262-when-rust-and-js-disagree.md)): nothing a program
prints, throws or reads can tell when such a thread-local is made.

React's `ReactFiberHiddenContext` makes its cursors by `ReactFiberStack`'s
`createCursor`, as upstream does, `export const
currentTreeHiddenStackCursor = createCursor(null);`:

```rust
pub fn create_cursor<T>(default_value: T) -> StackCursor<T> {
    StackCursor { current: RefCell::new(default_value) }
}

thread_local! {
    static CURRENT_TREE_HIDDEN_STACK_CURSOR: StackCursor<Option<Rc<HiddenContext>>> = create_cursor(None);
}
```

`create_cursor` is the crate's, so the amendment above made it where it's
first used, `$local(() => createCursor(null))`, read by its call, though
it runs nothing that could tell when: it makes a value of what it's given.

**A call of a function of the crate's that only makes a value is one
that runs nothing of the crate's** (`lazy_locals`' `Makers::makes_only`):
a free function that reads no `static` or thread-local, may not panic,
drops nothing of the crate's with a destructor, and calls only others
that do, not itself, and std's that only make a value
(`std_makes_only`): a `const fn` of no trait, asking nothing of its type
parameters but their size, `RefCell::new`, not `Iterator`'s methods, of a
`const` trait, nor `Option::map`, given a closure. No binding's, closure's,
`dyn`'s or fn pointer's call. So the cursor is `const
currentTreeHiddenStackCursor = createCursor(null);`, read by its name.

- **Not of a module that may load after its own**: one whose maker, or a
  maker that one calls, is of a module that imports the thread-local's
  back, in an import cycle, may run before that module made its `const`s,
  a `ReferenceError` in JS. The modules' imports are known once the crate
  is lowered (`load_order`): `load_cycles::made_in_cycles` finds each, the
  pipeline returns them, `Lowered::MadeWhereUsed`, and the driver lowers
  the crate again with them made where they're first used.
- What a maker of one that only makes a value reads by its name, a
  `const`, is read where its module has made it, as the cycle above can't
  be.

Held by `test/corpus/thread_local_made_by_constructor.rs`, against native
Rust and its snapshot: a maker's cursor and one of a maker of a maker
made as the module loads; one whose maker reads a `static` or a
thread-local, prints, or gives std's `Option::map` a closure that prints,
made where it's first used; and one in a module whose maker's module
imports it back, made where it's first used, which a `ReferenceError`
says otherwise. `fn_pointer_borrows_nothing`'s `const clz32 =
chosen(true);` and `nullable_generic_fields`' `const hidden =
createCursor(null);` read by their names now. The mutations
`lazy-local-maker-unseen`, `maker-reads-unseen`, `maker-std-calls-any`,
`std-maker-bounds-unseen` and `maker-cycle-unseen`.

## Amendment: a boxed closure only makes a value

Case: A ([0262](0262-when-rust-and-js-disagree.md)), as the amendment
above: nothing a program prints, throws or reads can tell when the value
is made.

React's ReactChildFiber makes its reconcilers by a factory of closures
that capture its flag, `export const reconcileChildFibers =
createChildReconciler(true);`. The natural Rust:

```rust
fn create_child_reconciler(should_track_side_effects: bool) -> Box<ChildReconciler> {
    Box::new(move |return_fiber, current_first_child, new_child, lanes| { .. })
}

thread_local! {
    pub static RECONCILE_CHILD_FIBERS: Box<ChildReconciler> = create_child_reconciler(true);
}
```

`Box::new` isn't a `const fn`, so `create_child_reconciler` was no maker,
and the thread-local was `export const reconcileChildFibers = $local(()
=> createChildReconciler(true));`, a getter: BeginWork's JS calling
`reconcileChildFibers(returnFiber, ..)` got the reconciler back.

**`Box::new(x)`, which is `x` (ADR 0069, `PureStd::BoxNew`), only makes
a value** (`std_makes_only`), as making a closure does, which runs
nothing: the reconciler is `export const reconcileChildFibers =
createChildReconciler(true);`, its Rust call `reconcileChildFibers(..)`.

- **A `LazyLock` static stays lazy, `{ init }`** (ADR 0318): its type says
  it's made where it's first used, which its `$forceLock` keeps; a value
  made as its module loads is a `thread_local!`, as React's module state
  is.
- **The factory's closures stay arrows**, `const deleteChild =
  (returnFiber, childToDelete) => { .. }`, where upstream writes nested
  `function deleteChild(..) { .. }`: which form a closure of statements
  bound by `let` takes is a decision of every closure's, left open.
  (Decided since by ADR 0882's amendment: a function declared,
  `function deleteChild(..) { .. }`.)

Held by `test/corpus/thread_local_made_by_boxed_closure.rs`, against
native Rust and its snapshot, and the mutation `maker-boxed-closure-lazy`.

## Amendment: an initializer that reads what may change, prints, or gives std the crate's closure (#376)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results changed;
Rust's behavior is kept. Three wrong answers.

```rust
thread_local! {
    static TRIES: Cell<u32> = const { Cell::new(0) };
    static MADE: Option<u32> = if TRIES.get() == 0 { Some(1) } else { None };
    static PRINTED: u32 = { println!("printed"); 3 };
    static MAPPED: Option<u32> = Some(2).map(|n| { println!("mapped {n}"); n * 2 });
}

fn main() {
    println!("main");
    TRIES.set(3);
    println!("{:?}", MADE.with(|m| *m));
}
```

Rust prints `main`, then `None`: `MADE` is made where it's first used,
after `TRIES` is 3. Each initializer called only std's code, `LocalKey::get`,
`_print`, `Option::map`, which the first amendment took to run none of
the crate's, so each was made as its module loaded: `MADE` was `Some(1)`,
and `printed` and `mapped 2` came before `main`. A maker of the crate's
that reads a `static` or a thread-local, or prints, was already refused
(`makes_only`); the initializer's own body wasn't asked so.

**An initializer is made where it's first used where it** (`lazy_locals`'
`Initializer`):

- **reads what may change after its module loads**: a `static mut`, or a
  `static` with interior mutability, an atomic's `HITS.load(..)`; or
  another thread-local, by its key, `TRIES.get()`, `NAMES.take()`, but one
  that's made as its module loads, in the same module before it, and holds
  what can't change once made (`unchanging`: numbers, text, functions and
  fn pointers, through shared references, `Box`, `Vec`, `String`, `Rc` and
  tuples, and types of no cell whose fields are; not a cell, an atomic, a
  `dyn` or a binding's JS value). That one it reads the same whenever it
  runs: React's scheduler's `schedulePerformWorkUntilDeadline` reads
  `localSetImmediate`, an `Option<fn>` made of a binding, and stays
  upstream's `let` and `if`. A read of one made where it's first used makes
  the reader so too, to a fixed point: reading it as the module loads would
  run its initializer there. A `static` that can't change, `BASE * 2`, or
  a binding's, React's `__DEV__`, is read as before, as its module loads.
- **prints**: std's `_print` or `_eprint` (`recognition::print_of`, which
  `classify` and the unwinding analysis ask too).
- **gives std a closure or a function of the crate's that runs its code**:
  among a std call's type arguments, a closure whose body would, by these
  rules, `Some(2).map(|n| { println!(..); .. })`, or a function that isn't
  one that only makes a value. Not a call that only makes a value of it,
  `Box::new(f)`, which stores it, and not a closure that only reads what
  it's lent, the scheduler's `LOCAL_SET_IMMEDIATE.with(|f| *f)`.

What a std call does inside std, its panic, and what a binding's call
does, stay at the module's load, as decided (the guide's difference 25).

Held by `test/corpus/thread_local_reads_another_set.rs`, against native
Rust and its snapshot: a `Cell`'s and a `RefCell`'s key, a `static mut`
and an atomic read, a print, a closure given `Option::map`, one reading a
lazy one, one reading one made after it, and one indexing past its end,
each made where it's first used; a `static` that can't change and a `fn`
thread-local made before, read as the module loads; and
`test/port-scheduler.test.ts`'s snapshot. The mutations
`initializer-*` and `lazy-local-panics-unseen`.

## Amendment: a library's flag, and another module's, read as the module loads

Case A, as the amendment above: it reads the same whenever it runs.

The amendment above made a thread-local reading another made where it's
first used unless the other is of its own module, made before it.
React's `ReactFiberPerformanceTrack`'s `supportsUserTiming` reads
`shared`'s `enableProfilerTimer`, a `bool`, and was `$local(() => ..)`,
called at each use, where upstream's is its module's `const`.

**One that holds what can't change (`unchanging`) and is made as its
module loads, before the reader, is read as the reader's module loads**
(`made_ahead`): a library's, whose contract doesn't say it's made where
it's first used (`ItemFact::Lazy`), as a library's modules load before
any of the crate's; another module's of the crate's, where that module
loads first: a maker of the reader's (`made_by`), which the pipeline
makes where it's first used where the two modules import each other
(`made_in_cycles`), as it does a function's.

Still made where they're first used: React's `ReactTaint`'s
`TaintRegistryObjects` and the like, which read cells of
`ReactSharedInternals`, which its `js::on_load!` sets, and its
`TypedArrayConstructor`, which calls `shared`'s `getPrototypeOf` fn
pointer, which may be any function: written up for a decision.

Held by `test/crates.test.ts`'s "a thread-local of a library's flag is
made as its module loads" (and its pair, against native Rust: one of the
library's count, which it sets, where it's first used) and
`thread_local_reads_another_set`'s `USES_FLAG`, made as its module loads,
and `uses`' `LIMITED`, in an import cycle, where it's first used;
mutations `library-read-taken-lazy`, `other-module-read-unordered` and
`other-module-read-lazy`.


## Amendment: a lazy thread-local's `with` of a path is tidy's to apply

`source.with(|e| e.clone())` of a thread-local made where it's first used
was the path of its call, `source()`, where `calls::apply` lowered it, and
again where tidy applies an arrow called where it's made (`applied
arrows`). The first decided nothing: without it the corpus, the snapshots,
the ports, JSX and the module tests wrote the same JS, and its mutation,
`lazy-local-path-unapplied`, survived. It went with its mutation.

## Amendment: a `pub` thread-local made where it's first used is refused

Since [1006](1006-a-pub-thread-local-is-made-as-its-module-loads.md)
(the user's decision, rd51), a `pub` thread-local is made as its module
loads, as upstream's `export const` is, and refused only where its
module's load can't make it before what it reads. What follows is the
history.

Case ([0262](0262-when-rust-and-js-disagree.md)): D, refused, where a JS
module importing it would have read another value.

React DOM's `DOMEventNames`, ported, writes upstream's `export const
ANIMATION_END = getVendorPrefixedEventName('animationend')` as

```rust
thread_local! {
    pub static ANIMATION_END: String = get_vendor_prefixed_event_name("animationend");
}
```

whose initializer reads and writes `getVendorPrefixedEventName`'s
thread-locals, so it's made where it's first used, as Rust makes it:
`export const ANIMATION_END = $local(() => getVendorPrefixedEventName("animationend"))`.
The crate's own modules read it by its call, `ANIMATION_END()`. Its
declaration said `export const ANIMATION_END: string`, and
`SimpleEventPlugin.js`, upstream's JS, which imports it, read the function
`$local` makes in a `switch` of strings: a wrong answer, without an error.

**A `pub` thread-local made where it's first used is refused** ("a `pub`
thread-local made where it's first used .. so JS's importers would read
the function that makes it, not its value"): JS's `export const` is made
as its module loads, and one made where it's first used can't be that
value. Write it `pub(crate)`, which the crate's modules read where they
use it, as before, and a `pub fn` that reads it for JS. One made as its
module loads, of an initializer that only makes a value, is exported as it
was.

Whether to make such a thread-local as its module loads instead, after
the modules it imports, as upstream's JS does, is a difference from Rust
where its initializer's effects are seen, which only the user decides
(`decisions-pending.md`, rd51).

### What holds it

- `test/diagnostics.test.ts`, "a pub thread-local made where it's first
  used"; `docs/guide/limitations.md`'s example.
- Corpus `thread_local_exported_made`, against native Rust: a `pub`
  thread-local made as its module loads, and a `pub(crate)` one made where
  it's first used, read by another module, and given JS by a `pub fn`.
  `thread_local_made_where_used`'s `START`, which no other module reads, is
  no longer `pub`.
- Mutations `exported-lazy-local-unrefused` and `lazy-local-refused-unexported`
  (`analysis.rs`).
