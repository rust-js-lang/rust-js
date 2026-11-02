# 0283. The js crate covers TypeScript's ES library, and a test holds it

Status: Accepted. Amended by [0994](0994-a-number-a-binding-takes-is-typescripts-number.md): a typed array takes numbers, `f64`s, and to `set` and `from` any `ArrayLike` of them. Extends [0102](0102-js-and-webapi.md) and [0281](0281-webapi-covers-typescripts-dom.md).
Amended by [0741](0741-an-array-buffer-view-is-what-each-view-derefs-to.md): each view
derefs to an `ArrayBufferView`, and `isView` takes any value.
Amended by [0698](0698-a-catch-handler-gives-a-value-or-a-promise.md): `catch`'s handler
gives a value or a promise; by [0699](0699-a-map-of-objects-is-a-weak-map.md): `js::Map`
is JS's `Map` of weak keys. Amended by [0666](0666-the-js-crate-binds-objects-reflection.md): a
`PropertyDescriptor` and a `ProxyHandler` are dictionaries, structs, and
`Object`'s statics are measured and bound. Amended by
[0677](0677-a-weak-key-is-what-kind-holds.md): a weak collection is typed as
TypeScript types it, and what `js::Kind` holds is a `WeakKey`. Amended by
[0782](0782-a-promises-executor-borrows-and-then-takes-two-handlers.md):
`promise::new`'s executor may borrow, and `then_with` is `then(f, g)`.
Amended: a `bigint` of any size is a `js::BigInt`, `Symbol()` takes an
optional description, and `Array.prototype` is bound (rd399).
Extended by [0887](0887-std-items-that-are-math-are-math.md): `Math`, left out as Rust's own, is
std's items as values too, `f64::ln` as a `fn` `Math.log`.
Amended (rd580, rd581, rd584, rd585): `Promise.prototype` is its
class's, a `then` handler of a `()`'s may give an `Option<Promise<()>>`,
`AsyncIterator::throw`, and `js::AsyncIterable<T>`, whose
`[Symbol.asyncIterator]()` is a method keyed by a global's value.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

[ADR 0281](0281-webapi-covers-typescripts-dom.md) holds the `webapi` crate
to TypeScript's DOM. The js crate, JS's own library, had what ports had
needed: JSON, a regular expression's `test`, a `Uint8Array`'s length. It
had no `Date`, no `Intl`, no typed array but bytes, no `WeakMap`, no
`Symbol`: what ReScript's Stdlib and TypeScript's `lib.es*.d.ts` have.

Most of JS's library is Rust's already. `Vec` is an `Array`, `str` a
string, `f64` a number with `Math`'s functions, `HashMap` a `Map`
(ADRs 0034, 0036, 0059). What's left is what Rust has no type for.

## Decision

**The js crate's goal is every member of the built-ins Rust has no type of
its own for, as TypeScript's ES2024 libs declare them; a test measures it.**

- **The built-ins measured** (`builtins/coverage.ts`'s `SCOPE`): `Date`,
  `RegExp`, `Error`, `Promise`, `Symbol`, `ArrayBuffer`,
  `SharedArrayBuffer`, `DataView`, the eleven typed arrays, `WeakMap`,
  `WeakSet`, `WeakRef`, `FinalizationRegistry`, `Proxy`, `Reflect`,
  `Atomics`, `JSON`, `Intl` and its classes, and the global functions
  (`parseInt`, `encodeURIComponent`, ..). Not `Array`, `String`, `Number`,
  `Math`, `Map`, `Set` nor `Object`: Rust's own types are those.
- **The reference** is TypeScript 5.9's `lib.es5` to `lib.es2024` files,
  pinned as the `typescript-es` package, read with TypeScript's parser: a
  built-in's interface's members, its constructor interface's (`Date.now`
  a static), and `new` where it has one.
