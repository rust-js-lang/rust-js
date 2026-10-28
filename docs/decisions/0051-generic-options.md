# 0051. `Option<T>` in generic code: boxed only when it looks like `None`

Status: Accepted. Extends [0030](0030-option.md) and [0049](0049-traits-and-generics.md). Amended: concrete code boxes too; a lazy iterator's `find` boxes its own; a box keeps the `null` it holds; a `&mut` of what a `Some` holds is `$someMut(o)`; a type parameter bound by a function trait is a function; a type parameter no call gives what may look like `None` is never boxed.

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

