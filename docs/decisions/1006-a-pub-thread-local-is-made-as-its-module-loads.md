# 1006. A `pub` thread-local is made as its module loads

Status: Accepted. The user's decision (2026-10-28, React port gap rd51).
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