- **A ratchet**: `builtins/coverage.txt` lists each member bound;
  `test/builtins-coverage.test.ts` fails when one isn't any more, or a new
  one isn't blessed, as webapi's does.
- **The bindings are written, not generated** (ADR 0116), as ReScript's
  Stdlib is: a built-in is a type, its members its methods (`date.get_time()`),
  its constructors and statics a module of its name (`date::now()`), as
  webapi's are (ADR 0282). Rust's own types' JS functions, `string::trim`,
  stay functions: Rust's `str` can't have methods of another crate's.

## Why

- **The same promise as the DOM's**, measured the same way.
- **Only what's missing is counted**: a program reaches `Math.sin` as
  `x.sin()`; measuring `Math` would count what Rust has as missing.

## Alternatives

- **Generate them from the `.d.ts`**: rejected for bindings (ADRs 0116,
  0119). TypeScript's overloads and generics have no one Rust a generator
  can choose; `Date` and `Intl` are few enough to write.
- **Measure all of JS's library**: a number that says `Array.prototype.at`
  is missing where `v.get(i)` is it.

## Consequences

- `typescript-es`, TypeScript 5.9.3, is a dev dependency beside TypeScript 7,
  which ships no lib files.
- At first 14 of 691 members, 2.0%.

## Since

- **`Date`**, all 47 of its members: `date::new_with_time(ms)`,
  `d.set_utc_hours(12.0)`, `date::now()`, `date::utc(..)`, numbers `f64`s as
  an invalid date's are `NaN`, and `to_iso_string()` a `Result` of the
  `RangeError` an invalid date throws. The crate's files beyond `lib.rs` are
  listed for its packages (`tooling/resources.js`). 8.8%.
- **The typed arrays, `ArrayBuffer`, `SharedArrayBuffer` and `DataView`**,
  every member: each element a Rust number of its kind, a `BigInt64Array`'s
  an `i64` (ADR 0086); `a.get(i)` and `a.set_index(i, x)` are `a[i]` and
  `a[i] = x`; a callback takes the element, `a.map(|x| x * 2.0)`, its index
  an `enumerate()` of `values()` away; `keys`, `values` and `entries` are
  JS iterators (ADR 0140). The eleven are one shape, written once in
  `builtins/typed_arrays.ts`, which writes `src/typed_arrays.rs`; a test
  holds the file to it. `uint8_array::new(buffer)` is `new_with_buffer`, as
  `new(length)` is the length's. 78.6%.
- **Checked against ReScript's Stdlib and TypeScript's libs, and adapted:**
  - `Date`: ReScript gives a part as an `int` both ways, which an invalid
    date's `NaN` makes unsound; here what a date gives is an `f64` and
    what it's given an `i32`. Its constructor of each arity,
    `makeWithYMD`, is `date::new_with_ymd`; its setters of several parts,
    `setHoursMS`, are `set_hours_m_s`; `Date.UTC` likewise. Its
    `toISOString` is a `string` that throws; here a `Result`.
  - Typed arrays: ReScript has one `TypedArray.t<'a>`, so an `Int8Array`
    is an `Int32Array`; here each is its own type, as TypeScript's are.
    From ReScript, each callback's `_with_index` form, a comparator that
    gives an `Ordering` (-1, 0 or 1 in JS, ADR 0057), `copy`,
    `slice_to_end`, `subarray_to_end`, `fill_range` and `index_of_from`.
  - `RegExp`'s members are methods, its match a `RegExpMatch` with
    `full_match()` and `get(i)`, and `reg_exp::escape`, as ReScript's.
  - `Error`'s `name`, `cause` and `stack`, `js_error::new`; `Symbol`, its
    registry and its well-known symbols as statics; the global functions.
  - `WeakMap`, `WeakSet`, `WeakRef` and `FinalizationRegistry`, whose keys
    are `WeakKey`s: JS throws on a string as one, so Rust doesn't take
    one, and a struct is one as its `unsafe impl` says.
