# 0637. A `match` of a throwing call's `Result` is `try` and `catch`

Status: Accepted. Amends [0035](0035-results-and-throwing-js.md), whose
`match` of a binding's `Result` was `$try` and its `TAG`, but for an `Ok`
arm that only gives the value. Amended by
[0639](0639-concat-text-of-any-value-may-throw.md): whether the call
threw, returned, `is_err()` or a `match` of `true` and `false`, is `try {
f(); return false; } catch { return true; }`. Amended by
[0660](0660-catch-unwind-catches-what-js-throws.md): `catch_unwind`'s
`Result` is `try` and `catch` by the same rule, and a throwing call's
`unwrap_or(d)` returned, `try { return f(); } catch { return d; }`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, nothing the program
can observe: the call is caught where Rust catches it and nothing else
is, the `Err` arm runs where its throw was, and the `Ok` arm where its
value was given.

## Context

A binding that may throw gives `Result<T, &JsError>`, its call `$try(() =>
f(x))`, `{ TAG, _0 }` (ADR 0035). React's `getComponentNameFromType`
catches a lazy component's `_init`:

```js
try {
  return getComponentNameFromType(init(payload));
} catch (x) {
  return null;
}
```

Its port writes the `match` a Rust developer writes:

```rust
return match init.call(payload) {
    Ok(resolved) => get_component_name_from_type(resolved),
    Err(_) => None,
};
```

and rust-js wrote `const call = $try(() => init(payload)); if (call.TAG
!== "Ok") { return null; } const resolved = call._0; return ..`.
Preparation wrote `try { return f(); } catch { .. }` only of an `Ok` arm
that gave the value as it was, `Ok(v) => v`, and of an `Err` arm that
didn't read the error.

## Decision

**A `match` of a throwing call's `Result`, its `if` of the `Ok` or the
`Err` either way and its `else` or what follows an arm that leaves, whose
arms read the `Result` by their bindings alone, is `try { .. } catch { ..
}` of the call: the `Err`'s binding is the `catch`'s, `catch (error)`.**
Only the call is in the `try`, as only it is caught in Rust:

- **What the `Ok` arm does is in the `try` where it can't throw**: what
  only makes values (`Stmt::only_makes_values`, the one answer of a
  statement's, which a panic's drops and a scope's `finally` asked of
  copies of their own), and a `return`, a `break` or a `continue` of
  what can't throw (`Expr::may_throw`). Its value is the call's, `try {
  return decodeURIComponent(text); }`, or its binding's, `try { const v
  = f(x); return $displayF64(v); }`.
- **Else it's after the `try`, where the `Err` arm leaves**, its value a
  `let` the `try` sets:

  ```js
  let resolved;
  try {
    resolved = init(payload);
  } catch {
    return null;
  }
  return getComponentNameFromType(resolved);
  ```

  React's own `try` holds `getComponentNameFromType`'s call too, which
  catches a throw of it as `init`'s; Rust's doesn't, so neither does
  this.
- **Else it's `$try`**, as before: an `Err` arm that goes on, with an
  `Ok` arm that may throw (`let doubled = match .. { Ok(v) => f(v),
  Err(_) => 0.0 }`), and a `Result` read otherwise, `?`'s `return
  result;`.

`if let Ok(v) = f() { .. }` and `let Ok(v) = f() else { return ..; }`
are the same `if` and are the same `try`. Preparation (`try_catch`)
writes it, the one place a `$try` becomes JS's `try`: lowering makes
every throwing call `$try` (`calls::catching`), and the `try` is a
rewrite of what's around it, as an ignored result's `try { f(); } catch
{}` is.

## Why

- **It's what a person writes**: React's `try`, `catch (x)` of what it
  reads, and no `{ TAG, _0 }` of a value the program never keeps.
- **Rust's semantics, exactly**: the `catch` is the `Err` arm, which runs
  of a throw of the call alone. A `let` the `try` sets is one statement
  more than React's, and the price of not catching what Rust lets
  through.

## Alternatives

- **The `Ok` arm in the `try`, as React writes it**: a throw of
  `getComponentNameFromType`, a panic among them, would be the `Err` arm's
  `None`. Rejected: Rust's `match` doesn't catch it.
- **A flag the `try` sets, `let ok = true; try { .. } catch { ok =
  false; .. }`, of an `Err` arm that goes on**: more than `$try`'s one
  line, which says it plainly.
- **Making the `try` where `$try` is made**, in lowering: the call
  doesn't know the `match` around it; MIR's arms are tidy's to shape.

## Consequences

- The guide's `decoded`, `try { return decodeURIComponent(text); } catch
  (error) { .. }`, and its `configLength`, the `let` and `try`; the
  `throws` snapshot's `sumJson`, `let numbers; try { numbers =
  JSON.parse(json); } catch (e) { .. }` and its loop after, and `uri`'s
  `try { decodeURIComponent(..); return ..; } catch (e) { .. }`; and the
  `todomvc` snapshot's `let Ok(storage) = window.local_storage() else {
  return ..; }`, `let storage; try { storage = window.localStorage; }
  catch { return []; }`. No corpus program changes. In
  `docs/readability.txt` the JS is a line longer, 88,080 to 88,081, the
  `let` of `todomvc`'s; no shape's share grows.
- `only_makes_values` answers what `drops.rs`'s `may_throw` and
  `unwinds.rs`'s `made_of_values` each answered of a statement: a
  statement of a value that can't throw, `x;`, now goes on a scope's run
  as it doesn't throw.

## What holds it

- `test/lowering-bindings.test.ts`, "a match of a throwing call's Result
  is try and catch": React's shape, a tail `match`, an `Err` arm first, a
  binding of the error, a value kept, and `$try` of an `Err` arm that
  goes on and of `?`, each run where the call throws and where the `Ok`
  arm's own code throws, which isn't caught.
- Mutations in `scripts/mutations/prepare.ts`.

## Amendment: an `Ok` arm that leaves, with nothing after it

**An `if let Ok(v) = f() { return v; }` with nothing after it in its
block is `try { return f(); } catch {}`**: its `Err` arm does nothing, and
what follows the block runs where the call threw, as it does after the
`try`. React's `describeElementType` writes it so of a lazy component,
`try { return describeElementType(init(payload)); } catch (x) {}`, which
its port's `if let Ok(name) = catch_unwind(..) { return name; }` wasn't:
the `Result` made, and its `TAG` tested, as only an `Ok` arm that goes
on, or one followed by statements that leave, were a `try`. Case N: the
same runs either way.

- `test/lowering-unwinds.test.ts`, "an if let Ok of catch_unwind that
  returns is a try whose catch does nothing", run where it throws and
  where it doesn't; mutation `ok-arm-leaving-alone-kept`.
