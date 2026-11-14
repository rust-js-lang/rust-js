# 1157. A global a host may lack is read by a call

Status: Accepted (2026-11-15, #1862, the lead's decision). Amends
[0269](0269-global-scope.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Each read is the
bare global the program names, at the place Rust reads it; where a host
lacks it, JS's `ReferenceError` is where Rust's call is, as of any
binding's API a host lacks.

## Context

The vercel UI kernel's `activeElement(doc = document)` (vercel-labs/ui@220c1a0,
`packages/core/src/foundation/platform`) reads `document` only where no
`doc` is given, so a server that renders with a document of its own never
reads the global. webapi declared `document` a `static`,
`webapi::document: &'static Document`, so the port wrote what clippy asks
of a value that's there, `doc.unwrap_or(webapi::document)`: `doc ??
document`'s arguments read first, `document` read where a `doc` was
given, a `ReferenceError` in Node's server render where upstream runs
(Codex's SSR regression, rust-js-lang/vercel-ui-sdk-in-rust). Its interim
form was a `match`, which no Rust engineer writes for a fallback.

A Rust `static` is a value that always exists, and a read of one runs
nothing, so its type said what isn't so of `document`, `window`,
`navigator`, `history` and `performance`: a host may lack each, a server
the first four, an old worker the last.

## Decision

**A global that isn't `[Exposed=*]` is read by a call, a getter of its
scope's module, as each of that scope's attributes already is
([ADR 0269](0269-global-scope.md), rd195)**: `window_global::document()`,
`window_global::window()`, `window_global::navigator()`,
`window_global::history()` and `global::performance()`, each JS's bare
read. Only `webapi::console` and `webapi::global_this` stay statics,
which every JS host has (`GLOBALS` in `webapi/generate.ts`).

```rust
use webapi::{Document, window_global};

pub fn active(doc: Option<&'static Document>) -> bool {
    let doc = doc.unwrap_or_else(window_global::document);
    doc.has_focus()
}

pub fn has_document() -> bool {
    js::is_defined(window_global::document)
}
```

```js
export function active(doc) {
  doc ??= document;
  return doc.hasFocus();
}

export function hasDocument() {
  return typeof document !== "undefined";
}
```

- **A lazy fallback is `unwrap_or_else`**, the form clippy asks for of a
  call, read only where it's needed.
- **A feature test is of the getter, uncalled**,
  `js::is_defined(window_global::document)`, as of `device_pixel_ratio`;
  and `js::is_defined(Window::new_error_event)` asks of the window's
  getter, as it asked of its static: `member_of_global` finds a root
  module's getter named as its type, as it found the static.
- **The five run nothing** (`#[rust_js::runs_nothing]`, ADR 0867): each
  spec's getter steps return the global's own object, HTML's document,
  window, navigator and history getters, High Resolution Time's
  `performance` getter, and call nothing. So a read of one reads the same
  each time, as a member's that runs nothing does, and
  `function::bind(performance().measure, Some(performance()), ..)` is
  still `performance.measure.bind(performance, ..)`. No extended
  attribute picks them out, `[LegacyUnforgeable]` being `location`'s and
  `top`'s too and `history` and `navigator` having none, so the
  generator lists them (`QUIET_GLOBALS`), each with its IDL type and the
  spec's steps, and fails, as of `THROWS`, where one is gone from WebIDL
  or of another type; the compiler names none.

## Why

- **Rust's type says what JS's host does.** A call may fail where the
  host lacks it; a static never does. A value that's there everywhere,
  `console`, `globalThis`, keeps a static.
- **One way to read each global.** The scope's other attributes were its
  module's functions already; these five were the exception.
- **The JS is upstream's**, `doc ??= document`, the read where upstream
  reads it.

## Alternatives

- **Keep the statics and lower `unwrap_or(webapi::document)` lazily:**
  Rust evaluates an argument first; lowering it lazily would change when
  a read runs of a getter that may throw, and every other `unwrap_or`.
- **An `Option` of each global:** `typeof` tests do that already, and a
  page's code would unwrap a value that's there wherever it runs.
- **Statics of all but `document`:** a server lacks the window and its
  navigator and history as well; one rule from the IDL covers all five.

## Consequences

- `test/webapi-globals.test.ts`: Codex's SSR acceptance, the globals
  deleted, the fallback reads `activeElement` of a given document only,
  `hasDocument()` false, the fallback's read a `ReferenceError`.
- Every consumer migrated in one push: examples, the guide, tests,
  `wasm/web`, and the React port (pin moved).
- The same test's `measurer`: React's `function::bind(Performance::measure,
  Some(&performance()), (performance(), ..))` is still
  `performance.measure.bind(performance, ..)`.
- `test/webapi-coverage.test.ts` runs the generator, so a listed getter
  gone or retyped fails it (probed: `Window.history` typed `Navigator`,
  and a misspelled `Window.navigator`).
- Mutations global-getter-reads-differ (`binding_calls.rs`) and
  global-getter-not-the-types (`member_of_global`, held by
  `webapi-bindings.test.ts`'s window constructor test).
- The React port's JS is unchanged: its fallbacks were feature tests
  already.
- Guide: interop/global-scope.md says which are statics and shows the
  feature test and the lazy fallback.