- **What TypeScript marks `@deprecated` isn't counted**: `RegExp.$1` and
  the rest of Annex B's legacy, by the interface it's on (`String.sub` is,
  `Atomics.sub` isn't). 88.1% of 675.
- **`Promise`**, every member, after ReScript's `Stdlib_Promise`:
  `promise::new(|resolve, reject| ..)`, `then` of a closure that gives a
  promise (`then_resolve` of one that gives a value, as ReScript's
  `thenResolve`), `catch`, `finally`, and `resolve`, `reject`, `race`,
  `any`, `all` and `with_resolvers` in its module. `all_settled` gives
  `PromiseSettledResult`s, a union tagged by `status` (ADR 0284), as
  TypeScript's `PromiseSettledResult` is and ReScript's `settledResult`.
  ReScript's `all2` to `all6` are what `Promise.all` makes of a tuple; here
  `all2` to `all4`, a tuple of promises to a promise of a tuple. A
  rejection's reason is a `JsError`, as `settle`'s is, where TypeScript has
  `any` and ReScript `exn`. 89.6%.
- **The measure counts what it missed**: a constructor interface in
  `namespace Intl` ends at its own brace, a `new` after its doc comment is
  one, and an interface's members are those of what it extends too
  (`Intl.Locale`'s `region` is `LocaleOptions`'). 87.8% of 689.
- **`Reflect` and `Proxy`**, every member, after TypeScript's; ReScript has
  neither. Each is of a JS value of any shape, an `Unknown`, as `get` and
  `set` are, a key a `PropertyKey`. A proxy is an `Unknown`, not its
  target's type: its traps may give anything, which a Rust type's fields
  wouldn't be. A `PropertyDescriptor` and a `ProxyHandler` are made empty,
  `{}`, and given what's set, as JS tells what's left out from what's
  `undefined` there: a descriptor's `writable` left out is the property's
  as it was. `set_prototype_of`'s `None` is `null` (ADR 0275); a
  `getPrototypeOf` trap gives an object, since a closure's `None` is
  `undefined`, which JS throws of. 90.0%.
- **`Atomics`**, every member, after TypeScript's; ReScript has none. Each
  takes an `AtomicArray`, an integer typed array, and its `Element`, the
  Rust number of its kind (`i64` of a `BigInt64Array`); `wait`, `notify`
  and `wait_async` an `Int32Array` or a `BigInt64Array`, a
  `WaitableArray`. `wait` gives a `WaitResult`, `"ok"`, `"not-equal"` or
  `"timed-out"`; `wait_async`'s `value` is one, or the promise of one, an
  untagged enum (ADR 0214), where TypeScript's union is told by `async`.
  An optional count or timeout is a function of its own,
  `wait_with_timeout`.
- **A view is generic over its buffer**, `Int32Array<B = ArrayBuffer>`, as
  TypeScript's are since 5.7: `new_with_buffer` takes an
  `ArrayBufferLike`, an `ArrayBuffer` or a `SharedArrayBuffer`, as JS
  does, and its `buffer()` is the one it views, so a shared one's has no
  `resize` to throw. One made otherwise is an `ArrayBuffer`'s. The buffer
  is given as the reference `shared_array_buffer::new` gives,
  `new_with_buffer(shared)`, not `&shared`, which a generic doesn't
  dereference. 91.9%.
- **`Intl`**, every member, after ReScript's `Stdlib_Intl` and TypeScript's
  `lib.es*.intl.d.ts`, in `js::intl`. A formatter is made of `locales`, a
  `&[&str]` whose empty one is the user's, as ReScript's `array<string>`,
  and its options: a struct of `Option`s, `None` left out (`undefined`,
  which Intl reads as not given), `..Default::default()` the rest, as
  webapi's dictionaries are. Each string union is an enum, as ReScript's
  polymorphic variants are, where TypeScript's resolved options are
  `string`s; a number's `format` has ReScript's `format_big_int` and
  `format_string` beside it. `Locale::new` and `get_canonical_locales`
  are `Result`s of the `RangeError` of a tag that isn't one; `Segments`
  are iterated as `segments::iter(s)`, `Iterator.from(s)`. A date's
  `to_locale_string_with(locales, options)` takes the same options. The
  shape is written once, in `builtins/intl.ts`, which writes
  `src/intl.rs`, as the typed arrays are. 100.0% of 689.

## Amendment: an `Error` object is a `js::Error` (2026-10-26)

React's `createCapturedValueFromError(value: Error, stack: null |
string)` takes an `Error`, made by `new Error(..)` where it's called. The
crate had one type of an error, `JsError`, whatever was thrown, which a
declaration says is `any`, as JS may throw anything and TypeScript types a
rejection's reason so; and `js_error::new` gave one of those. So the port's
`value: &JsError` was `value: any` in its `.d.ts`, looser than upstream's.

**`js::Error` is an `Error` object, declared `Error`; `js::error::new(message)`
makes one, `new Error(message)`**, in place of `js_error::new`, the one way
to make one. It's a `&JsError` by `Deref`, as what may be thrown is, so its
`message()`, `name()` and `stack()` are `JsError`'s, and it's given where a
`&JsError` is: `Some(error::new(".."))` of an `Option<&JsError>`, `Err(..)`
of a `Result<_, &JsError>`, by Rust's deref coercion. An `AggregateError`
is an `&Error` by `Deref` now, and declared `AggregateError`. Both are weak
keys, as `JsError` is. `JsError` stays what's thrown, `any`; a downcast of
a payload is of `&JsError` (ADR 0710).

- **Its JS is the same**: `new Error(message)`, the object itself where
  it's given as a `&JsError`, as an `AggregateError`'s `Deref` is.
- **Migrated**: the crate's tests and the guide's `unknownOwner`; React's
  port's six `js::js_error::new(..)` are to read `js::error::new(..)`.
- Case N ([0262](0262-when-rust-and-js-disagree.md)): a type's
  declaration.

Held by `test/js-builtins.test.ts`, "an Error object is declared Error,
and what's thrown any", whose declarations say `value: Error`, `Error`,
`AggregateError` and, of a `&JsError`, `any`, and whose `Error` is a weak
key, its `message()` read, run in Bun.

## Amendment: an `Error` is made of its options too (rd173)

`js::error::new(message, options)`: `new Error(message, { cause })`, as
React DOM's customizeViewTransitionError wraps what the browser threw.
`options` is an `Option<&ErrorOptions>`, left out where it's `None`, as
an optional argument is (ADR 0443), one function for both, and `cause` of
any value, a `&dyn IntoUnknown`, given as it is (ADR 0654). Every
`error::new(message)` reads `error::new(message, None)`.

Held by `test/js-constructors.test.ts`'s "an Error is made of its message
and options, its cause".


## Amendment: a `bigint` of any size, `Symbol()`, and `Array.prototype` (rd399)

React's Flight client reads a model's `$n` as `BigInt(value.slice(2))`,
its `$D` as `new Date(Date.parse(value.slice(2)))`, asks of an array
`getPrototypeOf(value) === ArrayPrototype`, `Array.prototype`, and makes
its response's key `Symbol()`, of no description. Its draft named each
by a global path of its own, as no binding had them but `Date`'s.

- **`js::big_int(value)` is `BigInt(value)`, a `&'static js::BigInt`**,
  TypeScript's `bigint` of any size, of what TypeScript's `bigint |
  boolean | number | string` takes (`IntoBigIntOrBoolOrNumberOrStr`, an
  `i64` and a `u64` a `bigint` too). Not an `i64`: an `i64` is a `bigint`
  of 64 bits (ADR 0086), which Rust's arithmetic on wraps, and text may
  name a wider one, which an `i64` would hold where none can. What JS
  throws of text or a number that's no integer is a panic.
