# 0644. A function whose result is a throwing call's `Result` is a throwing function

Status: Accepted. Extends [0035](0035-results-and-throwing-js.md), whose
throwing calls were a binding's alone, and with them
[0637](0637-a-match-of-a-throwing-call-is-try-catch.md)'s `try`,
[0639](0639-concat-text-of-any-value-may-throw.md)'s test and
[0643](0643-unwrap-of-a-throwing-call-is-the-call.md)'s `unwrap()`. As
[0638](0638-a-nullable-functions-call-is-null-already.md) is of `null`, of
throws.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, nothing the program
can observe: what was caught where the function returned is caught where
its caller reads the `Result`, of the same call, and nothing runs between.

## Context

React's `CheckStringCoercion` keeps its coercion in a function of its own,
so a stack trace shows it:

```rust
fn test_string_coercion(value: Option<&Unknown>) -> Result<String, &'static JsError> {
    js::concat_text(value)
}

fn will_coercion_throw(value: Option<&Unknown>) -> bool {
    test_string_coercion(value).is_err()
}
```

rust-js wrote `return $try(() => value + "");` and `return
testStringCoercion(value).TAG === "Err";`, and an `unwrap()` of it
`$unwrapOk(testStringCoercion(value), undefined, String)`: the throw
caught in the function and its `{ TAG }` read in the caller, where React
writes `return '' + value;`, `try { testStringCoercion(value); return
false; } catch (e) { return true; }` and `return testStringCoercion(value);`.

## Decision

**A function each of whose results is a throwing call's `Result`, given
as it is, is a throwing function: its JS returns the call's value and
lets its throw go, and a call of it is a throwing call**, by every rule a
binding's is: a `match` of it or its `is_err()` is `try` and `catch`
(ADRs 0637, 0639), its `unwrap()` the call (ADR 0643), and its `?` or a
`Result` held `$try`'s.

```js
function testStringCoercion(value) {
  return "" + value;
}

function willCoercionThrow(value) {
  try {
    testStringCoercion(value);
    return false;
  } catch {
    return true;
  }
}
```

- **Which calls throw has one answer**, `analysis::call_throws`: a
  binding's by the result it's declared with (ADR 0588), a function's by
  whether it's one of the crate's throwing functions, which
  `analysis::throwing_fns` finds once from their MIR, before any is
  lowered. Lowering's `catching` asks it of a binding's call and of the
  crate's function's alike; a call into the throwing function's own
  result is the call's value, `Expr::uncaught`.
- **Each write of its result is such a call's, into it as it is**: one
  that makes an `Ok` of its own too, `return Ok(None);` beside
  `json::parse(text)`, is none, nor one of a Rust `Result`, `text.parse()`.
- **Not a `pub` one**: JS may call it, and its JS callers read the
  `Result` its declarations say, `{ TAG, _0 }`; its `$try` stays.
- **Not a trait's method**, called through a dictionary where the impl
  isn't known, nor a generic function, whose call's `Result` is its type
  arguments', nor a closure.
- **Not one that returns its own call**: found from the least, a function
  is one where each call it returns is a binding's or a throwing
  function's found before; a recursion's base is never reached first, so
  it keeps its `$try`.

## Why

- **It's React's JS**: the coercion in its own function, its throw caught
  where the program catches it.
- **It's the same program**: a throw caught in the function and read in
  its caller is the throw caught in the caller, of the same call, with
  nothing between.

## Alternatives

- **Inlining the function**: its name, which React keeps for its stack
  trace, gone.
- **Every function returning a `Result` of `&JsError`**: one that makes its
  own `Err` or `Ok` would throw what was a value.
- **`pub` ones too**: their JS callers, which read `{ TAG, _0 }`, would
  see a throw instead.

## Consequences

- A throwing function's JS has no `$try`, and its callers each the form
  their use of the `Result` takes.

## What holds it

- `test/js-builtins.test.ts`, "a function whose result is a throwing
  call's is a throwing function": React's three, a `match` and a `?` of
  another, a `pub` one, a recursion, one that makes its own `Ok`, and one
  of a Rust `Result`, each run.
- Mutations in `scripts/mutations/lower/analysis/throwing.ts`,
  `scripts/mutations/lower/mir/structure.ts` and
  `scripts/mutations/lower/mir/calls.ts`.
