# 0051. `Option<T>` in generic code: boxed only when it looks like `None`

Status: Accepted. Extends [0030](0030-option.md) and [0049](0049-traits-and-generics.md). Amended: concrete code boxes too; a lazy iterator's `find` boxes its own; a box keeps the `null` it holds; a `&mut` of what a `Some` holds is `$someMut(o)`; a type parameter bound by a function trait is a function; a type parameter no call gives what may look like `None` is never boxed; a function JS calls takes what JS gives as it gives it, its crate's Rust calling a copy that boxes. Amended by [1017](1017-an-option-of-an-option-held-where-its-made-is-its-payload.md): a local `Option` of an `Option` made and read only where it's held is its payload, `Some(None)` `null`. Amended: a thread-local's `with` and `with_borrow` give their closure's, boxed as it makes it. Amended: a function JS alone reads gives its type parameters nothing (#565).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`Some(x)` is `x`, and `None` is `undefined` (ADR 0030). That's only right when
`x` can't be `undefined` or `null` itself, so `Option<()>` and
`Option<Option<T>>` are errors.

Generic code can't know. In `fn count<T>(xs: Vec<T>)`, `T` might be `()`,
whose value is `undefined`, or `Option<i32>`, one of whose values is. Then
`Some(x)` would be `undefined`, and would read as `None`. So every
`Option<T>` in generic code was an error, which ruled out ordinary Rust
like `fn first<T>(xs: &[T]) -> Option<&T>`.

`Option<&T>` was worse: the check didn't look through the reference, so it
compiled, and with `T = Option<i32>` a first element of `None` read as
`None`. Concrete code had the same hole: `Some(&())`.

## Decision

**In generic code, `Some` of a type parameter is `$some(x)`: `x` itself,
unless `x` looks like `None`, and then a box.** This is ReScript's approach
for nested options (`Caml_option.some`):

```js
function $some(x) {
  if (x === null) return { $someNone: 0, $value: null };   // Some(None) of JS's null
  if (x === undefined) return { $someNone: 0 };            // Some(None), Some(())
  if (typeof x === "object" && "$someNone" in x) return { ...x, $someNone: x.$someNone + 1 };
  return x;                                                // every other value
}
```

- **A box keeps the `null` it holds** (amended): a JS value's `None`,
  `Option<&js::Unknown>`'s, is `null` or `undefined`, which JS tells
  apart, `String(null)` being `"null"`; `$someValue` gives back which. To
  `$eq` and `$key` a `$value` that's nullish is one left out (ADR 0280),
  so `Some(None)` is `Some(None)`, whichever it holds.

- **`None` is still `undefined`**, so "is it `Some`" is still `o != null`:
  a box is an object.
- **Taking the value out is `$someValue(o)`,** which unboxes one level. It
  appears in a `Some(v)` pattern, `?`, `unwrap`, `unwrap_or`
  (`$someValue(o ?? $some(d))`), `map`, and the closures of `map_or`,
  `and_then`, `filter`, `is_some_and` and the like. `unwrap_or_else` and
  `unwrap_or_default` are `o != null ? $someValue(o) : fallback`, as `??`
  would give the box.
- **It costs nothing for ordinary values.** `$some(5)` is `5`, so a generic
  function called with numbers, strings or structs returns plain values,
  and a JS caller sees what it would without generics. Only a `None`-like
  value is ever boxed.