- **`js::symbol::new(description)` takes TypeScript's optional `string |
  number`**, `Option<&dyn IntoStrOrNumber>`, as `date::new` takes its
  optional union (ADR 0443): `symbol::new(None)` is `Symbol()`,
  `symbol::new(Some(&"id"))` `Symbol("id")`. One function for each, as
  every `symbol::new("id")` reads `symbol::new(Some(&"id"))` now.
- **`js::array::prototype` is `Array.prototype`**, a value of any shape,
  as `js::object::prototype` is `Object.prototype`.
- `Date.parse` and `new Date(time)` were bound, `date::parse` and
  `date::new(Some(&time))`; V8's `Error.stackTraceLimit` is the node
  crate's, `node::js_error::stack_trace_limit` (ADR 0648).

Case: N ([0262](0262-when-rust-and-js-disagree.md)), each JS's own
function, typed as TypeScript's lib, never narrower. Held by
`test/js-globals.test.ts`, which runs each.

## Amended (2026-11-01, rd347): a proxy is an object of its target's type

Case ([0262](0262-when-rust-and-js-disagree.md)): N; the same `new
Proxy(target, handler)`, typed as TypeScript types it.

ReactFlightReplyClient keys a `WeakMap` by a proxy,
`knownServerObjectReferences.set(new Proxy({}, handlers), id)`. `proxy::new`
gave an `&Unknown`, which no `WeakMap` keys, so the port tested
`typeof reference === "object"` first, a test upstream doesn't make of what
is always an object.

**`proxy::new<T: WeakKey + ?Sized>(target: &T, handler) -> &'static T`**,
TypeScript's `new <T extends object>(target: T, handler: ProxyHandler<T>):
T`: an object, of its target's type, a weak key as its target is. So is
`revocable`'s, `RevocableProxy<T>::proxy()`. A handler's traps are given
the target as any value, as before. Held by `test/js-builtins.test.ts`'
"Reflect and Proxy are JS's", a proxy keying a `WeakMap`.

## Amended (2026-11-01): a proxy's trap is given JS values (rd471)

React's Flight temporary references proxy a function whose `get` trap
answers `'Provider'` with its receiver, `return receiver;`. `ProxyHandler`'s
traps took `&Unknown` of any lifetime, so a trap couldn't give one back as
the `&'static Unknown` it returns. **Each trap's target, key, receiver and
values are `&'static Unknown`, as every value JS gives is**, so a trap
gives back what it's given. Closures written before compile as they did.
Held by `test/js-builtins.test.ts` ("Reflect and Proxy are JS's",
`provided`).

