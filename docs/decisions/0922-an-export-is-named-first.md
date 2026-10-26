# 0922. An export is named first, and two exports of one name are refused

Status: Accepted. Found porting React's reconciler: three
ReactContextValidator tests failed. Amends
[0579](0579-a-thread-local-is-named-as-a-variable.md) (a thread-local
named after another item keeps its Rust name) and
[0019](0019-one-js-file-per-module.md)'s exports, whose names were given
to whichever of a module's items came first.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change. A JS
module importing an export by its name read `undefined`.

## Context

React's ReactFiberContext exports `emptyContextObject`, and
ReactFiberClassComponent, still JS, imports it. Its port reads it through
a private function of its name:

```rust
thread_local! { pub static EMPTY_CONTEXT_OBJECT: &'static Dict<bool> = js::dict::new(); }
fn empty_context_object() -> &'static Dict<bool> { EMPTY_CONTEXT_OBJECT.with(|e| *e) }
pub fn get_context() -> &'static Dict<bool> { empty_context_object() }
```

was

```js
export const EMPTY_CONTEXT_OBJECT = {};
function emptyContextObject() { return EMPTY_CONTEXT_OBJECT; }
```

Two rules renamed the export for a private item. ADR 0579 kept a
thread-local's Rust name where another item of its module has its
camelCase one, the private function's too; and a module's items were
named in the order they're found, functions before thread-locals, so
`#[rust_js::name = "emptyContextObject"]` made the export
`emptyContextObject$1`. Either way the JS importer's `emptyContextObject`
was `undefined`, and nothing said so: the Rust crate, its `.d.ts` and its
library contract agreed with each other, and not with what the importer
was written against.

## Decision

**What a module exports by its name is its interface, which nothing the
module keeps to itself changes.** `bindings::exported_by_name` says which
items those are: a `pub` function, `const`, `static` or `thread_local!`
of a module of the crate's own.

- **Exports are named first**, each by its own name (`name_items`): a
  private item of the same name is named apart, `emptyContextObject$1`.
- **A thread-local's Rust name, kept beside another item of its camelCase
  name (ADR 0579), is kept for an exported one only beside another
  export**: `pub static EMPTY_CONTEXT_OBJECT` beside a private `fn
  empty_context_object()` is `emptyContextObject`; `pub static SHARED`
  beside `pub fn shared()` is still `SHARED`.
- **Two exports of one module with one JS name are refused**, naming
  both, the first by a note: an importer reads one export by a name, and
  naming either apart would leave its importers reading another.

```js
export const emptyContextObject = {};
function emptyContextObject$1() { return emptyContextObject; }
export function getContext() { return emptyContextObject$1(); }
```

The `.d.ts`, the library contract and the crate's other modules take the
name from the same place (`FnInfo`), so they agree.

## Why

- **An export's importers aren't all Rust**: a port's JS modules, an
  app's, a test's, import an export by the name the original had. A name
  that moves when a private helper is added breaks them silently.
- **A private name can move**: no importer reads it, and the module's own
  reads follow it.
- **A refusal, not a rename, for two exports**: either name given apart
  would be another than its importers read; only the program can say
  which is which.

## Alternatives

- **Export the renamed item under its name, `export { emptyContextObject$1
  as emptyContextObject }`**: the right name, but an alias where JS
  writes the declaration itself, and still a renamed declaration. Rejected.
- **Name the private item first and refuse the collision**: a private
  helper of an export's name is ordinary Rust, as the port has it.
  Rejected.
- **Let `#[rust_js::name]` settle it**: what the program writes to undo
  the compiler's choice; and the attribute, on the export, didn't. Rejected.

## Consequences

- An export colliding with a name JS keeps, `delete`, or a global the
  module reads, is named apart as before (ADR 0352); this decision is of
  the crate's own items.
- `test/corpus/export_named_first.rs` against native Rust, the guide's
  example, and the known limitation of two exports of one name.
- `docs/readability.txt`'s `renamed x$1` grows, 716.5 to 716.6 in 10,000
  lines, blessed: the corpus case's private `tickCount$1`, which this
  decision names apart so its export keeps `tickCount`. No other case's
  or snapshot's JS changed.

## What holds it

- `test/lowering-module-variables.test.ts`, "an export keeps its name
  beside a private item of it": its JS, a hand-written JS importer, its
  `.d.ts`, its crate's other module, its library contract and another
  crate; and "two exports of one JS name are refused".
- Mutations: exports not named first, a thread-local's name asked of a
  private item, the refusal left out.
