# 0914. A library's generic function is its program's; a consumer that gives it more compiles its own

Status: Accepted. Amends [0098](0098-destructors.md),
[0289](0289-copy-bounds.md), [0300](0300-drops-by-need.md) and
[0360](0360-the-app-is-a-program.md): a library's public generic function
takes the copies and drops its own crate's calls give it, no more.
Amended (below): a library's trait impls and copies are its program's too,
and amends [0100](0100-separate-crates.md) and
[0185](0185-library-trait-defaults.md); and a consumer gives an export the
copy functions its contract says it takes; and a `Clone` bound, and a
program's `Rc` of a type parameter's value, are what its callers give;
and a binding's result copies nothing; and a namespace's method called
through a value is its library's, never compiled here
([1037](1037-a-module-namespace-is-a-unit-struct-of-its-functions.md),
amended, rd311); and a slice's `concat()` copies
its items alone.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results change for
JS callers, so Rust's behavior holds for Rust callers, and JS's call is
upstream's. A JS caller calls a library's function as its declarations
say; a Rust caller that gives it a value to copy or drop gets that copy
and drop, by a function of its own.

## Context

A crate another crate uses is a library, whichever package is built (ADR
0360, amended), and a library's public generic function took a copy
function for each `T: Copy` (ADR 0289) and a drop for each type parameter
(ADRs 0098, 0300) a consumer could give a value that needs one, as its
consumers are callers it never sees. So a crate's JS changed when another
crate started using it. React's port added `react-isomorphic` to the
reconciler's dependencies, and `react`'s committed JS became:

```js
export function createContext(defaultValue, TCopy) {
  ..
    _currentValue: TCopy ? TCopy.copy(defaultValue) : defaultValue,

export function useState(initialState, dropS) {
  let dispatcher;
  try {
    dispatcher = resolveDispatcher();
  } catch (error) {
    if (typeof initialState !== "function") {
      dropS?.(initialState);
    }
    throw error;
  }
```

ADR 0289's amendment made the copy function optional, so React's own JS,
`createContext(defaultValue)`, still worked; but the parameters, the
`TCopy ? .. : ..` reads and the `try`s were in the JS that's meant to be
React's, and `startTransition(scope, options, dropFnOnce)` and every hook
with them, none of which a JS caller gives or React writes.

## Decision

**A library's generic function takes the copies and drops its own crate's
calls give it, as a program's does: its JS is the same whether another
crate uses it or not, the one its JS callers call. A consumer whose call
gives it a value to copy or drop that its export takes nothing for
compiles its own of it, from the MIR its library's metadata keeps, beside
its first caller, and calls that.**

```rust
// react
pub fn use_state<S>(initial_state: S, dispatcher: Option<&'static str>) -> (S, &'static str) {
    let dispatcher = resolve_dispatcher(dispatcher);
    (initial_state, dispatcher)
}
// its consumer
dep::use_state(Loud(1), Some("client"))
```

```js
// react's
export function useState(initialState, dispatcher) {
  dispatcher = resolveDispatcher(dispatcher);
  return [initialState, dispatcher];
}

// its consumer's
function useState(initialState, dispatcher, dropS) {
  let dispatcher$1;
  try {
    dispatcher$1 = resolveDispatcher(dispatcher);
  } catch (error) {
    dropS?.(initialState);
    throw error;
  }
  return [initialState, dispatcher$1];
}

function resolveDispatcher(dispatcher) { .. }
```

- **What a call gives more of** (`instances::needed`): for a type
  parameter its body may drop, a value whose drop runs a destructor, as
  the consumer drops it (`drop_runs`), its own type parameter given a drop;
  for a `Copy` bound, a value whose copy isn't itself, its own type
  parameter given a copy; where the export's contract, its `params`, says
  it takes none, `drop` or `copy` (library ABI 8, which adds `copy`).
- **Whether its body may drop one** is read of its MIR: a drop of a place
  that holds one, where it runs or unwinds to from what may throw, not a
  binding that can't (ADR 0301); or a call it's given to that may, a
  function of its library's that may drop it there (asked in turn), or
  any other but a closure's, a `dyn Fn`'s or a fn pointer's, whose own
  code drops what it's given. A value optimized MIR copies where it's last
  used, `copy _1`, is given as one moved is.
- **An instance is the crate's own**: `Foreign::item` has none of it, so
  its calls are of it, its drops and copies are what the crate's calls
  give it, narrowed to those its body uses (ADR 0300), and it isn't
  exported unless another module of the crate calls it. It's named as the
  library names it, `useState`, and placed in the module of the crate's
  that first calls it, its closures with it.
- **What it calls of its library** is the library's export where there's
  one, public, and its own instance where there's none, a private
  function, compiled too; a library function it calls given more, its own
  too, until none more is needed: the pipeline returns the instances it
  needs, `Lowered::Instances`, and the driver lowers the crate again with
  their bodies.
- **Refused**: one that reads its library's private `static` or
  thread-local, of which its library's code has the one there is, which no
  other crate can reach; and one that calls a binding its library names by
  a path relative to that library's own file. Each names the function and
  what it reads, and says what to write.
- **A library keeps its functions' MIR in its metadata**,
  `-Zalways-encode-mir`, which a consumer reads. Optimized MIR keeps a
  drop flag where a value may be moved before a panic, `switchInt(copy _4)`
  in a cleanup: lowered as the `if` of what each way drops, and the flag
  counted read, so its writes are kept.
- **Trait methods and impls are as they were**: a dictionary's callers
  call them, which no consumer can see to compile them for, so a generic
  method takes a drop of each of its own type parameters (ADR 0163), and
  an impl's of each of its own.
- **`@rust-js/std`'s bodies are as they were**: compiled once into a
  package, each takes a drop of each type parameter a consumer could give
  one of (ADR 0561).

## Why

- **It's the JS React writes**, and any library's author: `createContext(
  defaultValue)` and `useState(initialState)`, the same whether another
  crate of the workspace uses them or not.
- **It's the program Rust runs**: what a consumer's value needs, a copy,
  a drop on unwind, is done where Rust does it, by the function compiled
  for it, as Rust's codegen compiles a generic function in the crate that
  instantiates it.
- **A copy can't be made from outside**: two cells a library's body makes
  of one `T` hold one object in JS, which its consumer's change in place
  then changes in both; only the body can copy it. Nor can a drop on
  unwind: the caller can't tell a panic before its value was moved from
  one after.

## Alternatives