## Amendment (2026-11-01): `error::new`'s message is optional (rd474)

React's Flight server stashes `new Error()` on a thenable's DEV
`_stacks`. `js::error::new(message: &str, ..)` made `new Error("")`,
which differs: its `message` is an own property, where `new Error()`'s
is the prototype's. Case C ([0262](0262-when-rust-and-js-disagree.md)):
`Object.hasOwn(error, "message")` differs, so the binding is TypeScript's
`new (message?: string, options?: ErrorOptions)`, as `aggregate_error::new`,
`DOMException` and the other error constructors already are:
**`error::new(message: Option<&str>, options)`**, `None` left out,
`new Error()`. Every caller moved to `Some(message)`; their JS is
unchanged. Held by `test/js-errors.test.ts`'s "an Error object is
declared Error, and what's thrown any", run in Node: `new Error()` has
no own `message`, `new Error("")` has. A binding's signature only: no
compiler change, so no mutation.


## Amendment (2026-11-02): React's Flight server's promise and async iteration (rd580, rd581, rd584, rd585)

React's react-server-dom-webpack server, ported, needed four things the
crate didn't say how to write. Its drafts named each by a binding of their
own or read it off a value of any shape, `getAsyncIterator.call(iterable)`,
where upstream writes `iterable[ASYNC_ITERATOR]()`.

