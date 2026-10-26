# 0914. A library's generic function is its program's; a consumer that gives it more compiles its own

Status: Accepted. Amends [0098](0098-destructors.md),
[0289](0289-copy-bounds.md), [0300](0300-drops-by-need.md) and
[0360](0360-the-app-is-a-program.md): a library's public generic function
takes the copies and drops its own crate's calls give it, no more.

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
  would make the first the program's too.
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
