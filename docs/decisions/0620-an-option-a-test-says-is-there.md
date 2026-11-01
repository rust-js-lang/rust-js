# 0620. An `Option` a test says is there is written as it is

Status: Accepted. Extends [0578](0578-a-nullable-result-where-its-written.md)
and [0564](0564-a-std-body-is-tidied-as-the-apps-are.md)'s decided tests.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: what's written, not
what it does. `x ?? d` is `x` wherever `x` isn't `null` or `undefined`.

## Context

React's `getIteratorFn` returns the iterator method it finds, or `null`:

```rust
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn get_iterator_fn(maybe_iterable: Option<&'static Unknown>) -> Option<&'static Unknown> {
    // ..
    if js::type_of(maybe_iterator) == "function" {
        return maybe_iterator;
    }
    None
}
```

A `#[rust_js::nullable]` result of another `Option` is `null` for
`undefined` where it's written (ADR 0578), so rust-js wrote `return
maybeIterator ?? null;` under `if (typeof maybeIterator === 'function')`,
where upstream returns `maybeIterator`: what the test says is a function
is never `null` or `undefined`.

## Decision

**Where a test is known to hold, or to fail, an `x ?? d` of a variable
it says is there is `x`**: `typeof x === "function"`, of any kind but
`"object"` and `"undefined"`, a test of `x` against a constant, or `x !=
null`, holding; `x == null` failing, after `if (x == null) { return null;
}`. It's what `decided_tests` knows already, the test of an `if` that
holds through its `then` and fails through its `else` and what follows a
branch that leaves (ADR 0564), up to what may change what it reads; now
it decides a `??` there too (`present_in`), of a variable nothing
assigns.

```js
export function callable(value) {
  if (typeof value === "function") {
    return value;
  }
  return null;
}
```

`implies` answers whether one test says another, and gains its one
missing case, `x != null` saying `x != null`.

## Why

- **A value's JS is what it is where it's read**: `?? null` says the
  value may be `undefined`, which the test it's under says it can't be.
- **One place knows what a test decides**, `decided_tests`: a `??` it
  decides is the same knowledge as an `if` it decides.

## Alternatives

- **Not write `?? null` where the result is written** (`nullable_written`):
  lowering doesn't know the tests a value is under. Rejected for asking
  of a value what only the code around it knows.
- **Write the Rust another way**, `maybe_iterator.filter(..)`: what's
  natural is the test upstream writes, and rust-js is what changes.

## Consequences

- A `??` of a variable inside a closure isn't one this decides: none is
  there yet when it runs, and the test of `shadowed` holds that a
  closure's own `value ?? 7` stays.
- Only a variable's `??`: one of a field, `a.b ?? d`, stays.

## What holds it

- `test/lowering.test.ts`, "an Option a test says is there is written as
  it is": `callable`, `logged` after a failed `== null`, and a closure's
  `(value) => value ?? 7`, run in Node; and "a nullable result is written
  where it's made", whose `if (flag && first != null) { return first; }`
  had `?? null`.
- Mutations `present-coalesce-kept`, `present-coalesce-failed-test-unread`.

## Amendment: what's read of a nullable field is never `undefined`

React's `findFirstSuspended` keeps a fiber's `return` in a variable and
stores it in another's, and ReactFiberRoot takes a root's transition
types:

```js
const returnFiber = node.return;     // upstream: the same, then
sibling.return = returnFiber ?? null; //   sibling.return = node.return
const old = root.transitionTypes;    // upstream: const claimed = ..
root.transitionTypes = null;
return old ?? null;                  //   return claimed
```

A nullable place stores `null` for `None` (ADRs 0772, 0841), so what's
read of one is `null` or the value. **A read of a `#[rust_js::nullable]`
field there where it's made is never `undefined` (`Known::Defined`), and a
`const` of it, `x ?? null`, is `x`**: `representation::project` marks the
read, `fixed_place_of` keeps what's known of a place it makes once, and
tidy's `nulled_as_they_are` writes `x` of a `const x` its body declares
once, of what `may_be_undefined` says never is. Not of an optional one
(ADR 0810), absent until it's set, `undefined` read: `banner ?? null`
still. Case A, as above: `x ?? null` is `x` wherever `x` isn't
`undefined`; a field JS made against its declaration, `T | null`, is the
JS's to keep.

- `test/lowering-nullable.test.ts`, "a nullable field's value a variable
  holds is stored as it is", run, an optional one's too.
- Mutations `nullable-field-read-may-be-undefined`,
  `optional-nullable-read-defined` (`lower/representation.ts`),
  `fixed-place-facts-dropped` (`lower/mir/assign.ts`),
  `nulled-kept-coalesced` (`lower/mir/tidy/values.ts`).

