# 0298. An Option of a value never falsy is tested by its truth

Status: Accepted. Amends [0030](0030-option.md): how an `Option` is tested.
Amended: an `Option<bool>`'s `is_some_and` of its truth is `!!o`, and a
test reads `x ?? false` as `x`, and what `!` turns or a `while` tests as
an `if`'s test. Amended again, and its first decision reversed: an
`Option` of an object, an array or a function was tested `o != null`, as
every other `Option` was (below, "An `Option` is `None` where it's
nullish, whatever it holds"). **Amended a third time, which restores the
first decision and keeps that one's readability: an `Option` is tested by
what its payload's JS value may be, by its truth where that's never falsy,
against `null` where it may be** (below, "An `Option` is tested by what
its payload may be"). Amended last: a `const` of one `Option`'s presence
alone, read only by tests, is `x != null` (below). **Amended by the
user (2026-10-28), which supersedes the decisions that test an `Option`
by its truth: an `Option` is tested against `null`, whatever it holds,
one output form** (below, "One form: an `Option` is tested against
`null`"). An `Option<bool>`'s `Some(true)`, the value's truth, is kept.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An `Option` is its value or `undefined` (ADR 0030), and whether it's there
was `o != null`. react.dev tests an object that may be missing by its truth:

```ts
{error && <div>..</div>}
<SandpackConsole visible={!error} />
if (rawError && rawError.message === '..') { .. }
if (message.firstLoad) { .. }
```

## Decision

**An `Option` whose value is never falsy, an object, an array or a
function, is there where it's truthy: `is_some()` is `!!o`, `is_none()` is
`!o`, and a test of one, an `if let Some(..)`'s or a `map`'s, `o`.** A
test reads `!!o` as `o`, and so does JSX where it shows `o && <el />`,
which renders nothing for `undefined`. `!!o ? o.m(x) : undefined` is
`o?.m(x)`, as `o != null ? ..` is. (Amended: the `Option`'s `map` says so, which knows
it's never falsy; JS's `!!s` alone may be text's, `!s.is_empty()`, which
`""` fails, and `s ? s.m() : undefined` is no `s?.m()`.)

**A `const` only tests read is read by its truth too**: `const hideContent
= !!error || ..` read only by `if`s, `?:`s and `!`, is `error || ..`, as
react.dev writes it. One read as a value keeps its boolean.

**`o == Some(true)` of an `Option<bool>` is `!!o`**, which a test reads as
`o`: `true`, `false` and `undefined` test alike either way.

- An `Option` of a number or a string, which may be `0` or `""`, is tested
  against `null` still, and so is one of a binding's `unknown` or `any`.
- `is_some_and` and `is_none_or` keep `o != null` and `o == null`: they say
  "there, and" and "missing, or", as react.dev writes `rawError == null ||
  rawError.title === ..`.

## Why

- **It's the JavaScript a person writes** for a value that may be missing.
- **It's the same program**: a value never falsy is falsy only where it's
  `undefined`.

## Consequences

- The react.dev port accepts, by name, the explicit tests of an `Option` of
  a number, `h == null || h === 0` for `!h`: Rust's is exact for `NaN`.

## Amendment: what `filter_map` keeps, and `Boolean`

What `filter_map` and `find_map` keep of a value never falsy is tested by
its truth too, and a callback that only tests its argument's truth,
`(x) => !!x`, is `Boolean`, which is `!!x` and reads no other argument:
`.map(f).filter(Boolean)`, as react.dev's runESLint and TeamMember write
it. A number's or a string's `Option` keeps `(item) => item != null`. And `?` of one is
`if (!o) return;`, as react.dev's runESLint writes `if (!error)`.

## Amendment: an `Option<bool>` that's `Some(true)`, tested

React's `ReactNoopUpdateQueue` asks whether it warned of a key,
`js::dict::get(did_warn, &key).is_some_and(|&warned| warned)`, which was
`const option = didWarn[key]; option != null && option`, read twice, where
upstream writes `if (didWarnStateUpdateForUnmountedComponent[warningKey])`.
And `flag.unwrap_or(false)` in a test was `if (flag ?? false)`.

- **`is_some_and` of the truth of what it holds** (`tests_its_truth`, an
  `Option<bool>`'s `|&w| w` or `|v| js::truthy(Some(*v))`), its `Some`
  not boxed, nothing to drop, is `!!o`: `o != null && o` read once, as
  what's not there is falsy too. A test reads it as `o`, as it does `o ==
  Some(true)`'s `!!o`.
- **A test reads `x ?? f` as `x`, `f` a falsy constant** (`Expr::tested`,
  the one answer to what a test reads): `undefined` and `null` are falsy,
  as `false`, `0` and `""` are. `if (flag)` of `flag.unwrap_or(false)`.
- **What `!` turns is read as a test**, wherever it is, `const off =
  !flag`, and so is a `while`'s test, `while (flags[i])`, as an `if`'s
  is.

Case: A, as before: each reads true where the other does, for any value.

Held by `test/corpus/option_bool_tested.rs` against native Rust, and
the mutations `is-some-and-truth-twice` (`scripts/mutations/lower/combinators.ts`),
`tested-coalesce-kept` (`js/expr_walk.ts`), `falsy-constant-false-truthy`
(`js/make.ts`), `not-operand-untested` and `while-test-untested`
(`prepare.ts`).

## Amendment: an `Option` is `None` where it's nullish, whatever it holds

The user decided it, porting React's `ReactHooks`: `useEffect(create,
deps)` warns where `create == null`, and the port's `if __DEV__ &&
create.is_none()` printed `if (__DEV__ && !create)`. A JS caller's
`useEffect(false)` then warned where upstream doesn't. "Never falsy" is
what Rust's type says, not what a JS caller gives: an exported function
taking `Option<&Function>`, `Option<&Item>` or an `Option` of an untagged
enum of objects is given whatever JS passes, `false`, `0` or `""` too, and
Rust's `None` is JS's `null` and `undefined` alone (ADR 0030).

- **Every `Option`'s presence is `o != null`, and its absence `o ==
  null`**, of an object, an array, a function, a `dyn Fn`, a `NonZero`, a
  serde_json `Value` or an untagged enum of them too: `is_some()`,
  `is_none()`, `if let Some(..)`, a pattern's `None`, `?`, `map`'s test,
  what `filter_map` keeps, `(item) => item != null`, and a codec's and a
  `Hash`'s test. One owner says it, `std_types/option.rs`'s `present` and
  `absent`, which ask nothing of the payload's type; `never_falsy`, which
  did, is gone, and so is JSX's reading of `!!error` as `error`.
- **An `Option<bool>` keeps its truth** where a test asks for `Some(true)`,
  `o == Some(true)` and `is_some_and(|&w| w)`, `!!o` and, in a test, `o`
  (the amendment above): that's the value's truth, which `false` and
  `undefined` share, not its presence. Its `is_some()` is `o != null`.
- **`x != null ? x : y` is `x ?? y`**, and `x == null ? y : x` too, of an
  `x` that does nothing (`tidy/branches.rs::conditional`), as `x ? x : y`
  was `x || y` (ADR 0744): `match find(..) { Some(s) => s, None => blank
  }` is `state ?? blank`.
- **JSX reads `error != null` in place**, as it read `!error`: a test of
  a value that reads alike, `{error != null && <div>..</div>}`, no `const
  condition` before the element. Its reading of a `!` alone, which only an
  `Option`'s `!error` was given, is gone with it.
- **A `let`'s binding a `return` follows is set first**, as it was of
  every `Option` whose test is `!= null` (`tail_returns`): `const loaded$1
  = loaded; if (loaded$1 == null) { return; }` where `if (!loaded) {
  return; } const loaded$1 = loaded;` was.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a falsy value a JS
caller gives in place of an object is Rust's `Some`, and `!o` would take
it for `None`. The output is longer than react.dev's and React's,
`descriptor.get != null` for `descriptor.get`, `error != null && ..` for
`error && ..`: correctness first, and it's what React itself writes where
it means "missing", `create == null`.

### What it costs

The react.dev port and React's ports read `x != null` where upstream
wrote `x` or `!x`, which their committed JS changes to. `!!o ? .. :
undefined`'s `o?.m(x)` is `o != null ? ..`'s, as before.

### What holds it

`test/lowering-options.test.ts`, "an Option of an object or a function is
None where it's nullish", run in Node with `false`, `0`, `null` and
`undefined` given; "? of an Option tests it against null"; "a truth test
as a callback is Boolean"; `test/jsx-children.test.ts`, "JSX tests an
Option of an object against null"; `test/jsx-react-dev.test.ts`, "react.dev's
Preview tests its error in place"; `test/lowering-branches.test.ts`, "a match
that only gives a value is one expression" (`state ?? blank`). Mutations
`option-present-truthy`, `option-absent-falsy` and `filter-truth-map-chained`
(`std_types/option.ts`), `question-truth-test` (`std_types/result.ts`),
`filter-map-truth` (`combinators/adapters.ts`), `jsx-null-test-captured`
(`jsx.ts`) and `present-value-coalesce-kept` (`mir/tidy/branches.ts`).
What a value's truth still is, `js::truthy(v)`'s `!!v`, is held where the
`Option`'s had held it: `test/lowering-written-branches.test.ts`, "a
value's truth or else a bool is a bool part by part", and
`test/jsx-children.test.ts`, "a prop under a child's truth test is its
value there".

## Amendment: an `Option` is tested by what its payload may be

The user decided it, refining the amendment above: React's ports read
`componentInfo.owner != null`, `while (owner != null)` and
`descriptor.get != null` where upstream writes `!componentInfo.owner`,
`while (owner)` and `descriptor.get`, as JS is written of an object that
may be missing. The test is what the payload's JS value may be:

- **A payload that's never falsy, an object, an array, a function, a
  class's instance or a symbol, is tested by its truth**, as the first
  decision had it: `is_some()` is `!!o`, `is_none()` `!o`, and a test of
  one, an `if let Some(..)`'s, a `while let Some(..)`'s, `?`'s, a `map`'s,
  `o`; `filter_map` keeps it by `Boolean`; JSX shows it `o && <el />`.
  A `NonZero`, never `0`, and a serde_json `Value`, an object or
  `"Null"`, are never falsy too.
- **A payload that may be falsy is tested against `null`**: a value of
  any shape, `js::Unknown` or what a `dyn IntoUnknown` is given, a
  number, text, a `char`, a bigint, a type parameter's `T`, and an
  untagged enum one of whose variants may be. Its `Some(0)`, `Some("")`,
  `Some(false)`, `Some(NaN)` and `Some(0n)` are there, which `!o` would
  take for `None`: a wrong answer, never a reading.
- **An `Option<bool>` keeps its truth** where a test asks for
  `Some(true)` (the amendment of `Option<bool>`, above); its `is_some()`
  is `o != null`.
- **One owner says it**, `std_types/option.rs`'s `never_falsy`, which
  `present` and `absent`, the test of being there, `filter_map`'s kept
  item, `?`'s, a codec's, a `Hash`'s and JSX's shown child all ask. It
  asks the representation's owners what a type is, `is_object`,
  `is_callable` and `untagged`, and what `#[rust_js::types]` says of a JS
  object's struct, `declarations::written_falsy`: a keyword's but
  `object`'s and `symbol`'s, `""`'s, and a union's of one may be falsy
  (`Written::may_be_falsy`), so `js::Unknown`, `PhantomData<JsObject>`
  but written `unknown`, is tested against `null`. JSX asked
  `is_js_object` beside it, which took `js::Unknown` for an object and
  showed `filter(|v| js::truthy(Some(*v))).map(..)` as `value && <i />`,
  rendering `0`: it asks `never_falsy` alone now, `!!value && <i />`.
- **What the amendment above added that's still right is kept**: `x !=
  null ? x : y` is `x ?? y` (`tidy/branches.rs::conditional`), of a
  number's, text's or any value's `Option` now, as `x ? x : y` is `x ||
  y` of an object's; and JSX reads `height != null` in place, as it reads
  `!!error`, so neither is a `const condition` before the element.

Case: A ([0262](0262-when-rust-and-js-disagree.md)). A value never falsy
is falsy only where it's `undefined` or `null`, so `!o` and `o == null`
hold alike of every value its type has, and a payload that may be falsy
keeps `o == null`, which is exact. A JS caller that gives `false` or `0`
where an exported function's `Option<&dyn Fn()>` or `Option<&Item>` is
asked gives a value its type doesn't have, as TypeScript's declaration
says, `(() => number) | undefined`: it's taken for `None`, as a binding
is trusted as declared (the guide's semantics, "The JavaScript
boundary"); a value of any shape, `Option<&js::Unknown>`, is what's
tested against `null` whatever JS gives. So React's `useEffect` port,
`create: Option<&Create>` of a `dyn Fn`, is `if (__DEV__ && !create)`
again where upstream writes `create == null`: they differ only for a
caller giving `false`, `0` or `""` in place of the function its type asks,
which upstream's Flow type, `() => (() => void) | void`, doesn't let it.

### What it costs

The amendment above's corpus, snapshot and guide changes are undone where
a payload is never falsy, and kept where it may be: `o == null` of a
number, text or any value is unchanged. React's ports go back to
upstream's forms, `if (!componentInfo.owner)`, `while (owner)`. `!null`,
a `None` constant tested by its truth, is `true`, as `!false` is
(`js/make.rs`'s `unary`), where `null == null` was. `docs/readability.txt`
is blessed for the corpus case below alone: its four `?`s are `return;`,
a shape the gate counts, which its tests of `?` are. Its `while let
Some(p) = stack.pop()` is `while ((p = stack.pop()))` of an object, and
`while ((x = stack.pop()) != null)` of a number or text (ADR 0856).

### What holds it

`test/corpus/option_falsy_payloads.rs`, against native Rust: `Some(0)`,
`Some("")`, `Some(false)`, `Some(NaN)` and `Some(0n)` of a value of any
shape (`JSON.parse`'s, `Number`'s, `BigInt`'s), of numbers, text, bigints
and `bool`s, and `None`, through `is_some()`, `is_none()`, `if let`,
`match`, `?`, `filter_map`, `while let` of `pop`, beside objects, arrays
and functions tested by their truth. `test/lowering-option-truth.test.ts`:
"an Option of a value of any shape is tested against null, of an object
by its truth", "an Option of a binding whose written type may be falsy is
tested against null", "JSX shows an Option of a value of any shape where
it's not null" and "JSX tests a number's Option against null in place",
each run with those values given. The first decision's tests are back,
`test/lowering-options.test.ts`'s "an Option of an object is tested by
its truth" among them, and `test/lowering-branches.test.ts`'s "a match
that only gives a value is one expression" has text's `name ?? blank`
beside an object's `state || blank`. Mutations: the first decision's,
back (`std_types/option.ts`, `combinators/adapters.ts`, `jsx.ts`,
`js/stmt_walk.ts`, `std_types/result.ts`); `truthy-unknown`,
`truthy-any-shape-corpus` and `truthy-written-unread`
(`std_types/option.ts`); `written-number-never-falsy`,
`written-union-every-falsy` and `written-empty-literal-truthy`
(`declarations/written.ts`); `jsx-js-object-never-falsy`,
`jsx-null-test-captured` and `negation-spilled` (`jsx.ts`);
`not-falsy-constant-kept` (`js/make.ts`); and
`present-value-coalesce-kept` (`mir/tidy/branches.ts`).

## Amended (2026-10-25): a truth test of being set is one, so `p?.x` again

Case: A ([0262](0262-when-rust-and-js-disagree.md)), as above.

Once an object's `Option` was tested by its truth, React's
`startTransition` port read `match prev { Some(prev) => prev.types, None
=> None }` as `prev ? prev.types : undefined`, where it had been
`prev?.types`: `js/make.rs`'s `cond` folds `x != null ? x.f : undefined`
to `x?.f`, and of `!!x` it can't, as `!!s` of `!s.is_empty()` is no test
of being set, and `s?.toUpperCase()` gives `""` where Rust gives `None`.

**The test `present` and `absent` make of a payload never falsy says it's
one of being set**: `Expr::presence` writes `!!o` or `!o` marked
`Known::Presence`, and `null_test`, the one answer to what a test of being
set is, reads it as `o != null`, or `o == null`. Each fold of a set test
then holds of it, as it's the same answer for each value the type has: `p
? p.x : undefined` is `p?.x`, `p?.[k]` and `p?.m(x)` too, and `if (!x) {
x = e; }` is `x ??= e`. A `!!s` no `Option`'s presence made is no set
test, so text's truth keeps its conditional. `x ? x : y`, of a truth test
of `x` alone, is still `x || y` (ADR 0744), asked before `??`
(`tidy/branches.rs::conditional`), so `state || blank` stays.

Held by `test/lowering-option-truth.test.ts`, "an Option of an object read
in a conditional is an optional chain", run with an object, `undefined`,
and text's `""`; and the mutations `presence-unmarked` (`js/make.ts`),
`any-truth-a-null-test` (`js/stmt_walk.ts`) and `truth-or-coalesced`
(`mir/tidy/branches.ts`). `test/lowering-comments.test.ts`'s match arm of
many comments read away, `prev?.types`, has them before its statement, as
ADR 0444's amendment says, and a new arm, `((prev.types ?? 0) + 1) >>> 0`,
keeps them before its value in the conditional.

## Amendment: `Expr::cond` alone reads a presence test as a chain (#241)

The `Option`'s `map` turned its `!!o` into `o != null` before
`Expr::cond`, so the conditional would chain `o?.m(x)`. That was a second
reader of one rule: `Expr::presence` marks the `!!o` it makes
`Known::Presence`, and `Expr::cond` reads that mark through `null_test`
since the amendment above. The `map`'s copy changed nothing, and the
nightly's `truthy-option-map-unchained` survived. It is gone, along with
that mutation. A `filter`'s truth test, `!!s` of text, carries no mark,
so it keeps its conditional. `filter-truth-map-chained` now marks it and
is caught.

## Amendment: `presence_of` is gone; `null_test` answers its caller (#272)

`js/stmt_walk.rs`'s `presence_of` read `!!x` as a test of being set even
without the `Known::Presence` mark. That made it a second answer, next to
`null_test`, and no program reached its extra arm: `Expr::presence`
always marks the `!!x` it makes. Its one caller,
`a != null && Array.isArray(a)` → `Array.isArray(a)`, now asks
`null_test(.., true)`, and the arm's mutation `presence-truthy-unread`,
which survived, is gone with it. `text-truth-chained` still breaks the
optional-chain fold the same way, inline, and is still caught.

## Amendment: a const of one `Option`'s presence alone is `x != null`

React's completeWork writes `const isTimedOutSuspense = nextState !==
null; if (isTimedOutSuspense) { .. }`. Its port, `let is_timed_out_suspense
= next_state.is_some();`, printed `const isTimedOutSuspense = nextState;`:
`prepare.rs`'s `tested_constants` reads a `const` only tests read by its
truth, as the amendment of react.dev's Preview decided for `const hide =
error || !ready`, and so a name meant to hold a `bool` held the object.

The user decided (2026-10-27, cw3): **a `const` only tests read that is
one `Option`'s presence alone, `!!x` (`Known::Presence`), is `x !=
null`**: a test still, and a `bool`, as upstream writes it. A chain,
`!!error || !ready`, keeps its parts read by their truth, `error ||
!ready`, as before; one read as a value keeps its `!!`. ADR 0262 case N:
`x != null` and `x`'s truth answer the same of an `Option` never falsy
(ADR 0298), and the const is read only by tests.

Tidy's `negations`, which rebuilt each `!` read in place with
`Expr::unary`, dropped the `Known::Presence` mark `Expr::presence` gives
`!!x`; it keeps it now, where the rebuilt is the same `!`, so the mark
reaches preparation.

Held by `test/lowering-options.test.ts`'s "a const only tests read",
`const failed = error != null;`; mutations `tested-const-presence-truth`
(`prepare.ts`) and `negation-presence-dropped` (`lower/mir/tidy.ts`).

## Amendment: one form, an `Option` is tested against `null` (2026-10-28)

The user's decision (React port gap tr1). React's ports printed `if
(!retryQueue)`, `if (offscreenState)` and `if (updateQueue &&
updateQueue.events)` where upstream writes `retryQueue === null`, and two
forms of one test, by truth of an object and against `null` of a number,
were one rule's two outputs. **An `Option` is tested against `null`,
whatever its payload: `is_some()` is `o != null`, `is_none()` `o ==
null`**, and so is each test of one, an `if let Some(..)`'s, a `while let
Some(..)`'s, `?`'s, a `map`'s, a pattern's `None`, what `filter_map` keeps,
`(item) => item != null`, a codec's, a `Hash`'s and JSX's shown child,
`{error != null && <div />}`.

```rust
pub fn retry(retry_queue: Option<&Queue>) -> u32 {
    let Some(queue) = retry_queue else { return 0 };
    queue.size
}
```
```js
export function retry(retryQueue) {
  if (retryQueue == null) {
    return 0;
  }
  return retryQueue.size;
}
```

- **One owner, `Expr::presence`** (`js/make.rs`), which every test of
  being set is made by and `null_test` reads; `std_types/option.rs`'s
  `never_falsy`, `present` and `absent`, which asked what the payload
  may be, are gone, and so are `declarations::written_falsy`,
  `Written::may_be_falsy`, JSX's reading of `!!error` as `error`, and
  `Known::Presence`, the mark that read `!!o` as `o != null`. Tidy's
  `tested_options` reads `x != null` of `const x = t ? v : undefined` as
  `t`, as it read `!!x`.
- **What a value's truth still is, is kept**: an `Option<bool>`'s
  `Some(true)`, `o == Some(true)` and `is_some_and(|&w| w)`, `!!o`, and
  in a test `o`, the value's truth, not its presence (above); and
  `js::truthy(v)`'s `!!v`, `filter(|v| js::truthy(Some(*v)))`, which says
  "truthy" in the program.
- `x != null ? x : y` is `x ?? y`, of every `Option` now, where `x ? x :
  y` was `x || y` of an object's: `state ?? blank`.

Case ([0262](0262-when-rust-and-js-disagree.md)): None. Of an object, an
array or a function, `!o` and `o == null` hold alike of every value its
type has; of every other payload `o == null` was the form already. A JS
caller's `false` where an `Option<&dyn Fn()>` is asked is now `Some`, as
Rust's `None` is only `null` and `undefined` (ADR 0030), where it was
`None`: a value its type doesn't have, given past its declaration.

### What it costs

The corpus, the snapshots, the guide's examples and `docs/readability.txt`
read `x != null` and `x == null` where an object's `Option` was tested by
its truth, about 170 files, each corpus case run against native Rust
again. React's ports read upstream's `!== null` as `!= null`, its test
still, the form for `null` and `undefined` alike.

### What holds it

`test/lowering-option-truth.test.ts`, its tests now of one form, run in
Node; `test/corpus/option_falsy_payloads.rs` against native Rust; and the
mutations `option-presence-by-truth` (`js/make.ts`) and
`tested-option-null-unread` (`lower/mir/tidy/options.ts`).

## Amended (2026-10-28): what a test-only `const` drops the `!!` of now

Since an `Option`'s test is `x != null` whatever it holds (the amendment
of 4b30cd7cd), no `const` is one `Option`'s presence as `!!x`: the
check that wrote one `x != null` is gone with its mutation,
`tested-const-presence-truth`. What a `const` only tests read still
drops the `!!` of is a `bool` JS gives as a number, `const both = a & b
|| !ready;`, kept `!!(a & b)` where it's read as a value too; held by
`test/lowering-bits.test.ts`'s "a const of a bool's bits only tests
read", and `bindings.test.ts`'s `cn("a", (a & b || ready) && "b")`.
