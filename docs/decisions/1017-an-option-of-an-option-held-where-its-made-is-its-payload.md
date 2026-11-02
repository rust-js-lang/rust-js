# 1017. An `Option` of an `Option` held where it's made is its payload

Status: Accepted: the user's representation (2026-10-29), of each
value whose every write and read can be told, the box kept elsewhere. Amends
[0051](0051-generic-options.md) (a `Some` of what looks like `None` is
boxed). Follows [1015](1015-what-holds-an-identity-is-never-eq-of-its-contents.md),
whose Consequences left it open.

Case ([0262](0262-when-rust-and-js-disagree.md)): none: each of the three
values Rust tells apart is its own JS value, and each read is of what its
writes made, so every answer is Rust's.

## Context

React's `processDispatchQueueItemsInOrder` keeps the fiber of the
listener before, `Fiber | null`, or none yet:

```js
let previousInstance;
for (let i = dispatchListeners.length - 1; i >= 0; i--) {
  const {instance, currentTarget, listener} = dispatchListeners[i];
  if (instance !== previousInstance && event.isPropagationStopped()) {
    return;
  }
  ..
  previousInstance = instance;
}
```

Its port says the same in Rust, the field `#[rust_js::nullable]`:

```rust
let mut previous_instance = None;
for DispatchListener { instance, .. } in dispatch_listeners.iter().rev() {
    if previous_instance != Some(instance) && (event.is_propagation_stopped.get())() {
        return;
    }
    ..
    previous_instance = Some(instance);
}
```

`previous_instance` is an `Option<&Option<Rc<Fiber>>>`, whose `Some` may
hold a `None`. ADR 0051 boxes that `Some`, `$some(instance)`, so `None`
and `Some(None)` differ, and since ADR 1015 the compare read

```js
const right = $some(instance);
if (!(previousInstance == null || right == null
    ? previousInstance == right
    : $someValue(previousInstance) == $someValue(right)) && ..) {
```

A whole-type representation, every `Option<Option<T>>` with `Some(None)`
`null`, can't hold: generic code boxes, `$some` of its `T` as it runs,
and std's compiled bodies make and read `Option`s of their items, an
iterator's `next()` of a slice of `Option`s among them. A value crossing
from one to the other would be read wrongly, a box taken for `Some(Some(box))`
or a `null` for `None`.

## Decision

**A local `Option<Option<T>>` or `Option<&Option<T>>` whose `T` never
looks like `None` (`representation::nested_payload`), written only `None`,
`Some(..)` or another such, and read only by another such, by its
discriminant, by what its `Some` holds, or, through a shared reference, by
an `==` of two such, is its payload: its `None` `undefined`, its
`Some(None)` `null`, its `Some(Some(v))` `v`, never boxed.**

```js
let previousInstance;
..
  if (previousInstance !== instance && event.isPropagationStopped()) {
    return;
  }
  ..
  previousInstance = instance;
```

- **Where it's decided.** `mir/patterns.rs`'s `mir_nested`, a body's
  fact `OptionWrites::nested`, as its other `Option` facts are: each
  mention of such a local or its reference that isn't one of those is
  out, and so is each that's copied to or from, or compared with, one
  that's out. A parameter, the result, one with a destructor, one given
  to a function, returned, captured, matched against a constant, or
  borrowed `&mut` keeps ADR 0051's box.
- **What reads it.** `mir/assign.rs` writes `None` `undefined` and a
  `Some` what it holds, its `None` `null` (`Storage::NullForNone`), as is
  of a nullable field's or a reference to one (`stored_given`, which now
  follows a shared reference and a reborrow), `previousInstance =
  instance`, else `instance ?? null`; `mir/rvalues.rs` tests it `x !==
  undefined`; `mir/places.rs` reads what its `Some` holds as the value
  itself; `mir/dispatch.rs`'s `nested_equality` compares two by `===`
  where their payload's `==` is `===`, through `Box`es and uncounted
  `Rc`s, and else `a === undefined || b === undefined ? a === b : <a == b
  of the inner `Option`s>`.
- **One representation per value.** A local's values never meet code that
  boxes: what's out keeps the box all through, so no value is ever read
  in the other form.

## Why

- **It's upstream's JS**, `let previousInstance;`, `previousInstance =
  instance`, `instance !== previousInstance`, from the Rust a person
  writes for it.
- **It's exact.** `undefined`, `null` and the payload's values are three
  kinds no `T` that never looks like `None` makes, so `===` and `!==
  undefined` answer as Rust's `==` and `is_some()`.

## Alternatives

