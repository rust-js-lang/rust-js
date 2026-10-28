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
