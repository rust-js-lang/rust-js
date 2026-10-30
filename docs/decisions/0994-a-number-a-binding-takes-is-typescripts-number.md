# 0994. A number a binding takes is TypeScript's `number`, an `f64`

Status: Accepted; amended (below; the last: a nullable one's `Some`, and
a variable given one; and a counter, rd266). Amends [0024](0024-web-crate.md), whose WebIDL integer
parameters were Rust integers, an `unsigned long` a `u32`; and
[0283](0283-js-crate-covers-typescripts-es-library.md), whose typed
arrays took their element's type and `u32`/`i32` lengths, indices and
offsets, and to `set` and `from` only a slice of their element. Follows
[0967](0967-an-object-spread-a-splice-and-a-set-by-any-key.md)'s
amendment, `js::array::splice` of numbers, as one rule. Amended (the user's
decision): a number it gives is an `f64` too, and one it must be given is
any that's an `f64`, `impl Into<f64>` (below).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's (the program's) behavior is kept: a number the program gives is
given to JS as it is, and JS converts it as its own callers' would be.

## Context

React's ports gave numbers to bindings typed narrower than TypeScript:

- SchedulerProfiling logs `ms * 1000` and a task's id into an
  `Int32Array`, `eventLog.set([..], eventLogIndex)`, and grows it,
  `newEventLog.set(eventLog)` (gaps rd1, rd2). `set` took `&[i32]`, so
  the port wrote `(ms * 1000.0) as i32`, `$f64ToInt(.., -2147483648,
  2147483647)`, which saturates where the store wraps (a profile past ~35
  minutes logs 2147483647, upstream the wrapped value); another typed
  array couldn't be given at all; and its sizes were `u32`, each sum `>>>
  0`.
- react-dom-bindings' selection code gives `range.setStart(node, offset)`
  and `selection.extend(node, offset)` JS numbers (gap rd26). An
  `unsigned long` was a `u32`, so the port wrote `offset as u32`,
  `$f64ToInt(offset, 0, 4294967295)`, saturating where WebIDL's
  `ToUint32` wraps.

The cast was the program saying something upstream doesn't: another
result for a number out of the integer's range.

## Decision

**A number a binding takes is TypeScript's `number`, an `f64`**: every
WebIDL number a function or a setter takes, not a `boolean` (a dictionary
member it's given too), and each length, index, offset and element a
typed array, an `ArrayBuffer` or a `DataView` takes, a `BigInt64Array`'s
element its `i64` (TypeScript's `bigint`). The host converts it as WebIDL
or ECMAScript says, as it does a JS caller's:

```rust
range.set_start(node, offset);
event_log.set(&[1.0, ms * 1000.0, id], Some(index));
new_event_log.set(event_log, None);
```

```js
range.setStart(node, offset);
eventLog.set([1, ms * 1000, id], index);
newEventLog.set(eventLog);
```

- **What it gives is its own type** (superseded below: an `f64`), which the host's conversion made
  it: `length()` a `u32`, `node_type()` a `u16`, an element an `i32`.
  Rust's integer code, `0..view.length()`, a `match` of a constant,
  stays; giving one back is `f64::from(view.byte_offset())`, which is
  exact.
- **`set` and `from` take an `ArrayLike<T>`**, TypeScript's
  `ArrayLike<number>`: a slice, an array or a `Vec` of numbers each one
  `T` takes (`js::Takes`), or another typed array.
- **Written in `webapi/generate.ts`'s one place a number is typed**
  (`rustType`'s `NUMBERS`, of a parameter) and in
  `builtins/typed_arrays.ts`.

## Why

- **It's the type TypeScript gives, so the port's code is upstream's**:
  no cast where upstream has none, and none of a cast's other results.
- **One rule for every binding's number**, as `splice`'s was already,
  rather than an integer here and a number there.
- **It's exact**: every Rust number but a 64-bit integer converts to an
  `f64` exactly, so a program that had an integer gives the same value
  with `f64::from`.

## Alternatives

- **Results `f64` too, as TypeScript's are**: one type each way, but
  every integer a Rust program reads of the DOM, a length, a node's type,
  an index, cast back to count or match. Results are what the host
  converted already, so their integer type loses nothing.
- **A parameter generic over any number, `impl Into<f64>`**, so `item(0)`
  takes an integer literal: an optional one, `Option<impl Into<f64>>`,
  can't infer its type of `None`, the most common optional argument; and
  `splice`'s `f64` is the form already.
- **Keep the integer and convert in the lowering, `ToUint32` for a `u32`
  parameter**: the Rust would still have cast the `f64` first, which is
  where the value changed.

## Consequences

- An integer literal given to one is a float's, `labels.item(0.0)`,
  `int32_array::new(4.0)`, where it was `0`, `4`; its JS is the same.
- React's port's `binaryToComparableString` gives its view's offsets as
  `f64::from(view.byte_offset())`.
- `Atomics`' indices and values, and `js::string`'s `charAt`,
  `charCodeAt` and `substring`, take integers still: a Rust caller's
  index there is a loop's `u32`, which nothing casts. Each follows this
  rule where a port gives one a JS number.
- **A string read by any number is `js::string::get(text, index)`**,
  `text[index]`, `None` where JS reads `undefined`: React's
  FallbackCompositionState compares `endValue[endLength - end]`, an index
  that may be negative (gap ch1), where `charAt` gives `""`. Held by
  `test/lowering-strings.test.ts`'s "a string read by any number is
  text[index], None where JS reads undefined", run.

## What holds it

- `test/js-collections.test.ts`, "a typed array takes numbers and another
  typed array as TypeScript types them": SchedulerProfiling's growth and
  log, run, `3e9` stored wrapped as upstream's store does.
- `test/js-builtins.test.ts`'s typed array, view and atomics tests, and
  `test/web_forms.rs`, with numbers given as `f64`s.
- The guide's libraries page (libraries/README.md).
- A binding's signature has no mutation: the tests that call it with an
  `f64`, a typed array or a `ms * 1000.0` don't compile without it.

## Amendment (2026-10-28, the user's decision): what it gives is an `f64`, and a number it must be given is `impl Into<f64>`

With numbers taken as `f64`s and given as integers, a binding's result
couldn't be given back to another: react-in-rust's
binaryToComparableString, `uint8_array::new_with_buffer(view.buffer(),
Some(view.byte_offset()), Some(view.byte_length()))`, as upstream's `new
Uint8Array(view.buffer, view.byteOffset, view.byteLength)`, stopped
compiling, and a DOM loop read `list.item(f64::from(i))`.

The user's decision:

- **A number a binding gives is TypeScript's `number`, an `f64`**: every
  WebIDL number a getter, an operation, an iterable, a dictionary or a
  callback's argument gives (`size`, `onerror`'s line and column), and
  each WebIDL constant, `node::ELEMENT_NODE: f64 = 1.0`; a typed array's
  length, offsets, indices, elements, its callbacks' values and indices,
  and `BYTES_PER_ELEMENT`; `Atomics`' results. A `BigInt64Array`'s element
  is its `i64` still. Each pair agrees: what one gives, another takes.
- **A number it must be given is any number that's an `f64`, `impl
  Into<f64>`**: `list.item(i)` of a `u32`, `canvas.set_width(320)`,
  `int32_array::new(4)`, `range.set_start(node, offset)` of an `f64`. One
  API, std's own conversion, exact of every number it takes. An optional
  one is `Option<f64>`, whose `None` needs no type; a dictionary's field
  and a callback's argument, which no `impl` can be, are `f64`s.
  `webapi/generate.ts`'s `generic` and `builtins/typed_arrays.ts`'s
  `numbers` write it of a required `f64` parameter.
- **`Atomics`' value is `A::Element`**, an `f64` of a number array and
  an `i64` of a BigInt one: an `impl Into<i64>` would take an `i32`, which
  JS would be given as a number where a BigInt array takes a BigInt.

Consequences:

- A Rust loop over a list's indices reads it as an iterator,
  `node_list::iter(list)`, or counts its own, `for i in 0..n { list.item(i) }`
  of a `u32` `n`; `0..list.length()` is a range of `f64`s, which Rust
  doesn't step.
- `{}` of a number a binding gives is `$displayF64(x)`, Rust's text of an
  `f64`, as of any `f64`: `format!("HTTP {}", response.status())` is
  `` `HTTP ${$displayF64(response.status)}` ``, where a WebIDL integer's
  text is JS's own. A fact that a binding's number is an integer, which
  `{}` could show as JS does, is a gap still (closed by the amendment
  below: a WebIDL integer is `#[rust_js::whole]`).
- A binding named as a value, `js::is_function(Range::set_start)`, of a
  function taking `impl Into<f64>`, names no type for it, so rustc can't
  infer it: a path to one asks of its optional parameters, or of the
  member a value holds (ADR 0942's value form).
- react-in-rust's binaryToComparableString compiles as it was written,
  and SchedulerProfiling's sizes and items are numbers as upstream's are.

Held by `test/js-collections.test.ts`'s "a typed array takes numbers and
another typed array as TypeScript types them" (`set_index(0, 2.5)`,
`at(-1)`, `length()` given back), `test/js-builtins.test.ts`'s typed
array, view and atomics tests (`new_with_buffer(.., Some(view.byte_offset()),
Some(view.byte_length()))`), `test/web_forms.rs` (`set_width(320)`,
`item(0)`), the playground's compiler, and the examples' `item(n)`.

## Amendment (2026-10-29, the user's decision): a WebIDL integer is a whole number in its type's range

With every number a binding gives an `f64`, an integer WebIDL guarantees
was cast as any `f64` is: React DOM's `for i in 0..options.length() as
usize` read `$f64ToInt(options.length, 0, 4294967295)` where upstream
writes `i < options.length` (gap rd68), and the pilot's
`format!("HTTP {}", response.status())` `$displayF64(response.status)`
(fx1, #390).

The user's decision: **the bindings record that a WebIDL integer a
binding gives is a whole number in its type's range**, and the cast and
format owners consume it. No `len()` beside `length()`.

- **It's `#[rust_js::whole = "lo..=hi"]`** on a binding giving an `f64`:
  the least and the most whole number it gives, never `-0`, `NaN` or an
  infinity. `webapi/generate.ts` writes it (`wholeOf`, `INTEGERS`) of
  each getter's and operation's result WebIDL types an integer, through
  its typedefs: an `unsigned short`'s `"0..=65535"`, an `unsigned long`'s
  `"0..=4294967295"`, a `long long`'s its range too, the number JS is
  given its nearest, whole still. Not of a nullable one, an
  `Option<f64>`, `selectionStart`'s, whose `Some` isn't asked yet.
- **It replaces ADR 0302's `#[rust_js::position]`**, `indexOf`'s
  position or -1, now `whole = "-1..=4294967294"`: one attribute for the
  range of a binding's number. `position` is refused, naming its `whole`.
  `validate` refuses a `whole` on what isn't a binding giving an `f64`,
  or whose bounds aren't two integers in order below 10^21 in size.
- **A cast** (ADR 0302's `whole_casts`, the owner) of what such a call
  gives into an integer of 32 bits or fewer that holds each of its
  numbers is the number as it is: `options.length() as usize` is
  `options.length`. A variable it's given is known so as `indexOf`'s was.
  Into one that doesn't hold them all, `node.node_type() as i16` of an
  `unsigned short`, it saturates, as `as` does: Rust's result.
- **`{}` of it is JS's text of it** (`Known::Whole`, which
  `calls::given_by` sets on its call and `display_string` reads):
  `` `HTTP ${response.status}` ``. A whole number below 10^21 in size
  that isn't `-0` has one text in both, its digits. `{:?}` is Rust's
  `404.0` still, `$debugF64`; and `{}` of what's made of it,
  `response.status() / 8.0`, `$displayF64`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none, as the forms
are the same program: the value is what saturating gives of it, and its
text Rust's.

Consequences: 377 of webapi's functions say `whole`. A variable a
binding's integer was given, `let s = response.status(); format!("{s}")`,
is `$displayF64(s)` still: the fact is of the call's value, not yet of a
variable's.

Held by `test/webapi-bindings.test.ts`'s "a WebIDL integer is a whole
number in its type's range" (run, with `as u8`'s and `as i16`'s
saturation and the refusals) and `test/lowering-numbers.test.ts`'s
"an f64 known to be a whole number in range is cast as it is", and the
`whole-` mutations of `body_queries`, `calls`, `display/written` and
`bindings`.