## Amendment: an `unwrap()` of what a test says is there is its value

React's `findFirstSuspended` takes a fiber's sibling once its loop's test
says it's there, upstream's `node.sibling` read again:

```rust
while node.sibling.borrow().is_none() { .. }
let sibling = node.sibling.borrow().clone().unwrap();
```

Its port's JS was `$unwrap(node.sibling)`, after `if (node.sibling ==
null) { .. }`, whose branch leaves: the `unwrap()` never panics there.

**Where a test is known to hold or to fail (`decided_tests`), a
`$unwrap(p)` of a `p` it says is there, `p != null`, that reads the same,
a variable or a field, is `p`**; of a statement that may change what the
test reads, only the value it reads first, `const sibling =
$unwrap(node.sibling)`. A test through a field is decided where the
variables it reads change elsewhere too, as anything that does something
disturbs it (`disturbs_any`): a `node` the loop assigns later is
`node.sibling` still until then. Case N: the value is the same, and the
panic can't happen.

- Three corpus cases decide a test through a field's tag they couldn't
  before, `if (tmp.TAG === "Ok")` after `tmp.TAG !== "Err"`, their drops
  each once; `loop_assigned_tests`' `out.push(top)`.
- `test/lowering-nullable.test.ts`, "an unwrap of a field a test says is
  there is the field", run, a call between keeping its `$unwrap`.
- Mutations `field-test-undecided-where-its-variable-changes`,
  `unwrap-of-tested-kept`, `unwrap-past-a-call`
  (`lower/mir/tidy/branches.ts`).

## Amendment: a test against a `None` given says it isn't there

A nullable function's `if js::strict_equals(ref_prop, None) { return
ref_prop; }` printed `if (refProp === undefined) { return refProp; }`,
giving `undefined` where the function gives its `None` as `null`: a wrong
answer, a JS caller's `=== null` failing. Decided tests read `x ===
lit`, of a constant not `null` or `undefined`, as saying `x != null`, and
the `None` given to `strict_equals` is a `GivenUndefined` (ADR 0330),
which wasn't counted as `undefined`. **A test of `x` against a `None`
given says nothing of its being there** (`branches.rs`'s `implies`): its
`refProp ?? null` stays. Case R: results changed, now Rust's. Held by
`test/lowering-nullable.test.ts`, "a nullable result tested equal to a
None given is null", run in JS, and the mutation
`implies-given-none-a-literal` (`tidy/branches.ts`).

## Amendment: a `?? null` of a value its test shows defined is the value

Its port's `return if !js::strict_equals(ref_prop, None) { ref_prop } else
{ None };` was `refProp !== undefined ? refProp ?? null : null`, where
React writes `refProp !== undefined ? refProp : null`: a nullable result's
`x ?? null` (`Storage::written`) of a `const` the body declares once, in
a conditional's branch or an `if`'s where its test is `x !== undefined` or
`x != null`, is `x`, which isn't `undefined` there
(`values.rs`'s `nulled_as_they_are`, `shown_defined`). Case N: `x ?? null`
is `x` where `x` isn't `undefined`. Held by
`test/lowering-written-branches.test.ts`, "an if of returns the program
wrote stays an if", `test/lowering-nullable.test.ts`, "a nullable result
tested equal to a None given is null", and the mutations
`tested-defined-still-nulled`, `tested-defined-if-still-nulled` and
`tested-defined-where-it-isnt` (`tidy/values.ts`).

## Amendment: what a nullable function gave is never `undefined`, held or not (2026-10-26)

React's unsafe_markUpdateLaneFromFiberToRoot holds what it finds while more
runs:

```rust
let root = get_root_for_updated_fiber(source_fiber);
mark_update_lane_from_fiber_to_root(source_fiber, None, lane);
root
```

`get_root_for_updated_fiber` is `#[rust_js::nullable]`, and so is the
function returning `root`. A call of a nullable function written straight
to its result, or to a nullable place, was as it is (ADRs 0638, 0841); held
in a variable first, it was `return root ?? null;`, as a variable may be
`undefined`.

**What a call of a function that gives `null` for `None` gives is never
`undefined`, wherever it's kept** (`Known::Defined`, `calls::given_by`):
`const root = lookup(id);` is a `const` of what's never `undefined`, so
`nulled_as_they_are` returns it, and a nullable field stores it, as it is:
`return root;`, `holder.root = root;`. Not a trait method's, whose impl
says nothing of it (ADR 0841, amended).

- Case A: `null ?? null` is `null`, and a value is itself.
- Held by `test/lowering-nullable.test.ts`, "a variable of what a nullable
  function gave is returned and stored as it is", run in JS, and the
  mutation `nullable-call-kept-undefined` (`lower/mir/structure.ts`).