- **`Promise.prototype` is its class's, `js::class::<Promise<()>>().prototype()`**
  (rd580): `Object.create(Promise.prototype)`, and a proxy's
  `getPrototypeOf` trap that gives it. No `js::promise::prototype` beside
  it: a class's prototype is `js::class::<T>().prototype()`, the one way,
  which React's Flight client and reply server already write;
  `js::array::prototype` and `js::object::prototype` are bound because
  `Vec` and an object are Rust's own types, of no `Classed`. Case N
  ([0262](0262-when-rust-and-js-disagree.md)): JS's own value, read as JS
  reads it.
- **A `then` handler of a `()`'s may give an `Option<Promise<()>>`**
  (rd581), TypeScript's `void | PromiseLike<void>`: `Some(promise)`,
  which JS waits for, or `None`, `undefined`. Upstream's debug channel
  reader returns the next read's promise from `progress`, `return
  reader.read().then(progress).catch(error);`, and nothing once it's done;
  its port dropped the `return`, so the first read's promise settled
  before the last read's chain. `impl Fulfils<()> for Option<Promise<()>>`,
  only of a `()`'s: a general `Option<Promise<T>>` of what `()` fulfils,
  an `Option`'s too, makes a handler's `None` of `catch(|_| None)` either
  `Option`, which rustc can't tell (E0283). What it fulfils is told as
  `then`'s is: of a function's result, `Option<Promise<()>>`, or an
  annotated promise, `let _: Promise<()> = ..`. Case N: the same chain JS
  makes.
- **`AsyncIterator::throw(&self, e: Option<&dyn IntoUnknown>) ->
  Promise<IteratorResult<T>>`** (rd584), TypeScript's optional `throw?(e?:
  any)`, asked as any optional method is, `js::is_function((iterator,
  AsyncIterator::<T>::throw))`, `typeof iterator.throw === "function"`. Its
  link name `throw` was the `throw` statement's form, `js::throw(value)`'s:
  **of a Rust method, of `self`, `throw` is its member**, as each method's
  link name is; of a function whose `this` is its parameter it's still the
  statement. `return?(value?)` isn't bound: no port calls it yet.
- **`js::AsyncIterable<T>`** (rd585), TypeScript's `AsyncIterable<T>`, a
  JS object's type as `AsyncIterator<T>` is, where `Iterable<T>` is a trait:
  Rust's own collections are given where an iterable is, and nothing of
  Rust's is async iterable. Its `async_iterator()` is
  `iterable[Symbol.asyncIterator]()`, a method keyed by a global's value:
  **a link name `"[path]"`, as TypeScript declares the member, is a
  method's call by that key**, `this[Symbol.asyncIterator](..)`, its `this`
  the receiver. The key is a global's path, as a static's is (ADR 1033), a quoted name, `['matchMedia']`, still refused as a method's,
  and only a method's, of `&self` or `this`: `"[1]"` and a key of a
  function of no receiver are refused, naming why.

React's ReactSymbols names the symbol `ASYNC_ITERATOR`, `const
ASYNC_ITERATOR = Symbol.asyncIterator`; a binding's key is the global it
names, so the port's JS reads `iterable[Symbol.asyncIterator]()`.

Held by `test/js-globals.test.ts`'s "Promise.prototype is its class's
prototype", "a then handler gives a promise or nothing, as void |
PromiseLike<void>", "an async iterator's throw is its optional method" and
"an async iterable's [Symbol.asyncIterator]() is its iterator", each run in
Node; the compiler's two forms by the mutations
`method-throw-a-statement`, `this-throw-a-member`, `keyed-method-a-call`,
`keyed-method-a-member`, `keyed-method-quoted`, `keyed-method-key-unchecked` and
`keyed-function-unrefused`. `coverage.txt` is unchanged: `prototype` and
the iterator protocol's interfaces aren't measured.
