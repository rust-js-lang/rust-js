# 0691. A `dyn Any` drops where the crate makes one of what drops

Status: Accepted. Amends [0436](0436-every-dyn-keeps-its-drop.md), whose
`dyn Any` dropped where any type in the crate had a destructor, and
[0660](0660-catch-unwind-catches-what-js-throws.md), whose `try` a
`match` of `catch_unwind` whose `Err` arm sets its binding again is too.

Case ([0262](0262-when-rust-and-js-disagree.md)): A where the same drops
run, a `dyn Any` holding nothing with a destructor dropping nothing, and
only how the JS reads changes; B for a library's consumer, under
Consequences.

## Context

React's `getIODescription` port, `catch_unwind(|| ..).unwrap_or_default()`,
was JS's `try { .. } catch { return ''; }` when ADR 0660 landed. Once its
crate ported `ReactComponentStackFrame`, whose `Drop` guards restore what
it changed, it was `$catchUnwind(..)`, then a `let kept` set by an `if` of
its `TAG`, the payload dropped in its `else`:

```js
const result = $catchUnwind(() => { .. });
let kept;
if (result.TAG === 'Ok') {
  kept = result._0;
} else {
  kept = '';
  result._0.impl.$drop?.(result._0.value);
}
return kept;
```

No commit regressed it. The payload is a `Box<dyn Any + Send>`, and a
`dyn Any`, a pair of std's trait, dropped what it held wherever the crate
had a destructor anywhere (`may_have_destructors`): a guard of
`ReactComponentStackFrame` made every `dyn Any` of `getIODescription`'s
crate one that drops, though none held a guard. `unwrap_or_default` then
drops its `Err` (ADR 0509), which no `try` can be written for.

`ReactComponentInfoStack`'s `match catch_unwind(..) { Ok(info) => info,
Err(x) => .. }` was `let result; try { result = (() => {..})(); } catch
(thrown) { .. } return result;` for the same drop, and with it gone, kept
its `$catchUnwind`: its `Err` arm's `Some(x)` shadows `x`, a shadow that's
the binding it shadows (ADR 0650), `let x = result._0; x = ..`, which the
`try` didn't take for the `Err`'s binding.

## Decision

**A `dyn Any` drops what it holds where the crate makes one of a value
with a destructor**, as a `dyn Fn` does where the crate makes one of a
closure that holds one (ADR 0521): an unsizing the crate writes, `Box<T>`
to `Box<dyn Any + Send>`, of a `T` that drops, or of a type parameter's
`T`, where the crate's types may drop; or wherever a dependency has
destructors, which may have made one (`ClosureFacts::any_dyns_drop`). Else
a `dyn Any` drops nothing, whatever else in the crate does. std makes none
of a value of the crate's but a panic's message, `&str` or `String`; what
unwound of a panic or of JS drops nothing.

```rust
pub struct Restore(u32);

impl Drop for Restore {
    fn drop(&mut self) { .. }
}

pub fn describe(value: &Unknown) -> String {
    catch_unwind(|| ..).unwrap_or_default()
}
```

```js
export function describe(value) {
  try {
    ..
  } catch {
    return '';
  }
}
```

**A `match` of `catch_unwind`'s `Result` whose `Err` arm binds a `let`,
the shadow it sets again, is the `try`'s `catch`** as one of a `const`
is: `catch (thrown) { let x = $unwound(thrown); x = ..; }`.

```js
try {
  let info = '';
  ..
  return info;
} catch (thrown) {
  let x = $unwound(thrown);
  x = x.impl.typeId() === '&js::JsError' ? x.value : undefined;
  ..
}
```

## Why

- **It's the program Rust runs**: a `dyn Any` none of whose values has a
  destructor drops nothing in Rust either. The corpus's `dyn_any_drops_by_need`
  makes one of a type parameter's `Noisy`, and `catch_unwind_drops` resumes
  one, each dropped where Rust drops it, against native Rust.
- **One rule for each `dyn` of no trait the crate wrote**, `dyn Fn`'s and
  now `dyn Any`'s: what the crate makes one of. A `dyn Display` or a `dyn
  Error` keeps the crate's answer, as std makes one of a crate's value,
  `?`'s `Box<dyn Error>` of its error, where no unsizing of the crate's is.
- **React's `try`**, in a crate with destructors as without.

## Alternatives

- **Each payload's drop, `catch (thrown) { $unwound(thrown).impl.$drop?.(..) }`**:
  a helper and a drop where React writes `catch {}`, run for what can
  hold nothing.
- **A payload apart from every other `dyn Any`**: one type, `Box<dyn Any +
  Send>`, is the payload and any other box; which is which is where it
  came from, not its type, which the drops are of.

## Consequences

- **A library's `dyn Any` drops what the library and its dependencies make
  one of**: one a consumer makes of its own value with a destructor, and
  gives the library to drop, isn't dropped there. Case B: a consumer's
  destructor in a `dyn Any` the library drops is rare, and a value not
  dropped is a leak, which Rust's safe code may do too (`mem::forget`); a
  library with no destructor of its own dropped none of them before.
- `catch_unwind_resumed`'s snapshot drops no payload: its crate makes no
  `dyn Any`.

## What holds it

- `test/lowering-unwinds.test.ts`, "a crate's destructors leave
  catch_unwind JS's try": `unwrap_or_default()` and ReactComponentInfoStack's
  `match`, in a crate with a `Drop`, each JS's `try`, run with a getter
  that throws.
- Corpus cases against native Rust: `dyn_any_drops_by_need` (a type
  parameter's `dyn Any`, a payload resumed of one), `catch_unwind_drops`.
- Mutations `any-dyn-drops-unseen`, `any-dyn-generic-unseen`
  (`analysis/closures.rs`), `any-dyn-drops-by-the-crate`
  (`drops/types.rs`), `try-catch-shadow-unbound` (`prepare.rs`).
