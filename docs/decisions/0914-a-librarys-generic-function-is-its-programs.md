# 0914. A library's generic function is its program's; a consumer that gives it more compiles its own

Status: Accepted. Amends [0098](0098-destructors.md),
[0289](0289-copy-bounds.md), [0300](0300-drops-by-need.md) and
[0360](0360-the-app-is-a-program.md): a library's public generic function
takes the copies and drops its own crate's calls give it, no more.
Amended (below): a library's trait impls and copies are its program's too,
and amends [0100](0100-separate-crates.md) and
[0185](0185-library-trait-defaults.md); and a consumer gives an export the
copy functions its contract says it takes.

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