## Amendment (2026-10-29): a nullable one's `Some`, and a variable given one

`#[rust_js::whole]` was of an `f64` alone, and known only of the call
itself: `let status = response.status(); format!("{status}")` was
`${$displayF64(status)}`, and a nullable WebIDL integer, `selectionStart`'s
`unsigned long?`, an `Option<f64>`, had none, so `if let Some(start) =
input.selection_start()` was `$displayF64(start)` and `start as u32`
`$f64ToInt(start, 0, 4294967295)`.

- **`#[rust_js::whole]` is of an `f64` or an `Option<f64>`**, its `Some`'s
  number (`bindings::whole_output`); webapi's generator writes it of each
  nullable WebIDL integer too (22 bindings).
- **A variable given what one gives is known whole where it's read**
  (`LocalFacts::wholes`, `mir_wholes`, read by `LocalFacts::variable`):
  each of whose writes in its MIR is such a binding's call, or a copy of
  another such variable, an `Option`'s `Some` number too, borrowed only
  to be read, as `format!` borrows it. Never a parameter, whose caller
  gives it first, nor one written any other way, in part, through a
  `&mut` or a pointer: `n = response.status()` of a parameter given `-0`
  is still `$displayF64(n)`, Rust's `-0`.
- **A cast of a `Some`'s number a `let` or an `if let` binds** is ranged
  as a `let` of the call is (`whole_casts`): `let Some(end) =
  input.selection_end() else { .. }; end as u32` is `end`.

