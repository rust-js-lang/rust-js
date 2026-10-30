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

## Amendment (2026-10-25): an `Ok` arm that leaves, what follows reading the `Err`

**`match catch_unwind(f) { Ok(()) => return, Err(e) => { .. } }` with
code after it is `try { ..f; return; } catch (e) { .. }` and that code
after the `try`**: what follows the `Ok` arm's `if` that reads the
`Err`, or what that declares, is the `Err` arm's, the `catch`'s, as ADR
0830's `let Err(x)` takes it (`caught_run`), and the rest runs after the
`try`, where the call threw, as it ran after the `match`. React's
`recursivelyFlushAsyncActWork` writes it so, `try { flushActQueue(queue);
queueMacrotask(..); return; } catch (error) { thrownErrors.push(error);
}`, which its port's `match` was `const value = $catchUnwind(..); if
(value.TAG === "Ok") { return; } const error = value._0;`. Case A: the
same runs either way.

Held by `test/corpus/catch_unwind_ok_returns.rs`'s `flushed`; mutation
`ok-arm-leaving-err-after-kept` (`prepare/catches.ts`).

## Amendment (2026-10-28): an `Err` arm that ends the function returns

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same runs
either way.

React's throwAndUnwindWorkLoop, `match catch_unwind(|| throw_exception(..))
{ Ok(did_fatal) => { if did_fatal { .. } } Err(error) => { if let
Some(return_fiber) = .. { ..; resume_unwind(error) } else { .. } } }` at
the end of its function (gap wl6 of the port), stayed a `$catchUnwind`
result tested `.TAG === "Ok"`: its `Ok` arm can throw, so it goes after the
`try`, which needs the `Err` arm to leave, and it ended without a `return`,
the function's end being its own.

**An `Err` arm that ends the function, nothing after the `match`, leaves
it**: the `catch` ends with `return;`, in each branch of a last `if` that
goes on (`ending_in_return`), and the `Ok` arm follows the `try`: `let
didFatal; try { didFatal = throwException(x); } catch (error) { if
(returnFiber != null) { ..; throw error; } else { panicOnRootError(x); return; } }`.
Preparation tells a function's body from a block (`block_ending`); a
`match` that ends an `if`'s branch goes on, so stays as it was.

Readability (ADR 0565): `return;` grows from 9.1 to 9.2 in 10,000 lines,
each a `catch`'s that ends a function, as upstream writes `panicOnRootError(..);
return;`, where a `$catchUnwind` or `$try` result and its `.TAG` test went.

Held by `test/lowering-unwind-arms.test.ts`, "an Err arm that ends the
function returns from its catch", run where each arm runs and where the
`Ok` arm throws; mutations `err-ending-function-goes-on`,
`err-ending-return-after-its-if` (`prepare/catches.ts`) and
`err-ending-a-block-returns` (`prepare.ts`).

## Amendment (2026-10-31): its test's `else` that goes on is the program's own

The nightly at 83234ef63 found `err-ending-function-goes-on` and
`err-ending-return-after-its-if` surviving. Their test's `unwind` writes
`panic_on_root_error(x); return;` in the `Err` arm's `else`, and since
ADR 0742's amendment of 2026-10-30 that written `return` stays where it is
written, so neither the rule nor its `else` case was reached there.

The test now also compiles `last`, a `match` that ends the function and
whose `Err` arm's `else` goes on, `panic_on_root_error(x)` with no
`return`, and holds its `catch` as `} else { panicOnRootError(x); return;
}` with no `.TAG`, run where each arm runs. Both mutations are caught
again. Case ([0262](0262-when-rust-and-js-disagree.md)): A, as above.