- **The dictionaries as trailing optional arguments**, whose absence means
  a JS value, no copy and no drop (ADR 0289's amendment): JS calls them as
  declared, but the parameters, the `TCopy ? TCopy.copy(x) : x` reads and
  the `try`s stay in the library's JS, which differs from its program's.
- **A second entry for Rust callers in the library's module**,
  `useState(initialState)` beside one taking the dictionaries: JS calls
  upstream's signature, but the module gains a function upstream hasn't,
  and the crate's JS still changes when a consumer appears, or carries
  dead code where none does.
- **Per-type instances exported by the library**: it's compiled before
  any consumer, so it can't know their types.
- **Refusing a consumer that gives more** than the library's function
  takes, as ADR 0844 refuses counting an `Rc` a library doesn't: correct,
  but no generic library could be given a value with a destructor or a
  `Copy` struct changed in place, which `drops_by_need`, `copy_given` and
  generic containers do.
- **The workspace's consumers asked first**, the library compiled for
  what they give: a build in two passes Cargo has no order for, and a
  library on npm has consumers no build sees.

## Consequences

- `react`'s JS is the same with the reconciler using it as without:
  `createContext(defaultValue)`, `useState(initialState)`,
  `startTransition(scope, options)`.
- A consumer that gives more has its own copy of what it gives more to,
  beside its code, and its JS reads as the library's does, of MIR alone:
  no comments, and names of what MIR names.
- A crate is analyzed again for each round of instances it needs, which
  only a crate that gives a library's function more pays.
- **What still differs between a library's JS and its program's**: the
  dictionaries of its trait impls a consumer may use, a derive's
  `PartialEq` and `Hash` of a public type and a marker trait's impls, which
  a library emits and exports for its consumers (ADR 0100), as React's
  `fiberHash`, `requestCleanupQueueHash` and `refReactContextUsableT` are;
  and the copies a library makes of its public `Copy` types a consumer may
  change in place (`copies.rs`'s `crosses_crates`, ADR 0052). An impl's
  instance, compiled by the consumer that needs it as a function's is,
  would make the first the program's too. (Amended below: both are the
  program's now.)
- An `Option`'s clone whose payload's clone is the payload is the
  `Option` itself, `element.key`, not `element.key == null ? element.key :
  element.key`, which React's `cloneElement` was in its library's form.

## What holds it

- `test/crates.test.ts`, "a library's generic function is its program's,
  alone or a dependency, and its consumer compiles what it gives more
  of": `test/crates/pairs/program_form`, against native Rust, the
  library's JS built alone and as a dependency the same bytes, called from
  JS as declared, its consumer's own `createContext`, `useState`,
  `resolveDispatcher` and `pairOf`, and the library's `twice`, whose
  contract takes the copy its consumer gives.
- `test/crates.test.ts`, "a library's function its consumer compiles is
  refused where it reads its library's private state"; "a library's
  generic function takes the drops its body uses", `drops_by_need`; "a
  library's Copy bound takes a copy only where one is given",
  `copy_given`; and the pairs and triples, against native Rust.
- `test/bindings.test.ts`, "a binding that can't throw, read before a
  move, leaves nothing to drop": the consumer calls the library's `count`,
  and compiles its own `countUnmarked`.
- Mutations in `scripts/mutations/lower/instances.ts`, and
  `cleanup-flag-unread` (`mir/locals.ts`), `cleanup-flag-refused`
  (`mir/drops.ts`), `instance-imported`, `library-copy-unlisted`
  (`library.ts`), `instance-exported` (`mir.ts`),
  `instance-takes-unused-drops` (`drops/uses.ts`),
  `instance-copies-unasked` (`analysis/mutation.ts`) and
  `library-takes-every-drop` (`analysis/drops.ts`).

## Amendment: a library's trait impls and copies are its program's too

Case ([0262](0262-when-rust-and-js-disagree.md)): C. What a library's own
JS does is its program's; what its consumer's Rust needs more of, a
dictionary, a copy, the consumer's code does, as Rust's.

What this decision left a library writing for its consumers, which its
program didn't, still changed a crate's JS when another started using
it. React's port added `react-isomorphic` to the reconciler's
dependencies, and `react`'s committed JS gained, of dictionaries no code
of React's asks for:

```js
// ReactHooks.js
export function refReactContextUsableT(dropT) {
  return $traitImpl($refReactContextUsableT, [dropT], () => ({}));
}
// ReactStartTransition.js
export const fiberHash = { hash: fiberHash_hash, hashSlice: $hashSlice(fiberHash_hash) };
export function fiberHash_hash(fiber, state, HHasher, dropH) { .. }
```

and `ReactTaintRegistry.js` its `requestCleanupQueuePartialEq` and
`requestCleanupQueueHash`. A library also exported each trait impl's
method, `export function digitPartialEq_eq`, which its program doesn't,
and copied a public `Copy` type, a tuple and a `Vec` wherever it read one
(`copies.rs`'s `crosses_crates`), as a consumer might change it.

**A library's JS is its program's: it writes and exports what its program
does, and its manifest lists what its JS exports. What a consumer asks
of it that its JS hasn't, the consumer compiles from the library's MIR,
as it compiles the generic functions above:**

```rust
// dep
pub struct Digit(pub u32);
impl Hash for Digit { fn hash<H: Hasher>(&self, state: &mut H) { (self.0 % 10).hash(state); } }
pub fn hash_of<T: Hash>(value: &T) -> u64 { .. }
#[derive(Clone, Copy, Debug)]
pub struct Spot { pub x: i32 }
pub fn first_spot(spots: &[Spot]) -> Spot { spots[0] }
// app
dep::hash_of(&dep::Digit(1));
let mut spot = dep::first_spot(&spots);
spot.x += 10;
```

```js
// dep's: its program's, and the same as a dependency
export function firstSpot(spots) {
  return $index(spots, 0);
}

// app's
const digitHash = { hash: digitHash_hash, hashSlice: $hashSlice(digitHash_hash) };
function digitHash_hash(digit, state, HHasher, dropH) {
  HHasher.writeU32(state, digit[0] % 10);
}
function firstSpot(spots) {
  return { ...$index(spots, 0) };
}
hashOf([1], digitHash);
```

- **An impl its JS doesn't make** (a derived `Debug` or `Hash`, a
  `Hash`, a marker's, a `PartialEq` of a type equal only to itself, an
  operator's: what a program writes only where its code asks for it), or
  doesn't export (a private trait's), **is the consumer's**, where its
  lowering asks for it: its dictionary, `dictionary`, a call of one of its
  methods, a drop of its type. Lowering asks (`Foreign::compiles`, the
  crate's `asked`), the pipeline returns what was asked,
  `instances::asked`, the impl with each of its methods, and the driver
  lowers the crate again with them, beside where each was first asked.
  Of an impl, only what the consumer uses is written: `labelDebug_fmt`,
  not `labelDebug`.
- **The method of an impl whose dictionary it exports is called through
  the dictionary**, `strToKebabCase.toKebabCase(title)`, `hourDisplay.fmt(
  hour)` (`Foreign::through_dictionary`, `dictionary_entry`), where it's
  called as the method is: one with no type parameters of its own, given
  what it takes as the method takes it; else the consumer compiles the
  impl. The dictionary's manifest item says it may fail where a method of
  it may (ADR 0187).
- **A trait's default a consumer's impl doesn't write** is compiled by
  the consumer, a function over `Self` as ADR 0185 has it, the consumer's
  own: a library writes none.
- **A library function or impl that may copy, or clone, a value of what
  the consumer changes in place is the consumer's** (`instances::Copying`,
  asked where it's referred to, `fn_ref`): of its MIR, a `copy` of a
  `Copy` value of one, a constant of one, a call of std's that makes one of
  what it's given a reference to (`Cell::get`, `clone`, `to_vec`), a call
  of its library's that may, its trait's method resolved, and a closure it
  makes: the least answer that holds, of functions that call each other
  too, `outer` and `inner`. What the consumer changes is its `mutated` types and changed
  `Vec`s, matched as `copies.rs` matches them (`instance_of`), and what
  holds one by value (`copies::may_hold_changed`), not behind a reference,
  a pointer, an `Rc` or a std type of a lifetime. The library's own copies
  are its program's: what its own crate changes. `@rust-js/std`, compiled
  once for every consumer, still copies what any may change, and exports
  all they may reach (ADR 0561).
- **What an instance asks is the consumer's own**: its type facts (ADR
  0145), its writers' options (ADR 0137); and a write that may fail, of
  its MIR's calls as of THIR's (ADR 0187).
- **A derived serde impl's codec stays exported**, as rust-js writes it of
  its type's attributes, which no consumer sees (ADRs 0077, 0100).
- **A std type's `hash` in optimized MIR** is the writes std's impl makes,
  `HHasher.writeU32(state, digit[0] % 10)`, whatever references its MIR
  has, as its library's own is; and a derive's own type parameter is named
  without its `__`, `dropH`, as its dictionary, `HHasher`, is.
- **Refused**, as above, an instance that reads its library's private
  state: of an impl too, a destructor that counts in a private
  thread-local. The message names what it compiles and why it can't.

### Why

- **It's the JS of the program**, React's own: `react`'s JS built alone or
  as the reconciler's dependency is the same bytes, `git diff --stat
  packages/react` empty after `rust-js build --package react-reconciler`.
- **It's what Rust runs**: a consumer's `Hash` of a library's type is
  that impl's code, compiled for the consumer, as codegen compiles it;
  a copy the consumer needs is made where Rust makes it, in the function
  that reads the value, compiled with the consumer's knowledge of what it
  changes.
- **An exported dictionary's entry is what JS calls**: a JS module using
  the library would call `strToKebabCase.toKebabCase(title)` too, and the
  consumer copies none of the library's code to call it.

### Alternatives

- **Every impl's method exported by the program too**, `export function
  digitPartialEq_eq`: one export more for each impl of every crate, React's
  among them, that its upstream hasn't.
- **Every impl's method compiled by its consumer**: the docs' heck example
  became the whole of heck's `transform`, beside the app's `slug`.
- **Asking which impls a consumer needs before lowering it**: a
  dictionary is asked of evidence, a `dyn`, a drop, a default's body,
  which only lowering finds; asked there, nothing is missed, and a crate
  that asks nothing pays nothing.

### What still differs

- A library whose crate has no destructor and uses no library takes a drop
  of each of its generic impls' and trait methods' type parameters, which
  its program doesn't (`traits::may_have_destructors`), the dictionaries
  callers it can't see call; and its dictionaries have every associated
  constant, which its program's have only where generic code reads one
  (ADR 0176). React's crates, which have destructors and use `shared`,
  differ in neither.
- A library writes its derived serde impls' codecs a consumer can reach.

### What holds it

- `test/crates.test.ts`, "a library's generic function is its program's,
  alone or a dependency, and its consumer compiles what it gives more of":
  `program_form` against native Rust, its library's JS alone and as a
  dependency the same bytes, the consumer's own `keyHash`, `keyPartialEq`,
  `digitHash`, `tagHash`, `tagDebug`, `spotDebug`, `firstSpot`,
  `firstPair`, `spotsFirst_first`, `firstOf`, `outer` and `inner`; `labelDebug_fmt` without
  its dictionary, given its width; `footprint` given its size; the
  library's `digitPartialEq`, `hourDisplay` and `digitDoubled` called
  through; the instance `written` caught where it fails. And "refused where
  it reads its library's private state", of a destructor too; the pairs
  `copy_enum`, `tuple` (a `&mut` of the consumer's copy), `tree` and
  `ref_cells`; `test/cargo-workspace.test.ts`'s library that copies its
  `Point` as its program does.
- The guide's `dependencies.md` example, `strToKebabCase.toKebabCase`.
- Mutations: `instances.ts`' `instance-constant-not-copied`,
  `instance-box-read-copied`, `instance-trait-call-unresolved`;
  `copies.ts`' `library-copies-for-consumers`, `borrowing-std-type-holds`;
  `library.ts`' `library-lists-unexported`, `library-impl-not-compiled`,
  `library-exported-impl-method-compiled`,
  `library-dictionary-entry-unseen`, `library-dictionary-fails-unlisted`;
  `items.ts`' `unexported-library-item-imported`,
  `exported-copying-impl-imported`; `pipeline.ts`'
  `instance-impl-kept-unused`, `asked-instances-ignored`,
  `library-derive-kept-for-consumers`; `display.ts`'
  `instance-writer-options-library`; `traits/calls.ts`'
  `instance-facts-library`; `traits/impls.ts`' `instance-dictionary-exported`;
  `traits/dictionaries.ts`' `unexported-impl-evidence-refused`;
  `mir/dispatch.ts`' `mir-std-hash-dispatched`; `analysis/fmt_failures.ts`'
  `fmt-failing-call-of-mir-unseen`; and those whose code moved.

## Amendment: a derived serde codec is always its library's (#341)

The amendment above leaves derived serde codecs exported, as rust-js
writes them of attributes only the library sees. But a consumer that
changes a value of the type in place, the pilot's frontend its `Problem`,
asked the codec as one that may copy what it changes
(`instances::Copying`), and compiled the derive's body itself, whose
visitor isn't its own: the pilot didn't build. **A derived serde impl's
method is never the consumer's to compile** (`fn_ref`): it makes its
values anew from the text, sharing nothing the consumer changes, and is
its library's exported codec. Not another derive's, `Debug`'s, which the
consumer still compiles where its library doesn't export it. Held by
`test/crates/frontend`'s user read and changed in place, the pilot, and
the mutations `derived-codec-compiled-by-consumer` and
`any-derived-impl-not-compiled`.

## Amendment: a consumer gives an export the copy functions its contract says it takes

Case ([0262](0262-when-rust-and-js-disagree.md)): C, a wrong answer
fixed. A library's `pair<T: Copy, U: Display>` takes no `TCopy` where its
own crate's calls give none, `export function pair(t, u, UDisplay)`, as
above; but its consumer worked out the export's dictionaries as of any
function that isn't the crate's own, every `Copy` bound given one, and
called `pair(1, [2], undefined, wrapDisplay)`: `UDisplay` was `undefined`,
and `UDisplay.fmt(u)` threw where Rust gives `"w2!"`.

**A library's export's `Copy` bound is given a copy function only where
its contract says it takes one** (its parameter's `copy`, ADR 0914's
library ABI 8; `traits::bound_of`), so each dictionary after it is in its
place: `pair(1, [2], wrapDisplay)`. A consumer whose call gives a value
whose copy isn't itself compiles its own `pair`, as before, whose `Copy`
bound its own callers decide.

- `test/cross-crate-items.test.ts`, "a library's export is given the
  dictionaries its contract says it takes", run.
- Mutation `library-copy-bound-recomputed` (`traits.rs`).

## Amendment: a `Clone` bound, and a program's `Rc` of a type parameter's value, are what its callers give

Case ([0262](0262-when-rust-and-js-disagree.md)): N for the program,
which clones and drops as before; C at the boundary, where a JS caller
calls `readContext(context)` as upstream declares it.

React's reconciler reads a context's value under `react`'s unchecked
borrow of its `_currentValue`, a `RefCell<T>` (ADR 0863), and records the
context it read, as upstream's `readContextForConsumer` does:

```rust
fn read_context_for_consumer<T: Clone + 'static>(consumer: Option<&Rc<Fiber>>, context: &Rc<ReactContext<T>>) -> T {
    let value = if IS_PRIMARY_RENDERER {
        context.current_value.borrow().clone()
    } else {
        context.current_value2.borrow().clone()
    };
    let context_item = Rc::new(ContextDependency {
        context: unsafe { js::cast(js::any_of(Rc::clone(context))) },
        ..
```

`ReactFiberAsyncDispatcher`'s `readContext(CacheContext)`, of an
`Option<Rc<Cache>>`, which isn't `Copy`, needs it. It was refused twice:

- **The clone of `T` ran any code**: a `T: Clone` took its dictionary,
  `readContext(context, TClone)`, whose `clone` may be any impl, so the
  borrow held across it might meet another, which `react`'s contract says
  none can: "`react_isomorphic` borrows `ReactContext::current_value`
  unchecked .. holds a borrow of it while code that may borrow it runs".
- **The `Rc` it kept was counted**: an `Rc<ReactContext<T>>` a body owns,
  of a type parameter's value that may have a destructor, is counted
  (ADRs 0494, 0844), where `react` counts none: "this crate counts an
  `Rc<ReactContext<..>>` but `react_isomorphic` counts none" (ADR 0852).

Each answered for any `T`; every `T` the reconciler gives is an
`Option<Rc<Cache>>` or an `Option<&Unknown>`, whose clone is the value
and whose drop runs nothing. As this decision has it, a function is
compiled for what its crate's calls give it, a consumer that gives more
compiling its own.

**A `Clone` bound takes its dictionary as a `Copy` bound takes a copy
function** (`cloned_params`, beside `copied_params`, one rule,
`given_params`): only where a caller of the crate's gives a value that
isn't its own clone, or a function a dictionary may call is cloned of. A
value is its own clone (`clones_itself`) where it's its own copy, but a
closure, which may be cloned apart (ADR 0522); text, `String` too; an
`Rc` or an `Arc` the crate doesn't count, the value it shares (ADR 0023);
and an `Option` of one. Where it takes none, its clone is the value, and
it runs nothing (`CellBorrows`' `called`, asked of the body's owner), so
a borrow held across it meets none:

```js
function readContextForConsumer(consumer, context) {
  const value = isPrimaryRenderer ? context._currentValue : context._currentValue2;
```

A library's export says which of its type parameters take a `TClone`,
`clone`, as it says `copy` (library ABI 9): its consumer gives it only
those, and one whose call gives a value that isn't its own clone where
the export takes none compiles its own (`instances::gives_more`).

**A program's `Rc` of a type parameter's value is counted where a caller
gives that parameter a value whose drop runs a destructor** (its drop
parameters, `drop_params`), not wherever its bounds let it have one: the
two only grow together, as counting an `Rc` may make a value's drop run,
so they're found together until they settle (`analysis.rs`'s
`counted_with`). A library's still counts one of any type parameter its
consumers may give, as their `Rc`s share its counts (ADR 0852).

Both read the program as a whole, as rustc's codegen instantiates it:
what JS gives a `T` is a JS value, its own clone, with no destructor.

- `test/cross-crate-items.test.ts`, "a generic function's clone and its
  Rc are judged by what its callers give": a library's `Context<T>`,
  read and recorded as the reconciler does, `const value =
  context.currentValue;`, no `TClone`, no `strong`, no `$borrow`, run;
  "a library's export is given the dictionaries its contract says it
  takes", `pairClone(1, [3], wrapDisplay)` and `dup([1], { clone: .. })`,
  the library's own; and "a consumer cloning what isn't its own clone
  compiles its own of the function", its copy apart from what it was
  given.
- `test/lowering-field-cells.test.ts`, "a struct's RefCell field holds its
  value whatever else has its type": a `Cursor<Vec<String>>` read, whose
  clone takes the dictionary and is counted.
- Corpus `rc_generic_given_drops`, against native Rust: an `Rc` only
  generic code has, given a `Loud`, dropped by its last clone, and a
  counted `Rc` cloned by a `T: Clone`; `refcell_field_places` and the
  generic cases whose `TClone` is gone, as before.
- Mutations `clone-given-of-its-own-clone`, `string-clone-not-itself`,
  `counted-rc-clones-itself` (`analysis/mutation.rs`),
  `clone-bound-always-given`, `library-clone-bound-recomputed`
  (`traits.rs`), `library-clone-unwritten` (`lower/library.rs`),
  `instance-clones-unasked` (`instances.rs`), `own-clone-runs-anything`
  (`analysis/cell_borrows.rs`), `generic-rc-of-any-param-counted`
  (`analysis/rc_counts.rs`) and `rc-drop-params-unsettled`
  (`analysis.rs`).

## Amendment: std's bodies take the drops their bodies make, as a library's do

`@rust-js/std`'s generic bodies were given a drop for each type parameter
that isn't `Copy`, whether or not they drop one. Every one they drop they
take already, as a library's own function does, from its body's drops and
its crate's calls: without that seeding the committed package, the
corpus, the crates' and the workspace's JS were the same, and its
mutation, `library-generic-drops-nothing`, survived every run. The
seeding and its mutation went.

## Amendment: a binding's result copies nothing

React's reconciler is the first crate to call `shared`'s
`addObjectToProperties` and its kin, and compiled about 600 lines of its
own copy of them instead of importing them: they call
`js::object::keys(props)`, whose `Vec<String>` is a type the reconciler
pushes to, and `Copying` took a call of what the library doesn't compile
that's given a reference and gives a value of such a type for one that
may copy it, as `Cell::get` and `clone` do.

**A binding's call copies nothing** (`Copying::body_copies`): its result
is JS's own value, whatever JS made it of, and no copy of it is the
crate's to make, so the library's JS of it is what its consumer's would
be. Std's calls are asked as before.

Held by `test/cross-crate-items.test.ts`'s "a library's function of a
binding's Vec is imported, not compiled again"; mutation
`binding-result-copies`.


## Amendment: a box its optimized MIR reads through a pointer is the value it holds (#378)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a consumer given a
library's `vec![t]` with a destructor prints and drops what native Rust
does; before, the compiler panicked.

A consumer that gives a library's generic function more than its crate
does compiles its own copy from the MIR in the library's metadata, the
optimized MIR, not the MIR the crate's own bodies are read from. Two of
rustc's passes there read a box as no body of the crate does:
`ElaborateBoxDerefs` makes each `(*b).x` `_t = copy (b.0: Unique<T>) as
*const T (BoxDerefTransmute); (*_t).x`, and a debug build's UB checks
(`CheckAlignment`, `CheckNull`, `CheckEnums`) test the pointer's address
before it's read. Lowering `b.0`, a field of a box, which is the value it
holds, panicked: `fields of a type without fields`.

- **A box read through that pointer is read as the box**,
  `boxes_read_as_values` in `Mir::new`, of each body not the crate's: the
  pointer is made just before the one statement that reads through it,
  past the others made for that statement, so `(*_t)` is `(*b)` and
  nothing changed `b` between. A pointer used otherwise is left, and a
  box's field read in MIR is refused, never a panic.
- **UB checks are off in rust-js's builds**, as `UbChecks` is read off
  (ADR 0917): they test addresses JS hasn't, and fire only where a
  program's behavior is undefined. A crate's own bodies, read before those
  passes, never had them; a library's MIR now has none either.
- **Not yet**: a value moved out of its box, `*boxed`, whose optimized
  MIR then calls `<Box<T> as Drop>::drop`, and a generic method given a
  destructor where its library's takes none (ADR 0190): each a refusal.

Held by `test/crates/pairs/generic_boxed_drop/`, run as native Rust, and
`test/crates.test.ts`'s "a consumer refuses a library's generic code it
can't compile yet, and never crashes", with the mutations
`optimized-boxes-read-through-pointer` and `ub-checks-in-library-mir`.

## Amendment: a slice's `concat()` copies its items alone

The binding's amendment above lifted half of what made React's reconciler
compile its own copy of shared's `addObjectDiffToProperties`,
`addValueToProperties`, `addObjectToProperties`, `readReactElementTypeof`
and `getArrayKind`, about 620 lines with `bbN` labels, where it imports
them. The other half (pt1's trigger): `addValueToProperties` truncates an
array with std's `[&array[..1], &[Some(js::unknown("…"))]].concat()`, a
`Vec<Option<&Unknown>>`, and the reconciler pushes to such a `Vec`, its
memo cache's `memoCacheData.push(newData)`. `Copying` took a call of
std's that makes what the crate changes in place, given a reference, for
one that may copy what it was given, as `clone()` and `to_vec()` do,
whose JS a library may leave the value it was given where its crate never
changes one.

**A slice's `concat()` (`Comb::Concat`, `into_flattened()`'s too) makes
an array of its own, `[...a, ...b]` or `parts.flat()`: what it may share
of what it's given is its items**, each copied as `copies_of` copies its
item type. So a library's function of one copies what the crate changes
only where its items may hold that, `holds(item)`: `Option<&Unknown>`
doesn't, and `addValueToProperties` is imported; `Vec<u32>`s the crate
pushes to do, and a library's `[a, b].concat()` of them is compiled
again, its copies of them the crate's. Case N
([0262](0262-when-rust-and-js-disagree.md)): the crate's JS is what it
was, only where it's compiled changes.

Held by `test/cross-crate-items.test.ts`'s "a library's function of a
concat is imported, unless its items are what the crate changes", run:
the `Vec<u32>`'s push leaves what the library was given as it was;
mutations `concat-copies-its-array` and `concat-items-unasked`
(`instances.ts`).

## Amendment: a binding's static is JS's

Case ([0262](0262-when-rust-and-js-disagree.md)): none; the instance reads
the global its library's code reads.

React DOM's call of the reconciler's generic `runWithFiberInDEV`, which
it compiles itself, was refused as reading `shared::environment::__DEV__`,
"private to its library", though it's `pub` (gap rd75): the check took
every `static` no library item is of as its library's own, and an
`extern` block's static, a binding, is none, as it has no JS of its own.

**A binding's static, an `extern` block's, is read by the instance as its
library's code reads it**, `__DEV__`, a global's name; **one its library
binds to a module by a path relative to that library's file,
`#[link_name = "./stack.js#limit"]`, is refused**, as a binding called so
is: "but it reads `dep::LIMIT`, which its library binds to `./stack.js`".
Held by `test/crates.test.ts`' "a library's function its consumer compiles
is refused where it reads its library's private state" (`dep::unbounded`,
of `Infinity`, and `dep::limited`), and mutations
`instance-binding-static-private` and `instance-relative-static-read`.

## Amendment: a `()` given is the export's where nothing boxes it

Case ([0262](0262-when-rust-and-js-disagree.md)): none; the export does
what the copy would.

React DOM's `runWithFiberInDEV(ancestor, () => { .. })` compiled the
reconciler's function itself (gap rd75b), where upstream imports it from
`'react-reconciler/src/ReactCurrentFiber'`: its `()` arguments may look
like `None`, and the reconciler's own calls give none, so its export takes
each `Some` of them unboxed (ADR 0051, amended). But it makes no `Some` of
them. **A call giving what may look like `None` to a type parameter its
export takes unboxed compiles its own only where the function may make a
`Some` of it** (`instances::body_may_box`): an `Option` of what holds one
is the type of a place in its MIR or a closure's it makes, or it gives one
to its library's function that may, or to std's, which may. Not to a
binding, nor to a closure, a `dyn Fn` or a fn pointer, which take it as
their maker's code gives it. `wrapped(())`, `Some(value)`, is still the
consumer's own, boxing it.

And a copy compiled for another reason is refused where **it sets its
library's module variable**, a thread-local its library makes `export let`
(ADR 0270, amended), by `set`, `take`, `replace`, `with_borrow_mut` or
`with`: JS can't assign an import, and the copy threw `TypeError:
Assignment to constant variable.` where Rust runs, a wrong answer found
here. "but it sets `dep::LAST`, its library's module variable, which JS
lets only that module set".

Held by `test/crates.test.ts`' two-crate case `generic_given_unit`, against
native Rust, and its "a library's generic function given what may look like
None is its export where it makes no Some of it", which reads the import;
"… private state", `dep::remember(4, Loud(7))`; and mutations
`instance-unit-given-copied`, `instance-box-unseen` and
`instance-sets-library-variable`.

## Amendment: a closure holds the values it captures, takes or gives

Case ([0262](0262-when-rust-and-js-disagree.md)): none; the export does
what the copy would.

React DOM's `runWithFiberInDEV(instance, executeDispatch, event,
listener, currentTarget)` and validateDOMNesting's `runWithFiberInDEV(
ancestor, () => { .. })` still compiled the reconciler's function
themselves (gap rd85), a copy of its MIR with a `bb12:` label and six
drop flags, where upstream imports it. The port's calls give its
five-argument form `()`s, and `body_may_box` found it may make a `Some`
of them: its `CURRENT.with_borrow(|current| current.clone())` gives std,
which may, a closure, and a closure's type is of each of its maker's type
parameters, `A0` to `A4` and `T`, so it took the closure for a holder of
their values. It holds none: what it reads of them is what it captures,
and what it's given or gives back.

**A value of a closure's type holds one of its maker's type parameter
only where its captures, its parameters or its result do**
(`instances::holds_param`), so giving std that closure gives it no `()`,
and the five-argument form is the reconciler's export. A closure that
captures one, `move || callback(arg0, ..)`, still holds it; one it makes
is asked of its own body as before, and `wrapped(())` is still the
consumer's own.

Held by `test/crates.test.ts`' two-crate case `generic_given_unit`, which
now has React's five-argument form, its guard and its `with_borrow`,
called both ways, against native Rust, and its "… makes no Some of it",
which reads the import and `runWithFiberInDev(fiber, executeDispatch,
"click", "onClick", "button")`; and mutation
`instance-closure-holds-its-makers-params`.

Not yet: a copy that is needed, of one given a value with a destructor,
is printed from optimized MIR, its drop flags of each `()` it's given
kept, and its cleanup tests them nested, a branch for each, growing as
two to the power of their number (#394).

## Amendment: its optimized MIR's drop flags settled as rust-js's own (#381)

A consumer's copy of `dep::hold<T>(t: T) { let v = vec![t]; .. }`, from
the library's optimized MIR, kept the drop flag rustc's own elaboration
gave `t`, `_9 = const false; _9 = const true; Box::new_uninit(); _9 =
const false;`, as `const value = false;` and a `catch` testing it, `if
(value) { dropT?.(t); } throw error;`, where the crate's own copy, of the
drops rust-js elaborates, has neither. **Each `bool` of a body only ever
set to a constant and only tested, a `switchInt` of it alone, is one of
its drop flags** (`drops::constant_flags`), which `tidy`'s flags settle as
they settle rust-js's own (ADR 0372): its test where it's known goes, a
`try` whose `catch` then only throws on is no `try`, and a flag nothing
reads goes with its writes. Case N: what a value set only to `true` or
`false` reads is known where nothing between sets it; one lent, `&mut`,
or read otherwise is none. Held by `test/crates.test.ts`'s
`generic_boxed_drop` pair against native Rust, `dep::given`'s flag, of a
value moved on one path, still tested where it isn't known and dropped
by a panic, and the mutations `constant-flags-*` (`mir/drops.ts`).

The same settles React's `runWithFiberInDEV`'s copy (#394): its optimized
MIR's cleanup tested a flag of each argument, a `()`'s drop of nothing
joining both ways of each test, which each `catch` nested 2^k ways,
about 450 lines; none is tested now. Held by the `generic_flags_given`
pair against native Rust.

## Amendment: a copy of what the library changes too is its library's (rd606)

React's Flight client entries call react-client's exports, `close(response)`,
`getRoot(response)`, `processStringChunk(..)`, each a plain `pub fn` its
JS exports. react-server-dom-webpack compiled each itself and was refused,
as `close` reads react-client's private `DEBUG_CHANNEL_REGISTRY`: the
consumer changes a `Vec<Option<&Unknown>>` in place, a debug info list it
pushes to, and `close` reaches `ReactPromise::new` and `create_element`,
which clone one, so `instances::Copying` said `close` "copies what this
crate changes in place". But react-client changes those lists in place
too, so its JS copies each clone, as Rust does: nothing its JS shares
could be what the consumer then changes.

**A library function copies what its consumer changes, and so is the
consumer's, only of what the library doesn't change itself**: a library's
contract says what its code changes in place, `changed`, the shapes of
its `mutated` types and changed `Vec`s (`copies::Changed`), as it says what
its `Rc`s count (ADR 0852); `Copying` asks, of each body, what the crate
changes that the body's library doesn't (`Foreign::changes`). A contract
of a compiler that wrote no such fact says none, so its consumer compiles
its own as before.

- Case N ([0262](0262-when-rust-and-js-disagree.md)): the library's JS
  copies each value of a type it changes where Rust does, so importing it
  gives the consumer what its own copy would.
- Held by the `program_form` pair (`test/crates.test.ts`): `firstNotch`,
  which copies a `Notch` its library changes in place too, is imported,
  `Dep.firstNotch(notches)`, and the consumer's change of the copy is its
  own, against native Rust; `firstSpot`, of a `Spot` the library never
  changes, is still the consumer's; and by the `tree` pair, whose `Tree`
  the library changes too, and whose `potSize` reads a `Pot` it never
  changes through a `Box`, both imported. Mutations `library-changed-unshared`
  (`lower/instances.ts`) and `library-changed-unwritten` (`lower/library.ts`).

## Amendment: an operator of a library's type is called through the dictionary it exports (#468)

Since ADR 0477's amendment (ab401359e), a library writes an operator's
impl, and the other impls only generic code reads a dictionary of, only
where it's reached. rust_decimal's `impl Mul for Decimal` is reached by
its dictionary alone, `CheckedMul`'s supertrait, so its JS exports
`decimalMul = { mul: decimalMul_mul }` and no `decimalMul_mul`. A
consumer's `a * b` of two `Decimal`s, of known types, went to std's
operator, written in place, which has none for a struct: "calling
`std::ops::Mul::mul`". Its impl isn't one the consumer compiles, as its
dictionary is exported (`Foreign::compiles`), and `impl_call`'s route
through that dictionary (`dictionary_entry`) wasn't asked of an operator's
call.

**An operator's or a conversion's call of known types that resolves to a
library's impl's method its JS reaches only through the impl's exported
dictionary is called through it, `decimalMul.mul(a, b)`**, as `impl_call`'s
calls are; one its JS exports by name is called by it, as before.

- Case N ([0262](0262-when-rust-and-js-disagree.md)): the library's own
  method, called through the dictionary it exports.
- Held by `test/crates/pairs/operator_through_dictionary`, against native
  Rust, and the crate corpus's rust_decimal probe, which runs the same as
  native. Mutation `operator-through-dictionary-refused`
  (`lower/traits/calls.ts`).

## Amendment: a struct of no fields its optimized MIR names as a constant is made (#474)

Since this ADR's first amendment (9e9295569), a consumer of base64 compiles
`Engine::encode`, a trait's generic default, and the function inside it,
from base64's optimized MIR. There `&mut buf[..]` indexes by `const
RangeFull`, a value of a struct of no fields, `ConstValue::ZeroSized`,
where the crate's own MIR says the same `..` as a struct made. A value tree
of a std struct says nothing of its JS (`const_js_by` gives none of one that
isn't its fields), so base64's probe was refused, "this constant, from its
MIR".

**A struct of no fields that a body's MIR names as a constant no value
tree says is what making one is, `adt_value`'s, as of an aggregate of it**:
the JS is the same of a library's optimized MIR as of the crate's own.

- Case N ([0262](0262-when-rust-and-js-disagree.md)): a value with no
  fields, made as Rust makes it.
- Held by `test/crates/pairs/unit_struct_of_instance`, against native Rust:
  a consumer calls a library trait's generic default whose inner function
  indexes by `..` and gives a unit struct; and the crate corpus's base64
  probe, which runs the same as native. Mutation
  `mir-const-unit-struct-refused` (`lower/mir/operands.ts`).

## Amendment: a library's constructor given as a function is made in place (#476)

A function as a value asked first whether it's a library's that its
manifest doesn't list (`Foreign::unlisted`), then whether it's a
constructor. A manifest lists no constructor, so a library's was refused,
"`dep::Wrap` isn't one of the items `dep`'s manifest exports": a
consumer's `.map(dep::Wrap)` of a public tuple struct, and since this
ADR, the `.map(Self)` of smallvec's private `ConstNonNull::new` in code
its consumer compiles from smallvec's MIR, which kept smallvec's probe
from running.

**A constructor given as a function is the arrow that makes its value,
`(value) => [value]`, whichever crate's it is** (`constructor_value`), as
the crate's own is: it reads nothing of the library's JS.

- Case N ([0262](0262-when-rust-and-js-disagree.md)): the value is made
  as Rust makes it.
- Held by `test/crates/pairs/library_constructor_value`, against native
  Rust: a consumer compiling a library function whose private struct's
  `new` gives `.map(Self)`, and its own `.map(dep::Wrap)`. Mutation
  `library-constructor-value-unlisted` (`lower.ts`).

## Amendment: what a consumer compiles of a library is written where its code reaches it (#483)

Case ([0262](0262-when-rust-and-js-disagree.md)): none; what's written
and never called changes no result. A consumer's call of a library trait's
generic default of the library's own impl, base64's `Engine::encode` of
its `GeneralPurpose`, asked for the default where the call was first
lowered (`fn_ref`, `Foreign::compiles`); lowered again with it, the call
was made through the dictionary the library exports, the pair
`unit_struct_of_instance`'s `lettersEngine.encode(..)`, as this ADR's first
amendment has it. The default it had compiled was still written, and what
it calls: base64's probe held `engine_encode`, `engine_decode` and
`engine_encodeString`, their `inner`s and `encodeWithPadding`, and the
pair `engine_encode`, `inner`, `fill` and a `Debug` impl's `fmt`, which
nothing called. Only the impls compiled here were written where reached.

**What a consumer compiles of a library, an impl, its methods or a
function, is written where the consumer's own code reaches it**
(`pipeline/reached.rs`'s `written_where_reached`, of `Foreign::is_instance`):
what's reached through each item's uses from what the crate writes of its
own, as an impl compiled here was. One the crate's call reaches, by its
name or through a dictionary of the crate's own impl, is written as
before. The rule for impls is that rule now, not one of its own.

- Asking less instead, each call lowered as it will be in the round with
  its instances, would need lowering to know which of its forms it takes
  before the instances it may call exist: the form is chosen once they do.
  What's never reached is the one answer, whatever asked for it.
- `test/crates.test.ts`'s "a library's function a consumer compiles is
  written where the consumer's code reaches it": the pair
  `instance_through_dictionary`, a library's `Dog`'s generic default
  `shout` called through `dogShout.shout(..)`, neither it nor its private
  `loud` written; and `unit_struct_of_instance`, whose consumer's own impl
  `Upper` calls the default it compiles, `engine_encode`, `inner`, `fill`
  and `markerDebug_fmt` written and run against native Rust. The crate
  corpus's base64 probe runs the same as native, with none of the
  `Engine` defaults it called through the library.
- The mutation `instance-impl-kept-unused` (`lower/pipeline/reached.ts`)
  takes the rule away of impls and functions alike.

## Amendment: a closure capturing nothing made into a struct is found as its operand (#795)

Since f73daa2c5 (#736), `foreign_bodies` finds the closures a body makes in
two ways: a closure aggregate, and a closure a constant's type holds. It no
longer walks each local's type. But its `visit_rvalue` didn't walk on into
the operands of what it visited. A closure capturing nothing is a value of
no size, and where it's made into a struct's field, optimized MIR gives it
as a constant operand of the struct's aggregate:
`ScopeExitGuard { value: .., data: .., f: const ZeroSized: {closure} }`,
arrayvec's in `extend_from_iter`. That operand was never visited. Before
f73daa2c5, the local walk found the closure by the guard's type. After it,
a consumer compiling `extend_from_iter` itself (`ArrayVec<String, 4>`'s
`collect`) refused arrayvec's probe with "this closure, from its MIR".

**Each rvalue's operands are visited, so a closure is found wherever its
body names it as a constant**: in a call, and in an aggregate, a struct's
or a closure's own captures alike.

- Case N ([0262](0262-when-rust-and-js-disagree.md)): this is which bodies
  are compiled, and no observable difference follows from it.
- `test/cross-crate-items.test.ts`'s "a library's closure capturing
  nothing, made into a struct's field, is compiled by its consumer" holds
  it. There, a library's generic `cloned_all` makes a drop guard of a
  `move` closure that captures nothing. Its consumer compiles its own
  `clonedAll`, because what it clones isn't its own clone, and runs it.
  Without the fix, that crate is refused with arrayvec's message.
  `bun scripts/crate-corpus.ts --check arrayvec` holds the probe, which runs
  the same as native again.
- Mutation `closure-in-aggregate-operand-unfound`
  (`lower/analysis/foreign_bodies.ts`).

## Amendment: what copies is walked once a function (#884)

`Copying` kept the answer of a function that read none still being
asked, so of functions that call each other only the first asked was
kept, and each of the others was walked again from each way to it:
React's Flight servers spent minutes there. It's Tarjan's walk now: one
asked while it's still being asked answers what the first of them will,
and all are kept with the first's answer, each walked once (ADR 1100).
The answer is the least that holds, as before. Mutation
`copies-settled-alone` (`lower/instances.ts`).

## Amendment: another crate's closure, read from where it's made (#840)

A library's generic code its consumer compiles makes closures of the
library's own, itertools' `Product::next`'s `.map(|a| (a.clone(), elt_b))`
and `kmerge`'s `head.map(|h| Self { head: h, tail: it })`. Holding a value
with a destructor, an item of the consumer's, each was refused as "a
closure that holds part of a value with a destructor": whether a closure
takes what it holds whole was asked of rustc's `closure_captures`, which
gives the crate's own closures alone.

What a closure's drop drops was already read from where it's made, the
MIR its maker is lowered from (`drops::closure_made`), another crate's
too. So whether it takes them whole is too: `drops::types::foreign_takes_whole`
finds, in its maker's `optimized_mir`, the `Aggregate(Closure(..))` that
makes it, and it takes them whole where each operand it moves of a type
that drops is a whole local, `move _n`; a part, `move (_3.0)`, or no
aggregate found, stays refused. A library rust-js compiles refuses a
closure of its own that takes a part already, so a part here is std's,
of its MIR.

itertools' probe is refused at 9 places now, each a type fact of a type
parameter's (#934's and its kind); the two closures compile. Held by
`test/crates/pairs/closure_of_another_crate`, a library's iterator whose
`next` gives its item to `Option::map`'s closure, which `None` drops, of
the consumer's items with a destructor, against native Rust; and the
mutations `foreign-closure-captures-unread` and `foreign-closure-drop-refused`
(`lower/drops/types.ts`).

## Amendment: whether a library's item has JS the program calls is one answer (#1023)

Whether a library's item can be reached was asked five ways: of the
manifest's items, of `compiles`, of an instance, or of some of them. One
asked of `compiles` alone, as a library iterator's own `size_hint` was, and
`compiles` is false once the program has compiled the item itself, an
instance. So the first call of the library's `size_hint` made each later
generic ask of it std's `(0, None)`. Now `Foreign::compiled` is the one
answer: an item the manifest exports, one the crate compiles itself, or
one it already has. A drop's, an impl's dictionary's, `is_rust_fn`'s and
`writes_size_hint`'s questions each ask it. A trait default's call doesn't:
one whose impl's dictionary the library exports is called through it
(`through_dictionary`), the instance aside. Thirty-seven of
#1023's forty library rewrites run as native. Pair test
`library_iterator_own_size_hint`, mutation `compiled-not-an-instance`.

The last three, amended with the next two answers:

- **An impl's own method its exported dictionary holds is called through
  it wherever it's called.** The crate's own impl's method a call resolves
  to is called before std's recognitions of the call; a library's that its
  JS reaches only through its dictionary was called through it only of an
  operator, so `Picky(1, 'a').max(..)` of an own `max` and `A(1) != B(2)`
  of an own `ne` ran std's `max` and `!eq`. `dictionary_entry` is asked
  where the crate's own is, `pickyOrd.max(a, b)`, `aPartialEqB.ne(a, b)`,
  and the operator's own ask of it is gone.
- **Whether a library's impl copies what the crate changes asks its
  trait's defaults too.** `Copying::copies` of an impl walked the methods
  it writes, not the defaults its dictionary holds, so a generic default's
  `h.clone()` of a `Holder<T>` whose `T::Item` is a `Vec` the crate
  changes was the library's `(h) => h`. A derive the default calls is
  asked as std's, of the caller's types, rust-js writing it in place, and
  a field's type as the caller's bounds say, `T::Item` a `Vec`. The crate
  compiles `sTr` itself and calls it through its own dictionary, as #483
  calls a default. The contract holds: a consumer's changes are its own,
  and it compiles what of a library copies them.

- **Whether an impl's method may fail is its library's answer, wherever
  it's compiled.** A method whose dictionary entry takes `&mut self` boxed
  is one the crate compiles itself, a writer's `write_str`, and `fails`
  asked `item`, which lists no instance, so its `fmt::Error` was thrown
  where Rust gives an `Err`. It reads the manifest's listing of the impl.

Pair test `library_own_methods_through_its_dictionary`; mutations
`own-method-through-dictionary-skipped` (`lower/traits/calls.ts`),
`fails-of-an-instance-unlisted` (`lower/library.ts`),
`impl-defaults-unasked`, `derive-asked-as-its-library`
(`lower/instances.ts`) and `held-field-unnormalized` (`lower/copies.ts`).
#1023's forty library rewrites all run as native.

## Amendment: a writer given more options than its library's JS takes (#1026)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results changed:
whether a crate's writers take their `Formatter`'s options, and whether an
integer's `{:?}` may be hex, is answered crate-wide, of what its own code
formats (ADRs 0137, 0559). A library whose code passes no `{:>4}`,
`{:#?}` or `{:x?}` wrote writers that take none, and its consumer's
`{j:>4}`, `{j:#x}`, `{:#?}` of a debug builder and `{:x?}` were dropped.

The contract holds: a library's JS is its program's alone, and a consumer
compiles what it gives more of. A library's manifest says what its writers
take, `writers_take_options` and now `debug_hex`, and lists
`reads_formatter_options`, its functions that read a `Formatter`'s options:
anything done with one but its text written, `write!(f, ..)`,
`write_str` or `write_char`; `f.width()`, `f.pad(s)`, a debug builder and
`f` given on read them (`analysis/debug.rs`). A consumer whose writers
take what a library's don't compiles its own of each such writer it
reaches, an impl with one among its methods (`Foreign::writes_more`, asked
by `compiles`); a writer that only writes its text, `write!(f, "{:02}h",
..)`, which no width pads in Rust either, stays the library's.
`@rust-js/std` is not asked, compiled once for every program (ADR 0561).

A library's writer given as a value to a compiled one, `quoted` as a
`fn(&u32, &mut Formatter)`, is its import, which a pointer's call gives the
options a writer's caller gives, one that takes none leaving them, as the
crate's own is given by name; it was the arrow of its call as MIR's, which
took the `Formatter` for a `&mut` cell. A refusal of what a compiled writer
can't reach names what it was given: "a writer given a placeholder's
options, `{:>4}` or `{:#?}`, which its library's JS doesn't take", one
answer (`compiled_because`) of why a function is compiled here, which
each refusal's message asks.

Pair tests `library_writers_given_options` (a callback writer, a builder
stepped in a loop, a writer that only writes) and
`library_writer_given_debug_hex`; the private-state refusal test's
`dep::Shown`; mutations `writer-given-options-the-librarys`,
`writer-given-debug-hex-the-librarys`, `writer-reading-nothing-compiled`
(`lower/library.ts`), `formatter-options-never-read`,
`formatter-written-reads-options` (`lower/analysis/debug.ts`),
`library-writer-value-a-call-arrow` (`lower.ts`) and
`writer-refusal-unnamed` (`lower/instances.ts`). #1026's nine library
rewrites run as native.

## Amendment: what a call gives more of, by its items, its owned parameters and a pointer's drop (#1031)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results changed:
whether a call gives a library's function a value to drop its export
takes nothing for asked only its type parameters, of what they are
themselves, and whether the function's body may drop one asked only what
its calls are given that owns one. Three ways a value with a destructor
went undropped:

- **A std associated type's slot.** `second(Gen(0))`, an iterator of no
  destructor whose `Item` is a `Loud`, gave the export no drop for
  `I::Item` (ADR 0489's slot), which it takes none for. `gives_more` asks
  each of the callee's `drop_items` too, instantiated and normalized, of
  the caller's own slot or a type that drops (`items_given_drops`); and
  `body_may_drop` takes a call of a library's function whose item slots,
  instantiated where it's made, hold the parameter as one that may drop
  it, `second(made)` of an `M: Iterator<Item = T>`.
- **A type owning the caller's parameter.** A closure holding a `Vec<K>`,
  given `boxed`, gave it nothing where the caller is given a drop for `K`:
  a given type naming a parameter now drops as one of those it owns does.
- **`ptr::drop_in_place::<T>(p)`**, which drops what `p` points to, a
  `*mut T` owning none: it drops `T`, as `classify` names it.

Pair test `library_drops_its_consumer_gives`; mutations
`items-given-drops-unasked`, `owned-param-given-no-drop`,
`drop-in-place-drops-nothing`, `item-slot-callee-drops-nothing`
(`lower/instances.ts`).

## Amendment: single forms read across crates as their library reads them (#1032)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results changed.
Seven answers a consumer gave of a library's items otherwise than the
library's own compile did:

- **A class's constant is its prototype's** (ADR 0761), a library's as
  the crate's own: `Counter.prototype.limit`, where `Counter.limit` was
  undefined.
- **A dictionary's entry isn't the method it calls where its trait's
  signature names an associated type of `Self` bound by `Iterator`**, `fn
  numbers(&self) -> Self::Iter`: the entry gives generic code its form, a
  JS iterator of a `Range`, so a concrete call compiles the impl here
  (`dictionary_entry`); heck's `Self::Owned`, of no other form, is called
  through its entry as before.
- **A library's `#[track_caller]` function as a `fn` pointer** is given
  where it's defined, as the crate's own is (ADR 0427), not where it's
  made a value.
- **What a library's public function gives behind `impl .. + Clone`** is
  cloned by its consumers, so the library's compile makes it so
  (`cloned_types`' `given_out_clone`): a closure with its clone.
- **`body_may_drop` takes a `&mut` to what holds `T`**, `slot.renew()`, and
  a call given what holds `T` by value, `discard(slot)`, as one that may
  drop it: `renewed<T: Default>` and `wipe<T>` are compiled here, given a
  `Loud`, and drop the old value. The generic method given a destructor
  through `&mut self` that was refused, `Bag::<T>::clear`, now compiles
  and runs as native.
- **A closure that captures nothing, held in a local optimized MIR never
  writes**, its value of no size, is made where it's read (`&_2`), as its
  constant is, where it was `_2(x)` of nothing.
- **A library's blanket impl's entry** gives its `&mut self` box as it
  is, read of its signatures as the crate's own (`entry_unboxes`), so
  `&mut c as &mut dyn Reset` writes `c` in place.

Pair test `library_singles_across_crates`; mutations
`library-class-const-static` (`lower/items.ts`),
`entry-of-own-type-as-method`, `library-entry-unboxes-unknown`
(`lower/traits/impls.ts`), `library-located-fn-value-a-call-arrow`
(`lower.ts`), `library-given-out-clone-unseen`
(`lower/analysis/clones.ts`), `lent-mut-drops-nothing`,
`callee-given-holder-drops-nothing` (`lower/instances.ts`) and
`unwritten-closure-local-unmade` (`lower/mir.ts`). The two of #1032 left,
`class_trait_field_lent` and `refcell_borrow_while_mut`, are a library's
`RefCell` borrows a consumer may hold, which its own compile counts none of.
They're settled in ADR 0863's amendment of #1032: `refcell_borrow_while_mut` is
rd360's decided difference, and `class_trait_field_lent` is refused. Its
method `Holder::slot`, a class's trait method that gives its field, has no
JS of its own (rd304), so the library's manifest no longer lists it
(`classes::gives_its_field`); its consumer compiles it itself.

## Amendment: a closure an instance makes of an inlined function is placed where the instance is (#1029)

A library's MIR is optimized: `fold_with` calls `joined`, which rustc
inlined, so `fold_with`'s body makes `joined`'s closure itself. Its
consumer, compiling `fold_with` given closures that drop, found that
closure's body, as it finds each closure a body makes, but placed it by
its typeck root, `joined`, which no module of the consumer's holds: a
panic, "a crate given std's bodies has no module for
metamorph_lib::joined::{closure#0}". Each closure a body found makes is
placed where what made it is (`foreign_bodies`' makers,
`instance_bodies`), and `Instances::module` finds it as itself where its
root isn't an instance. What it then asks is asked as of any closure;
here what it holds is refused for now, a closure holding a value with a
destructor given away before it's called, as a refusal and no panic.
Held by the pair `closure_made_by_an_inlined_function` of
`test/crates/refused/` and the mutations
`closure-of-an-inlined-function-made-by-none`,
`closure-of-an-inlined-function-unplaced` and
`closure-placed-by-its-root-alone`.

And a library's declarations ask a `&mut`'s and a `dyn`'s form of its
item's context (ADR 1086, #619): a function's erased type parameters,
`impl FnMut(&mut U)` of `front`, were asked before its context was the
function's, of the module's, the crate root's, whose environment is no
item's, a rustc ICE, "generics_of: unexpected node kind". An item's
declaration is entered once (`Declarations::enter`): its context first,
then what it erases. Held by the pair `library_signatures_asked_of_their_items`
and the mutation `item-erased-asked-of-the-module`.

## Amendment: a function whose JS merges what its consumer compares (#1052)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results changed. A
crate decides how it represents an identity from what it reads itself. One
that compares no function's identity makes an `Rc<dyn Fn>` of a cloned
closure uncounted, the closure itself (ADR 0959). It also makes a closure
the function it passes on to: one that converts what it passes on (ADR
0605, amended), one that passes on to a scope struct's method (ADR 1043),
and one an `Rc` is made of (ADR 0959, amended). Two `Rc::new`s and two
boxes are then one JS function. A library's consumer that compares them,
with `Rc::ptr_eq` or `ptr::addr_eq`, got `true` where Rust promises
`false`. The metamorphic net's `library` rewrite found it, in
`rc_fn_ptr_eq_of_cloned`, `rc_of_an_impls_parameter_counted_as_any` and
`closure_converting_identity_read`.

The contract holds (the user's decision, 2026-11-09): a library's JS is
its program's alone, and a consumer compiles what it takes more of. The
library's manifest lists `merges_identities`: each of its functions whose
JS may give one function where Rust makes two, and each of its functions
that names one, followed through the crate's own namings (`library::merging`).
Two owners say which functions merge. `counted_rcs` gives `Counted::merges`,
the functions that make an `Rc<dyn Fn>` it doesn't count of what another
may be made of too. The lowering records `merges_identities` in a
function's `Dependencies` wherever passed-on (`passed_on.rs`, `mir/rvalues.rs`)
makes a closure that function only because the crate reads no identity.

A consumer that reads a function's identity compiles its own of each
listed function it reaches, and of an impl one of whose methods is listed
(`Foreign::merges_identities`, asked by `compiles`). It reads an identity
where it compares an `Rc<dyn Fn>` or holds a raw pointer to a `dyn Fn`:
`rc_counts::reads_fn_identity`, the same test `counted_rcs` makes
(`identity_read`). In its own compile, MIR's, each closure keeps its arrow,
as no THIR says it passes on. Its `Rc`s are counted where it compares
them, as `counted_rcs` now reads what a body of MIR alone makes another
`Rc`'s (`owners_made_others`, which reads its boxes too since #1053). Such a function doesn't hand the crate its
library's uncounted `Rc`s, so `Reached` skips its signature
(`merged_here`). For a trait's method, it checks the impl's method the
call resolves to. In generic code, where the call resolves to none yet,
it checks each other crate's impl of that method.

A consumer that compares no identity calls the library's JS as before.
The library's JS is unchanged: React's port regenerates with no new
difference. A refusal of what such a function can't reach says why it's
compiled here: "one whose JS may give one function where Rust makes two,
which this crate compares". Library ABI 21.

What it doesn't cover, refused or a difference as before:

- A value of a library's type that holds a merged function, made by
  library code the consumer doesn't reach by name. One example is a
  `dyn Trait`'s method its JS gives the crate.
- A consumer that counts an `Rc<dyn Fn(A)>` because its own compile of a
  listed function makes one counted, while another library function hands
  it one uncounted. That stays the refusal of ADR 0852.

Pair `library_identities_kept_apart`, and the test "a library's functions
that may merge what its consumer compares are compiled by that
consumer", with its refusal `identity_merged_reads_private_state`.
Mutations: `crate-reads-no-fn-identity`,
`uncounted-rc-of-a-clone-merges-nothing`, `rcs-an-instance-makes-unseen`
(`lower/analysis/rc_counts.ts`); `merging-function-hands-its-rcs`
(`lower/library.ts`); `merging-function-the-librarys`,
`merging-unlisted-of-its-callers`, `merging-closures-unlisted`,
`merging-impl-method-unresolved`, `merging-impls-of-generic-code-unasked`
(`lower/library/compiled_here.ts`); `compiled-for-identities-unsaid`
(`lower/instances.ts`); `converting-merge-unlisted`,
`scope-method-merge-unlisted` (`lower/passed_on.ts`);
`rc-closure-merge-unlisted` (`lower/mir/rvalues.ts`). What a crate
compiles of its libraries itself, and why, is one module now,
`library/compiled_here.rs`. #1052's three `library` rows of the
metamorphic net run as native, taken off.

## Amendment: the closures an instance is given are its to drop (#1051)

#1029 left `fold_with` refused, given closures that hold a value with a
destructor: "a closure that holds a value with a destructor, given away
without being called once". Its consumer compiles `fold_with` because
those closures drop, so it takes their drops as the crate's own function
does. ADR 0470, amended, answers for it as for the crate's own. A
library function that makes a closure of what holds its type parameter,
`joined`, is compiled too where it's given a value that drops, so its
closure carries the drop. The pair `closure_made_by_an_inlined_function`
now runs as native.