Case: none; each such number is whole and in range, so JS's text and
Rust's `{}` agree, and the cast is the number. Held by
`test/webapi-bindings.test.ts`'s "a WebIDL integer is a whole number in
its type's range" (`status_text`, `selection`, `selection_end`,
`given_or_status`, `changed`, run); mutations `whole-option-unseen`,
`whole-variable-unknown`, `whole-some-payload-unread`,
`whole-parameter-trusted`, `whole-cast-some-binding-unranged`;
`whole-fact-unset` moved.

## Amendment: a counter is whole (rd266)

React DOM's Fizz instruction set names its nodes by a counter, `let
index = 0; .. '' + index ..; index++`, an `f64` in the port, whose `{}`
was `$displayF64(index)`, a helper Closure's ES5 build must take in.

**A local `f64` each of whose writes is a whole number from -2^53 to
2^53, not `-0`, a copy of another such, or such a number plus or minus
one, `index += 1.0`, is whole** (`mir_wholes`, whose `Known::Whole` its
reads have), so `{}` of it is JS's text, `` `${index}` ``. By induction
each value is whole and not `-0` (one plus or minus `-1`'s one is `+0`),
and none is further than 2^53 from zero, where adding one gives the same
number: JS's text of a whole number below 1e21 is its digits, as Rust's
is. The locals are found as the largest set whose writes are each of
the set or a whole number, as a counter's writes are of itself. One of
another step, `big += 4e20`, which passes 1e21, one that starts past
2^53, and `-0` keep `$displayF64`.

Case (ADR 0262): N, no difference: the text is Rust's for every value
the counter can hold.

Held by `test/corpus/counter_shown_as_js_text.rs` (native comparison of
counters up and down and of each kept case); mutations
`counter-past-2-53`, `counter-negative-zero`, `counter-any-step` and
`counter-of-itself-unknown` (`lower/mir/local_facts.ts`).
