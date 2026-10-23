# 0645. `js::is_defined` of a static is given the static itself

Status: Accepted. Amends [0597](0597-whether-the-host-has-a-global-is-is-defined.md),
whose static was given by reference, `js::is_defined(&webapi::performance)`,
and [0624](0624-is-defined-of-a-path-of-bindings.md), whose path is asked of
a static too. Amended by [0647](0647-type-name-as-react-writes-it.md): the
static's value isn't read before the test.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: what's asked of the
host, part by part, as JS asks it.

## Context

React's `CheckStringCoercion` asks whether the host has
`Symbol.toStringTag` before it reads it:

```js
const hasToStringTag = typeof Symbol === 'function' && Symbol.toStringTag;
```

Its port wrote `js::is_defined(js::symbol::to_string_tag)`, the static as
itself, as a function binding is given, `js::is_defined(message_channel::new)`,
and rust-js refused it, "of what isn't a JS global's binding": a static
was asked of only by a reference to it, `&webapi::performance`, which is
a second spelling where a function's is the binding itself.

## Decision

**`js::is_defined` of a static is given the static itself**, as a function
binding is: `js::is_defined(webapi::performance)`, its value read through
its address in MIR, followed to the static. Its path asks each part (ADR
0624):

```rust
js::is_defined(js::symbol::to_string_tag)
```

```js
typeof Symbol !== "undefined" && typeof Symbol.toStringTag !== "undefined"
```

**A reference to a static is refused**, naming the one way: "`js::is_defined`
of a static is given the static itself, `js::is_defined(webapi::performance)`,
not a reference to it". Its reference is one `&` more than its type, the
static's type the one rustc says.

## Why

- **One spelling, the one a binding has**: what's asked of is named as
  itself, a function or a static.
- **It's the path the program names**, `Symbol.toStringTag`, part by part,
  as JS asks a host that may lack `Symbol`.

## Alternatives

- **Both, the static and a reference to it**: two spellings of one ask.
- **Upstream's `typeof Symbol === 'function' && Symbol.toStringTag`**:
  a test of the value's truth, which `is_defined` doesn't ask; the port
  reads the symbol after, where it's there.

## Consequences

- `js::is_defined(&webapi::performance)` is refused, written
  `js::is_defined(webapi::performance)`; the guide and the tests say so.

## What holds it

- `test/lowering-bindings.test.ts`, "js::is_defined is whether the host
  has a binding's global": a static as itself and a static of a path;
  `test/lowering.test.ts`, "js::is_defined of what isn't a global's
  binding is refused": a reference to a static refused, by its message.
- Mutations in `scripts/mutations/lower/mir/calls.ts`: the static's value
  unread, a reference accepted.