- **Every `Option<Option<T>>` so**, the user's representation as a type's:
  wrong where generic code meets it (above). Rejected in favor of the
  local, which is that representation wherever it can be told.
- **Convert at each generic boundary**, a box made of `null` going in, a
  `null` of a box coming out: every call, closure and std body returning
  or taking an `Option<P>` of a `P` that's an `Option`, `filter_map`'s
  too. Rejected: a second representation of one type the whole program
  would have to agree on.

## Consequences

- The port writes `Some(instance) != previous_instance` for upstream's
  operand order, `instance !== previousInstance`.
- `matches!(last, Some(None))` reads `last !== undefined && last == null`.
- `last == Some(None)`, a constant MIR promotes, keeps the box: the
  constant isn't a local made here. (Amended below: it's `last === null`.)

## What holds it

- Corpus `nested_option_held_as_payload`: React's loop over listeners of a
  nullable field and of a plain one, `match`, `if let` and `matches!` of
  one, payloads compared by `===`, through a `Box`, and by `$eq`, and one
  given to a function, which keeps its box, against native Rust.
- `test/crates/pairs/library_eq`: a consumer's loop over a library's
  listeners, whose field is nullable, `previousInstance !== instance` and
  `previousInstance = instance`, against native Rust.
- `docs/guide/interop/types.md`'s example, held by `test/docs.test.ts`.
- Mutations `nested-options-boxed`, `nested-option-mentions-unchecked`,
  `nested-option-copies-unlinked` (`lower/mir/patterns.ts`),
  `nested-option-none-null`, `nested-option-some-none-undefined`
  (`lower/mir/assign.ts`), `nested-option-test-loose`,
  `shared-ref-given-unknown`, `reborrow-given-unknown`
  (`lower/mir/rvalues.ts`), `nested-option-eq-loose`,
  `nested-option-eq-nones-unseen`, `nested-option-box-unpeeled`
  (`lower/mir/dispatch.ts`) and `nested-option-payload-unboxed`
  (`lower/mir/places.ts`).
- React's port with DOMPluginEventSystem installed: the seven DOM event
  suites pass, 9 suites, 401 tests.

## Amendment: `Some(None)` is `=== null`