- **Only where the payload could look like `None`**, looking through
  references, `Box` and `Rc`, which are the value itself (ADR 0023): a type
  parameter, `Option<T>`, `Option<&T>`, `Option<Box<T>>`; and in concrete
  code a `()`, a unit struct or an `Option`, `Option<Option<i32>>`. Every
  other `Option` keeps `Some(x)` as `x`. A literal that can't look like
  `None` is itself: `Some(Some(4))` is `4`, and `Some(None)` is
  `$some(undefined)`. (Amended: concrete code's were errors.)
- **Std functions that make an `Option` of a generic element box it too:**
  - `Vec::pop` is `$pop(v)`;
  - a slice's `first`, `last` and `get`, and an iterator's `find`, are
    `$someAt(v, i)`; a lazy iterator's `find` (ADR 0139), which has no
    index, is `$nextSome(items.filter(f))`, which stops at the first found,
    as `find` does. Iterator lowering, which chose the array or the lazy
    iterator, makes the whole result: `[None, Some(1)].into_iter().map(f)
    .find(..)` was `findIndex` of a JS iterator, which threw. (Amended.)
  - `bool::then` and `then_some` box their value, and `copied`, `cloned`,
    `as_ref`, `filter`, `or` and `or_else` keep the box they're given;
  - an `Option` of an `Option`'s `flatten` is `$someValue` of it, the inner
    one, a box one level shallower: `Some(None)` is `None` (amended);
  - an iterator's `last` is `$someAt(items, items.length - 1)`;
  - `Result::ok` wraps its value in `$some`.

  Any other std call that would make one is a compile error, not JS that's
  quietly wrong.
- **A copy of an `Option` changed in place copies only what's inside every
  `Some`:** a box holds nothing that can change.

A box can reach a JS caller only for a value whose `Some` looks like
`None`: an exported function's `Option<Option<i32>>` gives `Some(None)` as
`{ $someNone: 0 }`.

### A `&mut` of what a `Some` holds

A pattern of a `&mut Option<T>`, `Some(x)`, binds a `&mut T` to what the
`Some` holds, `$someValue(o)`, which JS can't assign. It's a handle, as any
`&mut` kept is (ADR 0099), of the runtime's `$someMut(o)`, given a handle
on the option: it reads `$someValue(o.value)`, and writes `o.value =
$some(value)`, so a value that looks like `None` is boxed as it's written.

```js
function write(o, v) {
  if (o.value != null) {
    const x = $someMut(o);
    x.value = v;
  }
}
```

Read, written, or read in a guard, chrono's `set_if_consistent`, which the
lowering from MIR (ADR 0364) had refused. `test/corpus/option_some_mut.rs`
holds it, a `Some(None)` written a `Some(Some(5))` and a `Some(Some(1))` a
`Some(None)` too.

## Why

- **Generic code is correct for every `T`.** Checked against native Rust
  with `T = ()` and `T = Option<i32>`, the cases where a plain `Some(x) = x`
  gets it wrong.
- **The output stays plain where it can.** `$some(x)` appears only in
  generic code, and it returns `x` unchanged for anything that isn't
  `None`-like, so interop is unchanged.
- **Proven prior art.** ReScript has represented nested options this way for
  years, and it's the same trade: an allocation only for the rare
  `Some(None)`.

## Alternatives

- **Reject `None`-like instantiations at the call site.** Keep `Some(x)` as
  `x`, and make `count::<()>(..)` an error wherever `T` ends up in an
  `Option`, however deep in generic calls. That needs the compiler to track
  every function's use of each parameter, and it would reject correct Rust.
- **Box every `Some` in generic code.** Simpler, but then a generic
  function returns boxes to its JS callers even for numbers.
- **Leave concrete code's an error.** It was, at first, since it changes
  the representation of concrete values. But it was 12 of rustc's tests,
  and the same helpers do it: `Option<Option<T>>` is ordinary Rust, a
  lookup that may find a `None`.

## Consequences

- An exported generic function called from JS with a `None`-like argument
  hands back a `{ $someNone }` box for `Some` of it.
- `$some`, `$someValue`, `$someAt` and `$pop` are runtime helpers, emitted
  only by modules that use them.
- A last `match` arm that does nothing (`None => {}`) no longer prints an
  empty `else {}`, which this change's example first showed.

## Amendment: `as_mut()` of one, and an `Option` of a `&mut`

- **`o.as_mut()` of an `Option<T>` of a generic `T`**, through a `&mut`,
  was refused, so rustc's `borrowck/call-and-ref-option-mutate.rs`,
  `x.as_mut().unwrap()` of a `&mut Option<T>`, didn't compile. It's
  `$someMut(o)` where it's `Some`, the handle a `&mut` to what a `Some`
  holds already is: it reads `$someValue` of the `Option` and writes it a
  `$some` of what it's given, so a `T` that looks like `None`, `()` or an
  `Option`, stays boxed.
- **An `Option` of a `&mut` that's a cell**, `Option<&mut T>`, is never
  boxed, even of a generic `T`: a cell is a handle object, never `null`, so
  `$some` of one was always the handle itself, and `$someValue` of it a
  read that did nothing. Generic code's is now concrete code's, `r == null
  ? .. : r`, with the same values.
- The `option_as_mut_generic` corpus case writes through it, of numbers,
  arrays, `()` and `Option`s, against native Rust.

## Amendment: a boxed `Some` compares by what's in it

`o == Some(())` was `o == { $someNone: 0 }`, two objects, which JS
compares by which object each is: never `true`, where Rust's is. An
`Option` whose `Some` is boxed compares by `$eq`, what's in it; one whose
`Some` is its payload, by `==`, as before. A corpus case,
`option_boxed_eq`, holds it against native Rust. Case C: results would
change.

## Amendment: more of what gives an `Option` of what looks like `None`

Each call that gives an `Option` boxes a `Some` that could look like
`None`, or is refused (`refuses_boxed_option`). More box it now (#146):
`Result::err()`, as `ok()` does; `nth`, `.map($some)` of the one item it
takes; a `Peekable`'s `next_if`, `$nextIfSome`, its test given the item
unboxed; and a `BTreeSet`'s `first`, `last`, `pop_first` and `pop_last`,
`$treeEnd`'s `boxed`. And more give the value as it's given, boxed
already: `xor`, `map_or` and `map_or_else`, and `mem::take` and
`mem::replace`, a place's old value. Held by
`test/corpus/boxed_option_results.rs`.

## Amendment: `last()` of items with a destructor

`last()` of items with a destructor is `$lastDropping`, which drops each
before the last as the next comes (ADR 0098), and gave the last item
itself: of `Option`s whose last is `None`, `None`, where Rust's is
`Some(None)` (#204). It's given `true` where an item could look like
`None`, a type parameter's too, and boxes what it gives as `$some` does
(`last_option_drops`, `last-dropping-unboxed`). That case's iterators,
stepped to drop each item they pass, grow the readability measure's
`$iter` and `Iterator.from` shares, as any of items with a destructor do.

## Amendment: a type parameter bound by a function trait is a function

Case: N ([0262](0262-when-rust-and-js-disagree.md)): each value is what
it was; the JS reads it without asking.

React's `flushSyncFromReconciler<R>(fn: (() => R) | void): R | void`
calls `fn` where it's given one, `if (fn) { return fn(); }`. Its port,
`fn flush_sync<R>(f: Option<impl FnOnce() -> R>) -> Option<R>`, read
what the `Some` held through the box it might be, `const f$1 =
$someValue(f); return $some(f$1());` (the port's gap wl8): a type
parameter could be anything, `()` or an `Option` too.

**A type parameter bound by `FnOnce`, `FnMut` or `Fn` is never
`undefined` or `null`**, as one a binding of its bound reads isn't (ADR
0307): what Rust lets be one is a closure, a function item, a `fn`
pointer, a `dyn Fn`, or a reference or `Box` of one, and each is a JS
function. So `Some` of one is the function itself, and `if let Some(f)
= f` reads `f` as it is: `if (f != null) { return $some(f()); }`. Its
`R` is any type still, an `Option` too, so `Some(f())` stays `$some`,
which is `f()` itself where it isn't nullish: upstream's `R | void` can't
tell `Some(None)` from `None`, and Rust can. `defined_by_its_bounds` in
`src/lower/representation.rs` is the one answer, of `boxed_payload` and
`can_be_nullish`. What's boxed is decided where each `Some` is made, as
`$some` does at run time, so a caller's `Some(closure)` and the generic
code that reads it agree whatever either knows.

`test/corpus/optional_callbacks.rs` passes an `Option` of an `impl
FnOnce`, of an `F: FnMut` through `as_mut()`, and of a `&F` where `F: Fn`,
`R` an `Option` and a `()` among them, against native Rust; mutation
`callable-param-some-boxed`.

## Amendment: a type parameter no call gives what may look like `None`

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a JS caller's
results changed, given a box its declaration doesn't name; each Rust
caller's are what they were.

The port's `flush_sync<R>(f: Option<impl FnOnce() -> R>) -> Option<R>`
printed `return f != null ? $some(f()) : undefined;`. React's JS callers
of it, react-test-renderer's `unstable_flushSync`, react-noop's
`flushSync` and ReactDOMClientFB, are given what the callback gives,
upstream's `R | void`; a callback that gives `undefined` or `null` got
`{ $someNone: 0 }` or `{ $someNone: 0, $value: null }` back, which the
declaration rust-js prints, `R | null | undefined`, doesn't name either.
No Rust caller gave `R` anything: only JS calls it, and JS's `R` is any
value, which has no Rust type to be an `Option` of.

**A type parameter that no call gives what may look like `None` is never
boxed: a `Some` of it is the value.** Rust can tell `Some(None)` from
`None` only of a value of a type that may look like `None`, `()`, an
`Option`, a unit struct, a type parameter; if no call gives the
parameter one, no value of it is ever `undefined` or `null` in Rust, and
`$some(x)` is `x` of every value it's given. So the box is left out where
it would never be made, and a JS caller is given the value its callback
gives, `undefined` too, as TypeScript's `R | undefined` says:

```js
export function flushSync(f) {
  if (f != null) {
    return f();
  }
}
```

- **Which parameters**, `unboxed_params` in
  `src/lower/analysis/mutation.rs`, as ADR 0289 finds a `Copy` bound's
  and ADR 0914, amended, a `Clone` bound's: of a function only its
  crate's calls call (`copies_by_callers`), each type parameter none of
  its calls gives a type that may look like `None`
  (`representation::may_look_like_none`, which sees through references,
  `Box`, `Rc` and `Pin` and takes any `Weak` and any type parameter as
  one, never less than `can_be_nullish`), nor a type parameter of its own
  that's given one: `relay<U>(x: U) { wrap(x) }` passes `U`'s on to
  `wrap`'s `T`. A function a dictionary calls, a trait's or an impl's,
  and what it passes its own on to, is given anything, as before.
  `defined_by_its_bounds` (`representation/options.rs`) asks it, of the
  body's owner, beside a function trait's bound and a binding's.
- **Where a call gives one, the box stays**, in that function and every
  one it passes it on to: `relay(())`'s `Some(())` is `Some`. A JS caller
  of that function is given the box, as before: its crate's own Rust
  needs it.
- **A library's export says which of its parameters are unboxed**, a
  fact of each, `unboxed` (library ABI 10). A consumer whose call gives
  one of them what may look like `None` compiles its own (ADR 0914),
  which boxes it: `dep::flush_sync(Some(|| ()))`'s `is_some()` is `true`.
  `@rust-js/std`'s functions are called by every program and compiled by
  none: none of theirs is unboxed.
- **What a copy of one is**: an `Option<T>` of a `T: Copy` whose copy is
  the value, `items.last().copied()`, is the `Option` itself, `return
  items.at(-1);`, not `value == null ? value : value` (`copies.rs`).

Alternatives: a bound the program writes, `R: Defined`, bends the Rust to
the compiler, which AGENTS.md rules out; a JS-facing export that unboxes
what it returns makes two functions of one, and a Rust caller in another
crate would have to call the other; boxing only where the instantiation
is known, at run time, can't be done: a Rust `()` and a JS callback's
`undefined` are the same value. Cost: a crate-wide walk of each call's
type arguments, which `given_params` already makes of `Copy` and `Clone`
bounds; generic code called with numbers alone reads plainer, `first`'s
`items[0]`, not `$someAt(items, 0)`.

`examples/generic_options.rs`'s `flush_sync` is called by
`test/compiler.test.ts`'s JS with callbacks that give `undefined`,
`null` and `5`; `test/corpus/unboxed_type_params.rs` gives a relay, a
generic type's methods, a function pointer, a closure in generic code
and a trait impl's method `()` and `Option`s, against native Rust;
`test/corpus/optional_callbacks.rs` keeps a trait method's `F: FnOnce`
unboxed by its bound alone; `test/cross-crate-items.test.ts` "a consumer
giving what may look like None compiles its own of an unboxed function".
Mutations `unboxed-param-given`, `nullish-passed-on`,
`unboxed-library-fact` and `instance-gives-nullish`.

## Amendment: a function JS calls takes what JS gives as it gives it

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a JS caller's
results changed with what the crate's own Rust gives a function.

The amendment above left a box for JS callers where the crate's own Rust
gives a type parameter what may look like `None`: `pick(undefined, true)`
of examples/generic_options.rs gave JS `{ $someNone: 0 }`, as `kept`
gives `pick` a `()`, where the declaration says `T | undefined`. What a
JS caller got depended on code it never sees. The coordinator's decision:
a function JS calls gives JS callers the value, never the box.

**A function JS calls, one its module exports, `pub` and no trait's,
takes each type parameter as JS gives it: a `Some` of it is the value,
`undefined` too, and the value may be `undefined` or `null`. Where the
crate's own Rust gives one what may look like `None`, its call calls a
copy, compiled from the same MIR beside it, that boxes:**

```js
export function pick(x, keep) {
  if (keep) {
    return x;
  }
}

function pickBoxed(x, keep) {
  if (keep) {
    return $some(x);
  }
}
```

- **Which lowering a call calls**, `src/lower/analysis/js_given.rs`
  (`JsGiven`): each lowering takes its type parameters as a set of those
  unboxed and, of those, those JS gives (`Params`). A function's own JS
  takes them as its crate's calls give them (`unboxed_params`), or, of one
  JS calls, each unboxed and JS-given. A call, in any lowering, of a
  generic function only its callers call gives each type parameter what's
  nullish (what may look like `None`, or one of its own the lowering
  boxes), what JS gave (one of its own the lowering takes so, seen through
  references, `Box`, `Rc` and `Pin`), or what's never nullish. The
  callee's own JS takes it where it unboxes nothing nullish and takes as
  JS gave what JS gave; else a copy that unboxes what its crate's calls
  give unboxed and what JS gave, and takes the latter as JS gives it. The
  analysis finds every copy from each function's own JS, of every call of
  its bodies and its closures, `calls_of`, as `unboxed_params` does; the
  lowering asks the same question of each call, `called_copy`, of a call
  and of a function given as a value, and one it finds no copy for is
  refused, never called otherwise.
- **What a value JS gave is**: any JS value. `boxed_payload` of it is
  `false` and `can_be_nullish` `true` (`representation/options.rs`): a
  `Vec<T>` of them sorts with `$sort`, which puts `undefined` where its
  comparison says. A type parameter a function trait bounds, or a
  binding's bound reads, is a function or an object, as before.
- **A copy is named for what it does apart**, `pickBoxed` of a function
  JS calls, `wrappedUnboxed` of another its JS's values reach, in its
  module, beside it, a method's in its type's object; exported only where
  another module calls it, and imported by its name.
- **A copy made alike another is left out**, its calls calling that one
  (`pipeline/copies.rs`): after the crate is lowered, each copy whose JS,
  each call of a copy named as what that copy is, reads as its function's
  own or an earlier copy's is that one, read as if it were that one, so
  a copy that calls itself, a walk of a tree, is alike one that does
  (React port gap rd61). A function whose `T` is never in an `Option`,
  `doubled<T>`, is one function.
- **A library's export says each type parameter of a function JS calls
  is unboxed** (`unboxed`, library ABI 10), as its JS is: a consumer that
  gives one what may look like `None` compiles its own (ADR 0914).

What a JS caller is given is what a JS-only caller was given before:
`count_some([undefined])` from JS counts `0` and `unwrapped(undefined)`
panics, as `Some(undefined)` is `None` to it, which its `T | undefined`
says; the crate's own Rust is given what native Rust gives, `Some(())`
`Some`.

Alternatives: the box kept for JS callers where Rust gives one, the
amendment above, gives JS what its declaration doesn't name, by code it
can't see; an export that unboxes what its Rust copy returns, `return
$someValue(pickBoxed(x, keep))`, keeps Rust's meaning inside, so
`unwrapped(undefined)` is `undefined`, but reaches only an `Option<T>`
returned whole, not one in an array, a field, a callback's argument or
what's written through a `&mut`, and would make two forms of one export;
one lowering for both, boxing at run time by what a caller is, can't
tell a Rust `()` from JS's `undefined`. Cost: a function JS calls whose
crate gives it `()` or an `Option` is two functions where its `T` is in
an `Option`; each generic function such a function's JS reaches is
lowered once more, and kept only where it reads otherwise. A trait's
method, a dictionary's, a class's member and std's compiled bodies still
box what they're given; a function JS calls that passes what JS gave to
one of them may give JS a box back, as it did. A copy says each `return;`
its function does, examples/generic_options.rs's `triedBoxed` beside
`tried`: one more of the readability ratchet's `return;`s (ADR 0565).

`test/corpus/js_given_copies.rs` relays through another module's
function, steps a `filter_map` of a function given as a value, a generic
type's method, and a function whose `T` is never in an `Option`, `()`,
`Option`s and numbers, against native Rust; `test/compiler.test.ts` calls
examples/generic_options.rs's `pick` and `relay` from JS with `undefined`,
`null` and values, and its Rust's `units` and `relayedUnit`;
`test/cross-crate-items.test.ts` "a consumer giving what may look like
None compiles its own of an unboxed function" calls a library's `wrap`,
which its own Rust gives an `Option`, from JS; `sortWith` sorts what JS
gives with `$sort`. Mutations `js-given-copy-unrouted`,
`js-given-facing-boxed`, `js-given-passed-on-boxed`, `js-given-not-nullish`,
`js-given-boxed`, `js-given-copy-kept`, `js-given-copy-unimported`,
`js-given-call-unrouted`, `js-given-value-unrouted` and
`js-given-copy-self-call-apart`.

## Amendment (2026-10-29): what a thread-local's closure gives

A std call whose `Option` may be boxed is refused unless rust-js knows it
gives the box its value has (`refuses_boxed_option`). A thread-local's
`with` and `with_borrow`, which give what their closure gives, weren't
among those, so React's SimpleEventPlugin's `let Some(react_name) =
NAMES.with_borrow(|names| names.get(dom).cloned()) else { return; }`, of
a `HashMap<DOMEventName, Option<String>>`, was refused (rd111). **They
give the closure's own `Option`, boxed as the closure makes it**, as
`and_then`'s does: `const withBorrow = $someGet(names, dom); if
(withBorrow == null) { return; }`. Upstream's `reactName === undefined`
needs ADR 1017's payload representation of a map's value, which a `Map`
holds; not yet. Held by `test/corpus/local_borrow_nested_option.rs`, with
native Rust, and the mutation `local-borrow-nested-option-refused`.

## Amendment (2026-11-02): a nullable `Option<Option<T>>` field is held as its payload (#470, the user's decision)

#470, P0, a wrong answer both ways. webapi's dictionaries hold an optional
nullable member, WebIDL's `DOMString? sdpMid` a dictionary may leave out
or `any value`, as an `Option<Option<T>>` field, and this decision's box
was its representation:

```rust
pub fn has_value(entry: &ReadableStreamReadResult) -> bool {
    entry.value.is_some()
}

pub fn candidate() -> RTCIceCandidateInit {
    RTCIceCandidateInit { candidate: Some("c".to_string()), sdp_mid: Some(None), ..Default::default() }
}
```

```js
export function hasValue(entry) {
  return entry.value != null;
}
export function candidate() {
  return { candidate: "c", sdpMid: $some(undefined) };
}
```

Of `{ value: null, done: false }`, what `reader.read()` resolves a `null`
chunk with, `hasValue` was `false` where Rust's is `true`, and the browser
was given `sdpMid: {"$someNone":0}` where it reads `null`. JS holds the
member's three values as three of its own: no key, `null`, the value.

The user chose to say it on the field (option A of two): **a
`#[rust_js::nullable]` field of an `Option<Option<T>>`
(`bindings::holds_nested`) is ADR 1017's held form wherever it's made, set
or read: its `None` not there, `undefined`, its `Some(None)` `null`, its
`Some(Some(v))` `v`.** webapi's generator writes the attribute on each
optional member whose type is an `Option` already, nineteen of them, and a
binding of a library's own JS object says it the same way. `nullable` on
an `Option<T>` field is still ADR 0275's, its `None` `null`; on an
`Option<Option<T>>` field it was that of the outer `None`, which boxed
`Some(None)` and was of no use, so no program changes.

- **What reads it** (`mir/nested_options.rs`, the owner, and its callers):
  its discriminant `!== undefined` (`rvalues.rs`), what its `Some` holds
  the value itself, its `None` `null` as a nullable field's is
  (`places.rs`), and `flatten()` the read, `entry.value` (`dispatch.rs`).
  A shared reference made once to it, a copy of it, and a copy of a copy
  are held so too (`mir_nested`'s `field_refs` and `of_fields`), so
  `is_some()` is `entry.value !== undefined`, `matches!(.., Some(None))`
  `entry.value === null`, and a parameter's pattern, Flight's
  `ReadableStreamReadResult { done, value }`, is `({ done, value })` with
  `value.flatten()` `value`.
- **What sets it** (`assign.rs`, `rvalues.rs`'s struct literal): `None`
  `undefined`, left out of a literal; `Some(x)` `x ?? null`, or `x` of what's
  `null` or a value already; what one that holds it so holds, as it is.
- **Where it meets the box**, given whole to code that boxes, `{:?}`, a
  generic function, a shared reference given on, it's the box there,
  `x === undefined ? undefined : $some(x)` (`boxed_of_held`), and a box
  given it is `x === undefined ? undefined : $someValue(x) ?? null`
  (`held_of_boxed`), each value read once. One representation per place:
  the field's is always the held form, and a local holds it so only where
  `mir_nested` says, else its box all through.
- **Refused**: a `&mut` to it, which would be a handle that writes the box
  in it ("set the field whole, `s.field = Some(None)`"), and a `T` that may
  look like `None`, a `()`, an `Option`, a type parameter, a unit struct or
  an untagged enum of `undefined` or `null`, whose `Some(Some(v))` couldn't
  be told from `Some(None)` (`bindings::nested_refused`).
- Its `.d.ts` is `sdpMid?: string | null | undefined`.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior: each of the three values is its own JS value, as JS
holds them, and each read and write is Rust's.

Not yet: a local of a `T` with a destructor, a `String`'s, isn't held
(ADR 1017's rule), so `sdp_mid: if c { Some(None) } else { .. }` is boxed in
its temporary and converted where it's given, correct but not what a
person writes.

Held by corpus `nested_option_nullable_field` (made of a constant, a
variable, a parameter and another field, read by `match`, `matches!`,
`is_some()`, `is_none()`, `if let`, `flatten()`, a copy taken apart, given
whole, shown, compared and set again, against native Rust),
`test/lowering-nested-options.test.ts`'s "a dictionary's nullable Option of
an Option is what JS holds" (webapi's `ReadableStreamReadResult` of `{}`,
`{ value: undefined }`, `{ value: null }` and values, destructured, and
`RTCIceCandidateInit` made `Some(None)`, in Node),
`test/diagnostics.test.ts`'s two refusals and `test/declarations.test.ts`'s
"declarations take JS's null for an Option"; mutations `nested-field-*` in
`lower/mir/nested_options.ts`, `places.ts`, `rvalues.ts`, `operands.ts`,
`assign.ts`, `dispatch.ts` and `lower/declarations.ts`,
`nested-flatten-unread` (`lower/mir/dispatch.ts`), and
`nested-payload-unchecked` and `nested-nullable-outer-null`
(`lower/bindings/nullable.ts`).

## Amendment (2026-11-02): a closure's capture of a nullable `Option<Option<T>>` field is held too (rd631)

Readability, P3. A closure that read such a field, React's Flight reply
client's `catch_unwind(AssertUnwindSafe(|| reply.stringify(entry.value.flatten())))`,
captured `entry.value` as ADR 0051's box and read it so: `const option =
entry.value; const captured = option === undefined ? undefined :
$some(option); .. stringify($someValue(captured))`, where upstream writes
`JSON.stringify(entry.value, ..)`. The capture is now held as the field
holds it, and the closure's body reads it so, as a field: a borrow of the
field is the field, read where it's used, `stringify(entry.value)`; one
taken by value, a `move` closure's, is its copy, `let captured =
entry.value;`, which the closure reads and sets as it's held.

- **One question, one owner.** `nested_options.rs::held_capture` says
  which captures are: a closure's (not a coroutine's) capture whose place
  ends in a nested field (`bindings::holds_nested`), borrowed shared or
  taken by value, of rustc's `closure_captures`. The closure's body asks
  it where it reads its environment (`nested_field`), its maker where it
  makes the closure (`mir_nested`, `held_captures`).
- **The maker always gives it held.** Where what it captures is held, a
  borrow made once and read only so, it's given as it is; where its maker
  read it as the box, a field read whole or a borrow given to code that
  boxes too, it's held again (`held_of_boxed`), so the closure's reads are
  right whichever its maker made.
- Not yet: a capture of a local that holds one, `let value = entry.value;
  move || value.flatten()`, stays the box, as a local captured is
  (`mir_nested`'s "captured").

Case ([0262](0262-when-rust-and-js-disagree.md)): none: the same values
are read, as the field holds them.

Held by corpus `nested_option_field_captured` (borrowed and moved, by
`Fn`, `FnMut` and `FnOnce`, under `catch_unwind`, read whole, flattened,
matched, shown, given to a generic function and set again, and of a
maker that boxes it, against native Rust); mutations `held-capture-*` in
`lower/mir/nested_options.ts`.

## Amendment (2026-11-02): a local isn't named as a copy's function (#467)

A copy alike its function, or an earlier copy, is that one: each call of
it is renamed once every body is tidied (`pipeline/copies.rs`). A body's
temporary named for its call (ADR 0569) took the copy's name's function's
name, free while the call was of the copy: chrono's `format::parse::parse`
was `const parseInternal = parseInternalUnboxed(..)`, then, the copy alike
`parseInternal`, `const parseInternal = parseInternal(..)`, which threw
"Cannot access 'parseInternal' before initialization". A wrong answer,
ADR 0262's "results change" case.

**Where a call of a copy is renamed to the one it's alike, a local of that
name in its function is named apart first**, as tidy names one apart:
`const parseInternal$1 = parseInternal(s, items, BBorrowU8);`.

- Held by corpus `copy_call_named_for_its_function`, against native Rust,
  and the crate corpus's chrono probe. Mutation `copy-target-shadowed`
  (`lower/pipeline/copies.ts`).

## Amendment (2026-11-04): a function JS alone reads gives its type parameters nothing (#565)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Nothing calls the
function where it's given, so nothing is given to it there to box.

alien-signals' `isSignal` compares a function's name with its
`signalOper`'s, which a generic `signal_oper<T>` names with some type,
`js::function::name(&signal_oper::<()>)`. That counted as a call giving
`T` a `()`, which may look like `None`, so `signalOper` boxed its
`Some`, and the closure of `pub fn signal<T>` that passes all on to it
bound a copy, `signalOperUnboxed.bind(null, this$)`: `isSignal` was
false, a wrong answer.

**A function of rest arguments given to a binding's parameter of any
shape whose result doesn't name it (`function::name`, `length`, `call`,
`js::unknown_of`), where JS alone reads it, gives its type parameters
nothing, and is its own JS, the one its module names it by**
(`instances::given_calls_of`, `read_by_js_alone`, which the analyses of
what each call gives read, `js_given` and `mutation::given_params`; and
`FnCx::rest_function_itself`, which the operand's lowering asks there
first, ADR 0664, amended). One the binding gives back, `function::bind`'s,
is called by Rust, and gives what it's given as before. A function of no
rest arguments is unchanged: its value is lowered as its call's arrow
elsewhere, which needs its copy.

What holds it: `test/rest-parameters.test.ts`, "a generic function of
rest arguments JS alone reads is its own JS" (`isSignal` true, one
`signalOper`; `bind` of `first::<()>` boxing its `Some(())`; a function
of no rest arguments named so). Mutations `js-alone-*`
(`instances.ts`, `mir/operands.ts`).
