# 0953. Two values' truths joined are the truth of the two

Status: Accepted. Extends [0298](0298-truthy-options.md)'s `!!o`, a
value's truth, to two joined by `&&` or `||`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. `!!a && !!b` and
`!!(a && b)` give the same `bool` and read `a`, then `b` only where `a` is
truthy, alike; `||` as well.

## Context

React's `shouldConstruct`:

```js
return !!(prototype && prototype.isReactComponent);
```

Its port, `js::truthy(prototype) && js::truthy(js::get(prototype,
"isReactComponent"))`, wrote `!!prototype && !!prototype.isReactComponent`.

## Decision

**`!!a && !!b` is `!!(a && b)`, and `!!a || !!b` is `!!(a || b)`**, of
each such chain (`prepare::expr`, after its parts).

## What holds it

- `test/lowering-option-truth.test.ts`, "two values' truths joined are
  the truth of the two".
- Mutations: `truths-joined-apart`, `truths-joined-by-and`.

## Amendment: a `bool` binding's static is a `boolean` (2026-10-27)

React's `supportsSingletons && fiber.tag === HostSingleton`, its port's
`extern` static `bool`, read `!!supportsSingletons && ..`: a binding's
static of type `bool` is `Known::Boolean`, as a `bool` constant read by
its name is (ADR 0031, amended), TypeScript's `boolean`, so `&&` gives it
as it is. Held by the test of ADR 0947's amendment and mutation
`bool-binding-static-not-boolean`.

## Amendment: a call that gives a `bool` is one (rd145)

React DOM's ChangeEventPlugin, ported, writes `CAN_USE_DOM.with(..) &&
is_event_supported("input") && (..)`, and it printed
`!!(canUseDOM && isEventSupported('input')) && (..)`: tidy's conditional
asks whether its test is a `bool` (`branches::is_boolean`), and a call is
one only where its name is among the module's own `bool`s, which another
module's function, `isEventSupported` of its import, isn't. The `!!`
changed nothing, every operand being a `bool`.

**A call of a function item whose result is a `bool` is known one**
(`calls::given_by`, `Known::Boolean`), as a `bool` constant's read and a
global's are: a Rust function's, which rust-js makes a JS `boolean`, and a
binding's, which TypeScript declares a `boolean`, wherever it's of. So
`ready && isSupported(name) && (mode < 1 || mode > 9)` reads as written.

Case N: the `!!` of what's a `bool` already changes nothing.

Held by `test/lowering-bool-ternaries.test.ts`, "a call of another
module's function that gives a bool is one", run; corpus
`bool_call_of_another_module`, against native Rust; the mutation
`bool-call-result-unknown`. A regex's `test(c)`, a binding's `bool`, loses its `!!` too: the lexer's and calculator's `/^[0-9]$/.test(c) || c === "\r"`.

## Amendment: no list of JS's methods that answer a `bool`

`branches::is_boolean` also took a call of a method named `some`,
`every`, `includes`, `has`, `startsWith` or `endsWith` for a `bool`, as
rust-js writes std's: a rule of the JS's shape, beside the one above of
what its Rust gives. Since the amendment above, each such method rust-js
writes is the value of a call of std's that gives a `bool`, `any`,
`all`, `contains`, `starts_with`, which `calls::given_by` knows is one,
so the list decided nothing: emptied, every corpus case's JS and the 1,300
snapshot, lowering, JSX, React and docs tests' were the same, and its
mutation, `boolean-methods-unknown`, survived a full check (2026-10-29).
The list is gone with its mutation; what a call gives is the one answer.

Case N: as above.