`matches!(last, Some(None))` printed its two tests, the outer's and the
inner's, `last !== undefined && last == null`. Of one place, `x !==
undefined && x == null` holds only of `null`, and `x === undefined || x !=
null` of all but it, whatever `x` is: `Expr::bin` folds them, `last ===
null` and `last !== null`, as a person writes the test of the one value
`Some(None)` is. A test of `undefined` beside another place's of `null`,
`last !== undefined && plain == null`, stays two.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same truth of
every value.

Not yet: `last == Some(None)` and `last.is_some()` give the `Option` to
`PartialEq::eq` and `is_some(&self)`, so `last` keeps its box there
(`$eq(last, { $someNone: 0 })`); reading them as the tests they are is the
representation's to extend.

Held by `test/corpus/nested_option_held_as_payload.rs` against native
Rust, `test/lowering-nested-options.test.ts`, and the mutations
`null-apart-unfolded` and `null-apart-of-two-places` (`js/make.ts`).

## Amendment: `is_some()`, `is_none()` and `== Some(None)` are its tests (2026-10-29)

The amendment above left `last.is_some()` and `last == Some(None)` keeping
the box: each gives `&last` to a function, `Option::is_some(&self)` and
`PartialEq::eq`, so `mir_nested` took it out, and the whole local with it:

```js
last = $some(item);
..
if (last !== undefined) { .. }
if ($eq(last, { $someNone: 0 })) { .. }
```

**A call of `Option::is_some` or `Option::is_none` of one through a shared
reference, and an `==` or `!=` of one with a constant `Some(None)`, are
reads of it too** (`mir_nested`): `last !== undefined`, `last ===
undefined`, `last === null` and `last !== null` (`dispatch.rs`'s
`nested_equality`). The constant is told by its value tree
(`operands.rs`'s `nested_some_none`), of the promoted `&Some(None)` MIR
gives a local, through its reborrow; another constant, `Some(Some(2))`,
keeps the box, compared as it was. And `Expr::bin` folds `x !== undefined
&& x != null`, `if let Some(Some(v)) = last`'s test, to `x != null`, and
`x === undefined || x == null` to `x == null`, the second test saying the
first:

```js
last = item ?? null;
..
if (last !== undefined) { .. }
if (last === null) { .. }
if (last != null) {
  out += String(last);
}
```

`== None` isn't one: clippy's `partialeq_to_none` asks for `is_none()`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Each of the three
values is its own JS value, so each test is Rust's answer.

Held by `test/corpus/nested_option_tests.rs` against native Rust, and the
mutations `nested-option-is-some-boxed`, `nested-option-some-none-eq-boxed`
(`lower/mir/patterns.ts`), `nested-option-is-some-flipped`,
`nested-option-eq-some-none-undefined` (`lower/mir/dispatch.ts`),
`nested-constant-any-some` (`lower/mir/operands.ts`) and `null-said-twice`
(`js/make.ts`).

## Amendment: the inner `Option` given on keeps the box (2026-11-01)

rd489, P0, a wrong answer. A read of what its `Some` holds was the payload
itself, so the inner `Option` of `Some(None)`, read whole, was `null`:

```rust
pub fn first(v: Option<&'static Unknown>, f: &dyn Fn(Option<&'static Unknown>)) {
    let o = if v.is_some() { Some(v) } else { Some(None) };
    if let Some(v) = o {
        f(v);
    }
}
```

```js
const o = v != null ? (v ?? null) : null;
if (o !== undefined) {
  f(o);
}
```

`first(undefined, f)` called `f(null)`, where Rust's `None` given to JS
is `undefined` (ADR 0030); an `Option<u32>`'s or an `Option<&str>`'s
alike. And a `Some` of what may be `undefined` is written `?? null`,
so it lost which it was: React's Flight port's `Some(chunk.value)`, a
value of any shape, `chunk.value ?? null`, resolved a chunk whose value
is `undefined` with `null`, which `js::classify` tells apart, `Kind::Null`
(ADR 0676). Four values, `None`, `Some(undefined)`, `Some(null)` and
`Some(v)`, can't be three JS values; only the box, `{ $someNone: 0,
$value: null }`, holds the fourth.

**The inner `Option` read whole, `(x as Some).0` or through its
references, `*(x as Some).0`, is a read of one held as its payload only
where it's only tested** (`mir/nested_options.rs`'s `only_tested`): its
discriminant read, its `Some`'s payload read, or it, or a shared reference
to it made once, given to `is_some()` or `is_none()`, each of which reads
`null` as the `None` it is. Read any other way, given on, kept, returned or
compared, the local keeps ADR 0051's box, which holds what was given:

```js
const o = v != null ? $some(v) : $some(undefined);
if (o != null) {
  const v = $someValue(o);
  f(v);
}
```

`if let Some(inner) = last { inner.is_some() }` is still `last != null`,
and its payload's reads, `if let Some(Some(v)) = last`, its `===`
compares and its `match`es are as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior: a callback is given the `None` Rust gives, `undefined`,
and a value of any shape the one it was.

Not yet: `$some(v)` where `v != null` was just tested is `v` itself, and
`const v = $someValue(o); f(v);` reads the box once where it's used;
both are the box's readability, P3. React's port's JS, each package built
before and after, is unchanged: its Flight client, whose `Some(chunk.value)`
found it, isn't ported yet.

Held by `test/js-any-values.test.ts`'s "the None an Option of an Option
gives of any value is undefined" (the repro, Flight's `Some(chunk.value)`
and `js::classify` of it, in Node), `test/lowering-nested-options.test.ts`'s
"the None an Option of an Option gives on is undefined" (numbers, text, a
struct through a reference, a number read through one, `Some(&value)`, a
returned one and a tested one, in Node),
corpus `nested_option_inner_none` (numbers, text and structs kept, looked
for and shown, against native Rust), and the mutations
`nested-inner-given-on-unboxed`, `nested-inner-through-reference-unchecked`,
`nested-inner-copied-on-unchecked`, `nested-inner-test-boxed` and
`nested-inner-test-reference-unseen` (`lower/mir/nested_options.ts`).

`mir_nested`, which was `mir/patterns.rs`'s, is `mir/nested_options.rs`'s
now, with `only_tested`, and so are its mutations above that were
`lower/mir/patterns.ts`'s, but `nested-options-boxed`, of its call.

## Amendment: an `Option` of a value that's `null` but never `undefined` (2026-11-02)

rd547, P3. React's Flight client keeps its chunk cache as a `Map<string,
null | Promise<any>>`, `null` once a chunk has loaded:

```js
const entry = chunkCache.get(chunkId);
if (entry === undefined) {
  ..
} else if (entry !== null) {
  promises.push(entry);
}
```

Its port's value is an untagged enum whose `Loaded` is `null`,
`#[rust_js::null]`, so a `Some` of it is boxed (ADR 0051), and the read
was the box's:

```rust
match cache.get(chunk_id).cloned() {
    None => { .. }
    Some(ChunkEntry::Pending(entry)) => promises.push(entry),
    Some(ChunkEntry::Loaded) => {}
}
```

