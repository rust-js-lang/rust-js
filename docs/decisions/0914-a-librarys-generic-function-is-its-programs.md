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
and a binding's result copies nothing; and a slice's `concat()` copies
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
