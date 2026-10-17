# 0442. A web API that throws for what a program can't prevent is a `Result`

Status: Accepted. Extends [0035](0035-results-and-throwing-js.md); amends
[0440](0440-a-binding-needs-no-allow.md)'s consequence.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): whether a page may keep
data decides what a program does, and a throw rust-js can't catch, a
panic, would end it.

## Context

A JS call whose binding returns a `Result` runs in a `try`, its `Err` what
it threw (ADR 0035). webapi, generated from WebIDL, returned none: WebIDL
doesn't say what throws, only each spec's prose does. So a page in a
sandboxed frame, as the playground runs TodoMVC, panicked at
`window.local_storage()`, which throws a `SecurityError` there, and
TodoMVC kept its own `localStorage` bindings to get an `Err` (ADR 0440).

## Decision

**A web API that throws for what a program can't prevent or check first
returns a `Result`, as Rust's I/O does; one that throws only of a misuse a
program can avoid, an invalid selector, panics, as Rust's indexing does.**
Which is which is one list, `THROWS` in `webapi/generate.ts`, by an
interface and its member's JS form, each with why, which its `Err`'s doc
says:

| Member | `Err` |
|---|---|
| `Window.local_storage()`, `session_storage()` | a `SecurityError` where the page may not keep data: storage turned off, a sandboxed frame, a `file:` or `data:` page |
| `Storage::set_item` | a `QuotaExceededError` where it's full |
| `Document::cookie()`, `set_cookie` | a `SecurityError` in a sandboxed document, or one of an opaque origin |

```rust
let Ok(storage) = window.local_storage() else { return Vec::new() };
```

```js
const value = $try(() => window.localStorage);
if (value.TAG !== "Ok") {
  return [];
}
```

- **Held**: the generator fails where the list names a member webapi
  doesn't bind, so none is quietly no `Result`.
- **TodoMVC** uses webapi's `localStorage`; its own `JSON` bindings stay
  until the playground has serde, which needs its derive, a procedural
  macro, in the browser.
- **The test**: `test/throws.rs`'s `kept` reads `localStorage` and is
  `None` where it's refused; `compiler.test.ts` runs it.

## Why

- **It's the Rust a server's I/O is**: what the environment may refuse is
  an `Err` the program handles, not a crash.
- **A misuse stays a panic**, as Rust's: a `Result` on every call would be
  `unwrap()`s a reader skips past.

## Consequences

- A program that read `local_storage()`, `cookie()` or `set_item`
  directly handles their `Result`.
- What else throws so is added to the list as it's met, with the spec's
  reason; the list starts with storage and cookies.