```js
const entry = $someGet(chunkCache, chunkId);
if (entry == null) {
  ..
} else if ($someValue(entry) !== null) {
  const entry$1 = $someValue(entry);
  promises.push(entry$1);
}
```

**An `Option<V>` or `Option<&V>` local, `V` an untagged enum that may be
`null` but never `undefined`, each of its variants of a kind JS tells and
none `#[rust_js::undefined]` (`untagged::never_undefined`,
`representation::defined_payload`), held under this decision's
conditions, is its value: its `None` `undefined`, its `Some(v)` `v`, the
`null` of `v`'s variant its own.** Its writes and reads are this
decision's, with three more:

- **What its `Some` holds, read whole**, is the value as it is everywhere,
  so given on, kept or returned it needs no `only_tested`: the inner
  `Option`'s `null`, which is `undefined` elsewhere, has no counterpart.
- **A std map's `get` of such a value** writes one as JS's read itself,
  `cache.get(id)`: a JS `Map` holds its values as they are, so a key it
  lacks is `undefined` and one it has is its value, never `undefined`
  (`mir/nested_options.rs`'s `reads_held`, which the call's lowering asks,
  `binding_calls::held_read`). Not a map of `Option`s, whose `None` it holds
  as `undefined`: that keeps `$someGet`'s box.
- **`cloned()` or `copied()` of one, to another**, where the clone is the
  value itself (`needs_clone`, `clone_value`'s rule), is the value: a link,
  as a copy is, so one given on boxes both.

An `==` of two of them keeps the box: `nested_equality` compares an
`Option` of an `Option`'s payloads.

```js
const entry = cache.get(id);
if (entry === undefined) {
  return false;
}
if (typeof entry === "string") {
  pending.push(entry);
}
```

Case ([0262](0262-when-rust-and-js-disagree.md)): none. `undefined`, `null`
and the other variants' values are each their own JS value, so each test
is Rust's answer.

**A thread-local's `with`, `with_borrow` or `with_borrow_mut` of a
closure whose own body holds its result so writes one as it gives it
back**, the port's form, `CHUNK_CACHE.with_borrow(|chunk_cache|
chunk_cache.get(chunk_id).cloned())`: `chunkCache.get(chunkId)`, the
closure applied where its cell is quiet, or `$withBorrow(chunkCache,
(chunkCache) => chunkCache.get(chunkId), true)`. Which form the closure's
result is in is a fact of two bodies, so each says its half once, both
read of MIR before either is written:

- **The closure's**: `mir_nested` of its own body, its result a local
  like the others, given back where it returns (`result_held`). Its maker
  asks it where it's given, a call's closure type naming it.
- **The maker's**: whether the local the call writes stays held, as any
  local's (`mir_nested`), each closure it's so of `OptionWrites::
  held_results`, which its lowering tells the closure's
  (`lowered_from_mir`'s), as an offset's seed is told (ADR 0642).

The closure's own lowering holds its result so only where its maker said:
one given on, `owned(CACHE.with_borrow(|cache| cache.get(id).cloned()))`,
is boxed in both. A closure its maker says holds its result whose own
lowered body boxes it is refused, the two never read one value in two
forms; its maker read the same MIR, so no program meets it, and it has no
mutation.

Held by corpus `untagged_null_option_held` (a map's `get` and its
`cloned()` and `copied()` read by `match`, `is_none()` and whole, in a
thread-local's `with_borrow` too; one given to a function, compared, of
an enum whose clone copies, one that's `undefined` too, one that's
neither, a map of `Option`s, and a closure whose result is a call's or is
given on, against native Rust), `test/lowering-nested-options.test.ts`'s
"a thread-local's with_borrow gives back a map's value held as it is"
(React's chunk cache, in Node), and the mutations `with-borrow-result-unseen`,
`with-borrow-result-of-any-closure`, `held-result-not-given-back`,
`held-result-not-a-candidate`, `held-results-of-any-local`,
`closure-result-held-untold` (`lower/mir/closures.ts`), `held-whole-payload-only-tested`,
`held-cloned-unseen`, `held-cloned-unlinked`, `held-cloned-copies`,
`held-map-get-any-value`, `held-eq-of-any` (`lower/mir/nested_options.ts`),
`never-undefined-of-undefined` (`lower/untagged.ts`),
`defined-payload-unboxed-too` (`lower/representation/options.ts`),
`held-some-null-for-none` (`lower/mir/assign.ts`) and
`held-read-binding-only` (`lower/mir/structure.ts`).
