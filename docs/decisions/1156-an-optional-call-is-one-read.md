# 1156. An optional call is one read of its method, `js::optional_call`

Status: Accepted (2026-11-15, #1859, the form the lead approved, with its
three cases by what the method gives).

Case ([0262](0262-when-rust-and-js-disagree.md)): R where they differ:
Rust's evaluation order, its arguments evaluated before the call.

## Context

Upstream's composedClosest (vercel-labs/ui@220c1a0,
`packages/core/src/foundation/platform/platform.ts:64`) calls a hop's
`closest` where it has one, a `Text` node or a `ShadowRoot` having none:

```ts
const found = (hop as Element).closest?.<T>(selector)
```

rust-js had no form for it. The guide's `js::is_function((hop,
Element::closest))` then a call reads a getter twice and skips a
non-function instead of throwing; `function::call` of `js::method(..)`
reads the function's own `call` where one has it. Codex's regression: one
`get`, then the call with the receiver its `this`, no `.call` read.

## Decision

**`js::optional_call(path, args) -> Option<R>`**, of the path
`js::method` and `js::is_function` take, typed by its method's signature
(`MethodCall`, beside `Method`, without its `'static` bounds, as the call
is given what it's given for the call alone):

```rust
js::optional_call((hop, Element::closest), (selector,)).flatten()
```

```js
hop.closest?.(selector)
```

- **One read, its `this` kept**, a thrown error as it is: the one maker of
  `?.`, `Expr::optional_call`. A member that's there and no function is
  called, JS's `TypeError` a panic.
- **Rust's order.** The value, then the arguments, run before the call;
  lowering holds the value's code before the arguments' as it does
  anywhere, and each argument that runs code is held,
  `const arg = f(); log.closest?.(arg)`, as JS's `?.` wouldn't run one
  where the method isn't there. Arguments aren't lazy: where one mustn't
  run without the method, test with `js::is_function` and call in the
  branch, the program's own two reads.
- **What it gives, by the method's result `R`:**
  1. one that can't look like `None`, the call itself;
  2. one whose `None` is `null`, a binding's `T | null` (`NonePlace::
     CallResult`), its `Some(None)` boxed, `$someDefined(..)`, and
     `flatten()` of that the call itself, `None`'s `null` and `undefined`
     alike (ADR 0950, amended);
  3. one that may be `undefined`, refused, as `?.` gives that of a method
     that isn't there too: read it once with `js::method`.
- **A method alone.** A getter's binding, or another form's, is refused:
  read it as it is.
- **One way.** `js::is_function` stays the program's own `typeof` test,
  `js::method` the function read as a value, and this the one call of a
  method where it's there.

## Consequences

- `test/js-optional-call.test.ts`: the JS above, its declaration, Codex's
  getter-and-own-`call` regression, a hop without `closest`, one whose
  `closest` finds none and one that finds, a non-function member's
  `TypeError`, the receiver, the argument and the read in Rust's order;
  a getter and a method of `()` refused.
- Mutations optional-call-arguments-in-place, optional-call-nested-unboxed,
  optional-call-of-undefined-results, optional-call-of-a-getter and
  optional-call-flattened-boxed.


## Amendment: the documented Node lookup is an unsafe signature choice (#1899)

The initial example was tested only with an Element receiver. Its documented
Node receiver passed rustc's MethodCall bound but was refused by the shared
strict binding-path check, even though absence is the point of this call.

The safe optional_call binding retains its checked native receiver. The separate
unsafe optional_call_unchecked explicitly asserts the final receiver's method
signature: a normal return must obey its selected native argument/result
contract. This is the standard checked/unchecked API distinction, as in get and
get_unchecked, rather than two APIs for the same guarantee. The user approved
preserving the SDK's unchecked JavaScript boundaries on 2026-11-15. Neither API
adds a runtime class/realm check.

defined_path remains the single owner of native path validation. Its explicit
PathEnd policy permits only the final optional method's receiver to be
asserted by this unsafe call. Ordinary method reads, mandatory path steps,
native method form, argument binding and result/nullability rules retain
their checks. Getters and undefined-valued method results remain refused.
Rust still evaluates receiver and arguments in its order (ADR 0262 case R);
the new receiver support accepts no additional semantic difference.

The focused Node case checks generated source/declarations, absent/null/found
results, a non-callable property, single getter read, exact receiver, an own
call getter that must not run, and thrown-value identity. Rust rejects a call
without an unsafe boundary; ordinary extraction still rejects an unrelated
receiver. Full validation is pending the coordinated machine window.
