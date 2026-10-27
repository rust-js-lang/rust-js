# Basics: values, functions, control flow

You know JavaScript. This page shows, for each thing you write every day in
JavaScript, the Rust you write instead, and the JavaScript rust-js writes
for it. Each section goes: the JS you'd write, the Rust, then rust-js's JS.

The Rust is ordinary Rust: what you'd write for a server or a CLI. Nothing
here is special to rust-js. Every example was compiled by rust-js as it is
today; the JS shown is what it writes, without its header comment and
source-map line.

Two rules hold on every page:

- **Names become camelCase.** `first_word` is `firstWord`, a field
  `in_stock` is `inStock` ([ADR 0573](../../decisions/0573-every-rust-name-is-camel-case-in-js.md)).
- **A `pub fn` is an `export function`.** A private one stays in its module.

Next: [Collections](collections.md), then [Advanced](advanced.md).

## Contents

- [`const` and `let`](#const-and-let)
- [Numbers](#numbers)
- [Strings and template literals](#strings-and-template-literals)
- [Functions and arrow functions](#functions-and-arrow-functions)
- [Default and optional parameters](#default-and-optional-parameters)
- [Objects and structs](#objects-and-structs)
- [Destructuring](#destructuring)
- [`if` as an expression](#if-as-an-expression)
- [`switch` and `match`](#switch-and-match)
- [Loops](#loops)
- [`null` and `undefined`: `Option`](#null-and-undefined-option)
- [Equality](#equality)
- [`typeof` checks](#typeof-checks)
- [Modules and imports](#modules-and-imports)

## `const` and `let`

JS has `const` for a binding you never reassign and `let` for one you do.
Rust has `let` and `let mut`.

```js
function total(prices, tax) {
  const rate = 1 + tax;
  let sum = 0;
  for (const price of prices) {
    sum += price * rate;
  }
  return sum;
}
```

```rust
pub fn total(prices: &[f64], tax: f64) -> f64 {
    let rate = 1.0 + tax;
    let mut sum = 0.0;
    for price in prices {
        sum += price * rate;
    }
    sum
}
```

```js
export function total(prices, tax) {
  const rate = 1 + tax;
  let sum = 0;
  for (const price of prices) {
    sum += price * rate;
  }
  return sum;
}
```

- `let` is `const`; `let mut` is `let`.
- The last expression of a block, with no `;`, is its value: `sum` is the
  `return`.
- A module-level `const` is a JS `const`, exported if it's `pub`:

```rust
pub const MAX_USERS: usize = 100;
const GREETING: &str = "Welcome";

pub fn banner(count: usize) -> String {
    let left = MAX_USERS - count;
    format!("{GREETING}! {left} seats left")
}
```

```js
export const MAX_USERS = 100;
const GREETING = "Welcome";

export function banner(count) {
  const left = (MAX_USERS - count) >>> 0;
  return `${GREETING}! ${left} seats left`;
}
```

- What a module makes where it loads, a `thread_local!`'s or a `static`'s
  value, reads another module's `const` by its name, `createCursor(NoLanes)`,
  as that module, which it imports, loads first; but where the two import
  each other, its value, `{ value: 100 }`, as the one loaded first runs
  before the other has made its `const`s. So it is where they import each
  other through a JS file of the program's own that one binds, which
  rust-js reads with what it imports: React's `NoEventPriority = 0`, where
  ReactFiberLane's hook imports ReactEventPriorities, which reads
  ReactFiberLane's `NoLane`. A function reads it by its name
  ([ADR 0722](../../decisions/0722-a-repr-enums-variant-is-its-number.md),
  [ADR 0031](../../decisions/0031-consts.md), amended).
- A `const` of literals is the value rustc computed, `4 * 1024` is
  `4096`. One written of other constants, the crate's or a library's, is
  as it's written, each read by its name, after what it reads; one that
  calls the crate's functions is rustc's value still. A `bool` one's read
  is a boolean, `ENABLED && count > 0`, with no `!!`
  ([ADR 0031](../../decisions/0031-consts.md), amended):

```rust
pub const SIZE: u32 = 4 * 1024;
pub const SYNC_LANE: i32 = 0b0010;
pub const DEFAULT_LANE: i32 = 0b1000;
pub const SYNC_UPDATE_LANES: i32 = SYNC_LANE | DEFAULT_LANE;
```

```js
export const SIZE = 4096;
export const SYNC_LANE = 0b0010;
export const DEFAULT_LANE = 0b1000;
export const SYNC_UPDATE_LANES = SYNC_LANE | DEFAULT_LANE;
```

- A `const` written in a function's body is a `const` there, where it's
  written, read by the function's closures and the functions written in
  it; before what reads it first where that's earlier, as JS reads a
  `const` only once it's made. One something outside the function reads
  too, a method of a type written in it, is the module's
  ([ADR 0031](../../decisions/0031-consts.md), amended):

```rust
pub const SYNC_LANE: i32 = 0b0010;
pub const DEFAULT_LANE: i32 = 0b1000;

pub fn includes_blocking_lane(lanes: i32) -> bool {
    const BLOCKING_LANES: i32 = SYNC_LANE | DEFAULT_LANE;
    (lanes & BLOCKING_LANES) != 0
}
```

```js
export const SYNC_LANE = 0b0010;
export const DEFAULT_LANE = 0b1000;

export function includesBlockingLane(lanes) {
  const BLOCKING_LANES = SYNC_LANE | DEFAULT_LANE;
  return (lanes & BLOCKING_LANES) !== 0;
}
```

Rust lets you shadow a name with a new `let`, `let input = input.trim();`.
Where nothing reads the old one after it, the new one is the same JS
variable, assigned, a `let` now. Where something still can, a closure made
before it, a loop's next turn, a drop at the block's end, JS can't declare
the name twice, so the new one gets a suffix, `count$1`:

```rust
pub fn label(input: &str) -> String {
    let label = input.trim();
    let mut label = label.to_uppercase();
    label.push('!');
    label
}

pub fn counted(start: u32) -> (u32, u32) {
    let count = start;
    let read = move || count;
    let count = count + 1;
    (read(), count)
}
```

```js
import { $trim } from "@rust-js/runtime";

export function label(input) {
  let label = $trim(input);
  label = label.toUpperCase();
  label += "!";
  return label;
}

export function counted(start) {
  const count = start;
  const read = () => count;
  const count$1 = (count + 1) >>> 0;
  return [read(), count$1];
}
```

Two places set the same variable or constant one after the other are one
statement, `a = b = v`, as JS chains them: each a variable or a plain
field, the second's object reading nothing the first sets. Set first,
the place on the right is set first, as in Rust
([ADR 0968](../../decisions/0968-two-places-set-the-same-value-are-one-assignment.md)):

```rust
use std::cell::Cell;

pub struct Hook {
    pub memoized_state: Cell<u32>,
    pub base_state: Cell<u32>,
}

pub fn mount_state(hook: &Hook, initial_state: u32) {
    hook.base_state.set(initial_state);
    hook.memoized_state.set(initial_state);
}
```

```js
export function mountState(hook, initialState) {
  hook.memoizedState = hook.baseState = initialState;
}
```

## Numbers

JS has one number type. Rust has one per size, and you pick: `f64` for a
JS-like number, an integer type where the value is a count or an index.

| Rust | JS | Kept in range by |
|---|---|---|
| `f64` | number | nothing: it is a JS number |
| `f32` | number | `Math.fround` after each operation |
| `i8` `i16` `i32` `u8` `u16` `u32` | number | `\| 0`, `>>> 0`, `& 255` |
| `usize` `isize` | number, 32 bits | as `u32`, `i32` |
| `i64` `u64` `i128` `u128` | BigInt, `5n` | `BigInt.asIntN`, `BigInt.asUintN` |

```rust
pub fn average(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

pub fn next_id(id: u32) -> u32 {
    id + 1
}

pub fn score(hits: i32, bonus: i32) -> i32 {
    hits * 10 + bonus
}

pub fn half(n: i32) -> i32 {
    n / 2
}

pub fn big(n: u64) -> u64 {
    n * 1_000_000
}

pub fn percent(part: u32, whole: u32) -> f64 {
    f64::from(part) / f64::from(whole) * 100.0
}
```

```js
export function average(xs) {
  return xs.reduce((a, b) => a + b, -0) / xs.length;
}

export function nextId(id) {
  return (id + 1) >>> 0;
}

export function score(hits, bonus) {
  return (((hits * 10) | 0) + bonus) | 0;
}

export function half(n) {
  return (n / 2) | 0;
}

export function big(n) {
  return BigInt.asUintN(64, n * 1000000n);
}

export function percent(part, whole) {
  return (part / whole) * 100;
}
```

What to know:

- **Integer division truncates**, `(n / 2) | 0`, as Rust's does.
- **Overflow wraps,** as a Rust release build's does: a `u8` of 100 plus
  200 is 44. Where you want something else, say so with std's methods:

```rust
pub fn checked(a: u32, b: u32) -> Option<u32> {
    a.checked_add(b)
}

pub fn saturating(a: u8, b: u8) -> u8 {
    a.saturating_sub(b)
}

pub fn to_byte(f: f64) -> u8 {
    f as u8
}

pub fn parse_age(s: &str) -> Result<u8, std::num::ParseIntError> {
    s.trim().parse()
}
```

```js
import { $checked, $f64ToInt, $parseInt, $trim } from "@rust-js/runtime";

export function checked(a, b) {
  return $checked(a + b, 0, 4294967295);
}

export function saturating(a, b) {
  return Math.max(a - b, 0);
}

export function toByte(f) {
  return $f64ToInt(f, 0, 255);
}

export function parseAge(s) {
  return $parseInt($trim(s), 0, 255);
}
```

- **Casts are Rust's:** `300.7 as u8` is `255`, `NaN as u8` is `0`.
- **`parse` is Rust's grammar,** not `Number()`'s: `"42"` parses, `" 42"`
  is an `Err`, so trim first.
- **Dividing an integer by zero panics,** with Rust's message. An `f64`
  divided by zero is `Infinity`, as in JS.
- **A step of one is `++`:** a statement `n += 1` is `n++`, and `n -= 1`
  is `n--`
  ([ADR 0631](../../decisions/0631-a-step-of-one-is-plus-plus.md)); a
  bitwise chain of its target, `flags = flags | UPDATE | LAYOUT`, is
  `flags |= UPDATE | LAYOUT` (ADR 0631, amended); a
  value read and then stepped, `let id = n; n = id + 1.0;`, is `const id
  = n++;`
  ([ADR 0948](../../decisions/0948-a-value-read-then-stepped-is-plus-plus.md)). An
  integer's keeps its wrap, `n = (n + 1) >>> 0`, unless what comes before
  shows it can't wrap: below a length, `i < xs.len()` or `i + 1 <
  js::string::length(s)`, or a constant, `i <= LIMIT`, `i += 1` is
  `i++`, in a labeled block too
  ([ADR 0720](../../decisions/0720-a-labeled-block-and-a-named-constant-in-a-bodys-ranges.md)).
  Below another unsigned integer, `for k in 0..n` or `k < n`, `n - k`
  is at least 1, so `n - k - 1` is JS's own; and `line + 1`, even
  wrapped, is at most `line + 1`, so below it `line - k` can't wrap
  ([ADR 0576](../../decisions/0576-what-a-bodys-integers-can-be.md)).
  A JS string's `length` is
  at most `2^31 - 1`, as every engine's is, so two steps below it don't
  wrap either, nor does the sum of two lengths; a third, or a `- 1`,
  keeps its wrap
  ([ADR 0632](../../decisions/0632-a-js-strings-length-is-below-u32-max.md),
  [ADR 0682](../../decisions/0682-a-js-strings-length-is-at-most-int32-max.md)):

```rust
pub fn count_below(xs: &[u32], limit: u32) -> usize {
    let mut i = 0;
    while i < xs.len() && xs[i] < limit {
        i += 1;
    }
    i
}

pub fn placeholders(format: &str) -> u32 {
    let mut n = 0;
    let mut i = 0;
    while i + 1 < js::string::length(format) {
        if js::string::char_code_at(format, i) == 37.0 {
            n += 1;
            i += 1;
        }
        i += 1;
    }
    n
}

pub fn fits(str: &str, substr: &str) -> bool {
    js::string::length(substr) < 10 && js::string::length(str) + js::string::length(substr) < 40
}
```

```js
export function countBelow(xs, limit) {
  let i = 0;
  while (i < xs.length && xs[i] < limit) {
    i++;
  }
  return i;
}

export function placeholders(format) {
  let n = 0;
  let i = 0;
  while (i + 1 < format.length) {
    if (format.charCodeAt(i) === 37) {
      n = (n + 1) >>> 0;
      i++;
    }
    i++;
  }
  return n;
}

export function fits(str, substr) {
  return substr.length < 10 && str.length + substr.length < 40;
}
```
- **A bit set is written with JS's operators,** each assignment its
  compound, `lanes &= ~lane`, of a field or a `Cell`'s too, and of an
  item, its index checked, `v[$at(v, i)] |= lane`
  ([ADR 0894](../../decisions/0894-a-bitwise-assignment-is-its-compound.md)).
  An `i32`'s `<<` is JS's, and what a bitwise operator takes needs no
  wrap of its own, `lanes & -lanes`, as JS's takes it modulo 2^32 anyway;
  elsewhere a wrap stays, `-lanes | 0` of `i32::MIN`
  ([ADR 0895](../../decisions/0895-an-operand-of-a-bitwise-operator-needs-no-wrap.md)).
  So `lanes.isolate_lowest_one()`, std's `lanes & lanes.wrapping_neg()`,
  is `lanes & -lanes`
  ([ADR 0156](../../decisions/0156-integer-bits-and-bytes.md)); and a
  test of bits against `0` is their truth, `if (lane & SYNC_LANE)`, and
  so is any integer's `!= 0`, `while (lanes)`, never `NaN`; its `== 0`
  stays `i === 0`, as does a remainder's `n % 2 === 0` and a float's
  `x !== 0`
  ([ADR 0896](../../decisions/0896-a-test-of-bits-against-zero-is-their-truth.md), amended):

```rust
use std::cell::Cell;

pub struct Root {
    pub pending_lanes: Cell<i32>,
}

pub fn remove_lane(mut lanes: i32, lane: i32) -> i32 {
    lanes &= !lane;
    lanes
}

pub fn mark_updated(root: &Root, lane: i32) {
    root.pending_lanes.set(root.pending_lanes.get() | lane);
}

pub fn lane_of(index: u32) -> i32 {
    1 << index
}

pub fn highest_priority_lane(lanes: i32) -> i32 {
    lanes.isolate_lowest_one()
}

pub const SYNC_LANE: i32 = 0b10;

pub fn label_for_lane(lane: i32) -> Option<&'static str> {
    if lane & SYNC_LANE != 0 {
        return Some("Sync");
    }
    None
}

pub fn count_lanes(mut lanes: i32) -> u32 {
    let mut count = 0;
    while lanes != 0 {
        lanes &= lanes - 1;
        count += 1;
    }
    count
}
```

```js
export function removeLane(lanes, lane) {
  lanes &= ~lane;
  return lanes;
}

export function markUpdated(root, lane) {
  root.pendingLanes |= lane;
}

export function laneOf(index) {
  return 1 << index;
}

export function highestPriorityLane(lanes) {
  return lanes & -lanes;
}

export const SYNC_LANE = 0b10;

export function labelForLane(lane) {
  if (lane & SYNC_LANE) {
    return "Sync";
  }
}

export function countLanes(lanes) {
  let count = 0;
  while (lanes) {
    lanes &= lanes - 1;
    count = (count + 1) >>> 0;
  }
  return count;
}
```
- **A literal keeps its base and its exponent**, `0xff`, `0b1010`, `1e4`,
  without Rust's type suffix, separators and a decimal's leading zeros;
  any other is JS's digits,
  `5000` of `5000.0`
  ([ADR 0627](../../decisions/0627-a-number-literal-keeps-its-base-and-exponent.md)):

```rust
pub const OWNER_STACK_LIMIT: f64 = 1e4;

pub fn low_byte(n: u32) -> u32 {
    n & 0xffu32
}
```

```js
export const OWNER_STACK_LIMIT = 1e4;

export function lowByte(n) {
  return n & 0xff;
}
```

The whole story, with each difference from native Rust, is in
[Semantics](../semantics.md) ([ADR 0011](../../decisions/0011-numbers.md), [ADR 0086](../../decisions/0086-64-bit-integers.md)).

## Strings and template literals

A `String` and a `&str` are both a JS string. `format!` is a template
literal.

```js
function label(name, count) {
  return `${name} has ${count} items`;
}

function price(cents) {
  return `$${Math.trunc(cents / 100)}.${String(cents % 100).padStart(2, "0")}`;
}
```

```rust
pub fn label(name: &str, count: u32) -> String {
    format!("{name} has {count} items")
}

pub fn price(cents: u32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

pub fn shout(s: &str) -> String {
    s.to_uppercase() + "!"
}

pub fn log_user(name: &str, age: u8) {
    println!("user {name}, age {age}");
}
```

```js
export function label(name, count) {
  return `${name} has ${count} items`;
}

export function price(cents) {
  return `$${(cents / 100) >>> 0}.${String(cents % 100).padStart(2, "0")}`;
}

export function shout(s) {
  return s.toUpperCase() + "!";
}

export function logUser(name, age) {
  console.log(`user ${name}, age ${age}`);
}
```

- `{name}` names a variable in scope; `{}` takes the next argument.
- Width, fill and precision are Rust's: `{:02}`, `{:>5}`, `{:.1}`.
- `println!` is `console.log`, `eprintln!` is `console.error`.
- `{:?}` is Rust's debug form: a string in quotes, a struct with its
  name and fields. Use it for logging, not for UI text.

`s.len()` is the length in UTF-8 bytes, not `s.length`. See
[Collections: strings](collections.md#strings).

A string literal is written as its source writes it. A long one split
over lines, which JS joins with `+`, is std's `concat!` of its pieces;
one written across lines is a template literal of them:

```js
export const OMITTED_PROP_ERROR =
  'This object has been omitted by React in the console log ' +
  'to avoid sending too much data from the server. Try logging smaller ' +
  'or more specific objects.';

export const PAGE = `<main>
  <h1>Hi</h1>
</main>`;

export function formatProdErrorMessage(code, url) {
  return (
    `Minified React error #${code}; visit ${url} for the full message or ` +
    'use the non-minified dev environment for full errors and additional ' +
    'helpful warnings.'
  );
}
```

```rust
pub const OMITTED_PROP_ERROR: &str = concat!(
    "This object has been omitted by React in the console log ",
    "to avoid sending too much data from the server. Try logging smaller ",
    "or more specific objects."
);

pub const LAZY_HINT: &str = concat!(
    "const MyComponent = lazy(() => imp",
    // Break up imports to avoid accidentally parsing them as dependencies.
    "ort('./MyComponent'))"
);

pub const PAGE: &str = r#"<main>
  <h1>Hi</h1>
</main>"#;

pub fn version() -> &'static str {
    concat!("v", 2, ".", 1)
}

pub fn format_prod_error_message(code: u32, url: &str) -> String {
    format!(
        concat!(
            "Minified React error #{}; visit {} for the full message or ",
            "use the non-minified dev environment for full errors and additional ",
            "helpful warnings."
        ),
        code,
        url
    )
}
```

```js
export const OMITTED_PROP_ERROR =
  "This object has been omitted by React in the console log " +
  "to avoid sending too much data from the server. Try logging smaller " +
  "or more specific objects.";

export const LAZY_HINT =
  "const MyComponent = lazy(() => imp" +
  // Break up imports to avoid accidentally parsing them as dependencies.
  "ort('./MyComponent'))";

export const PAGE = `<main>
  <h1>Hi</h1>
</main>`;

export function version() {
  return "v" + "2" + "." + "1";
}

export function formatProdErrorMessage(code, url) {
  return (
    `Minified React error #${code}; visit ${url} for the full message or ` +
    "use the non-minified dev environment for full errors and additional " +
    "helpful warnings."
  );
}
```

- Each of `concat!`'s arguments is its own JS string, in order: a
  `char`'s, a number's or a `bool`'s too, as `concat!` writes it, `"2"`.
  A comment between them is before the piece it's written before.
- A `\` ending a line continues one literal, as it does in Rust: one JS
  string.
- A space one can't tell from `' '`, `\u{2007}` or `\u{a0}`, a mark one
  can't see, `\u{200b}`, and a control, `\u{1}`, are written as their
  escapes, their hex in lowercase, `"\u2007\xa0"`, `"\x01"`, in a string
  and a template; what joins or varies an emoji, `\u{200d}` and
  `\u{fe0f}`, is as it is ([ADR
  0732](../../decisions/0732-a-space-one-cant-see-is-its-escape.md),
  amended).
- A `concat!` of another macro, `concat!("v", env!("VERSION"))`, is the
  one string rustc makes of it.
- A format string of a `concat!`, `format!(concat!(..), ..)` or
  `println!`'s, is its pieces too: each a template literal where it holds
  a placeholder, and otherwise a string. One whose placeholder or `{{`
  is split across two pieces, `concat!("{", "}")`, is one, as
  only the string they make has it; so is one rustc writes a literal
  argument into, `format!(concat!("a ", "{}"), "b")`.
- A format argument written across lines, an `if .. { .. } else { .. }`,
  may break its template literal's line after its `${` where the line is
  too long for one, as Prettier breaks one written so; one written on one
  line, or that fits, stays on the template's line.

## Functions and arrow functions

A `fn` is a `function`; a closure, `|n| n * 2`, is an arrow function.
Parameter and return types are written out; inside a body, Rust infers.

```js
function add(a, b) {
  return a + b;
}

function applyTwice(x) {
  const double = (n) => n * 2;
  return double(double(x));
}
```

```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn apply_twice(x: i32) -> i32 {
    let double = |n: i32| n * 2;
    double(double(x))
}
```

```js
export function add(a, b) {
  return (a + b) | 0;
}

export function applyTwice(x) {
  const double = (n) => (n * 2) | 0;
  return double(double(x));
}
```

A closure of nothing, `()`, whose body you write as one call's value, no
`;`, is that call's arrow, `() => log(n)`, where the call's JS gives
`undefined` too: a function of the crate's, a closure of its own, or a
binding that says so, a web API WebIDL says gives `undefined` among them.
A comment before the call is after the `=>`, as Prettier writes it. Written
as a statement, `|| { log(n); }`, it's a block, and so is one of a call
whose JS gives a value Rust's `()` doesn't, `|| v.push(n)`'s array's
count
([ADR 0883](../../decisions/0883-a-closure-of-its-calls-value-is-an-arrow-of-it.md)):

```rust
use std::cell::RefCell;

fn log(seen: &RefCell<Vec<u32>>, n: u32) {
    seen.borrow_mut().push(n);
}

fn later(f: impl FnOnce()) {
    f();
}

pub fn run(seen: &RefCell<Vec<u32>>) {
    later(|| log(seen, 1));
    later(|| {
        // Logged second.
        log(seen, 2)
    });
    later(|| {
        log(seen, 3);
    });
}
```

```js
function log(seen, n) {
  seen.push(n);
}

function later(f) {
  f();
}

export function run(seen) {
  later(() => log(seen, 1));
  later(() =>
    // Logged second.
    log(seen, 2),
  );
  later(() => {
    log(seen, 3);
  });
}
```

Closures that capture and change their surroundings are in
[Advanced: closures](advanced.md#closures).

## Default and optional parameters

Rust has no default parameters. A parameter a caller may leave out is an
`Option`, and `unwrap_or` gives its default. A JS caller passes
`undefined`, or nothing.

```js
function greeting(name = "stranger") {
  return `Hello, ${name}`;
}

function repeat(text, times = 1) {
  return text.repeat(times);
}
```

```rust
pub fn greeting(name: Option<&str>) -> String {
    let name = name.unwrap_or("stranger");
    format!("Hello, {name}")
}

pub fn repeat(text: &str, times: Option<usize>) -> String {
    text.repeat(times.unwrap_or(1))
}
```

```js
export function greeting(name) {
  name ??= "stranger";
  return `Hello, ${name}`;
}

export function repeat(text, times) {
  return text.repeat(times ?? 1);
}
```

`None` is `undefined`, so `unwrap_or` is `??`. A Rust caller writes
`greeting(None)` or `greeting(Some("Ada"))`.

A trailing `None` after another argument, given to the crate's own
function, is left out, `repeat("ab")`, and so is one given through a `fn`
pointer that may only be the crate's functions and closures, a
dispatcher's. Given to what may be JS's, a binding's, a `dyn Fn`'s, or a
pointer JS may have made, it's `undefined`, as a JS function may count its
arguments ([ADR 0330](../../decisions/0330-undefined-given-to-js.md),
amended):

```rust
pub struct Dispatcher {
    pub repeat: fn(&str, Option<usize>) -> String,
}

static DISPATCHER: Dispatcher = Dispatcher { repeat };

pub fn twice() -> String {
    (DISPATCHER.repeat)("ab", None)
}

pub fn twice_by(repeat: &dyn Fn(&str, Option<usize>) -> String) -> String {
    repeat("ab", None)
}
# pub fn repeat(text: &str, times: Option<usize>) -> String {
#     text.repeat(times.unwrap_or(1))
# }
```

```js
const DISPATCHER = { repeat };

export function twice() {
  return DISPATCHER.repeat("ab");
}

export function twiceBy(repeat) {
  return repeat("ab", undefined);
}
export function repeat(text, times) {
  return text.repeat(times ?? 1);
}
```

## Objects and structs

A JS object with a fixed set of fields is a Rust `struct`. Declare it once;
rustc checks every use. In JS it's a plain object literal.

```js
const origin = () => ({ x: 0, y: 0 });
const make = (x, y) => ({ x, y });
function shift(p, dy) {
  p.y += dy;
}
```

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub fn origin() -> Point {
    Point { x: 0.0, y: 0.0 }
}

pub fn make(x: f64, y: f64) -> Point {
    Point { x, y }
}

pub fn shift(p: &mut Point, dy: f64) {
    p.y += dy;
}

pub fn moved(p: &Point, dx: f64) -> Point {
    Point {
        x: p.x + dx,
        ..p.clone()
    }
}
```

```js
export function origin() {
  return { x: 0, y: 0 };
}

export function make(x, y) {
  return { x, y };
}

export function shift(p, dy) {
  p.y += dy;
}

export function moved(p, dx) {
  return {
    x: p.x + dx,
    y: { ...p }.y,
  };
}
```

- `&mut Point` is the object itself, changed in place. `&Point` is the
  object too; rustc won't let you change it.
- `..p.clone()` is Rust's spread, `{ ...p, x }`; the rest of the fields
  come from `p`.
- The object lists the fields in the order you write them, which is the
  order Rust evaluates them in: `Point { y: f(), x: g() }` is `{ y: f(),
  x: g() }`. What `..base` fills comes after them, or is spread before
  them, `{ ...p, y, x }`
  ([ADR 0674](../../decisions/0674-a-struct-literals-fields-are-in-the-order-written.md)).
  `{:?}` still shows them as the struct declares them.
- A struct literal written across lines, as rustfmt writes one too long
  for a line, is an object on lines of its own; one written on one line is
  on one line where it fits
  ([ADR 0835](../../decisions/0835-an-object-is-across-lines-where-its-literal-is.md)).
- Methods go in an `impl` block: [Advanced: classes](advanced.md#classes).
- An object used as a dictionary, with keys only known at run time, is a
  `HashMap`: [Collections: maps](collections.md#map-and-set).

How a struct looks to JavaScript that calls your crate is in
[Structs, enums and traits as JavaScript sees them](../interop/objects.md).

## Destructuring

Rust's `let` takes a pattern, as JS's destructuring does, and a function
parameter can be one too.

```js
function describe(user) {
  const { name, age } = user;
  return `${name} (${age})`;
}

function swap([a, b]) {
  return [b, a];
}
```

```rust
pub struct User {
    pub name: String,
    pub age: u32,
    pub email: Option<String>,
}

pub fn describe(user: &User) -> String {
    let User { name, age, .. } = user;
    format!("{name} ({age})")
}

pub fn contact(User { name, email, .. }: &User) -> String {
    match email {
        Some(email) => format!("{name} <{email}>"),
        None => name.clone(),
    }
}

pub fn swap(pair: (i32, i32)) -> (i32, i32) {
    let (a, b) = pair;
    (b, a)
}

pub fn first_two(items: &[String]) -> Option<String> {
    let [first, second, ..] = items else {
        return None;
    };
    Some(format!("{first} and {second}"))
}
```

```js
export function describe(user) {
  const { name, age } = user;
  return `${name} (${age})`;
}

export function contact({ name, email }) {
  if (email != null) {
    return `${name} <${email}>`;
  }
  return name;
}

export function swap(pair) {
  const [a, b] = pair;
  return [b, a];
}

export function firstTwo(items) {
  if (items.length < 2) {
    return;
  }
  const [first, second] = items;
  return `${first} and ${second}`;
}
```

- `..` skips the fields you don't name. Rust makes you say so.
- A tuple, `(i32, i32)`, is a JS array, `[a, b]`.
- `let ... else` handles the pattern not matching: here, fewer than two
  items. Its `else` is a guard, first, where it's written, before a
  `return` written after it too, or a function's tail its value is alike
  ([ADR 0879](../../decisions/0879-an-if-and-a-closures-captures-read-as-written.md), amended).
- A pattern is one destructuring, a struct's of `Cell` fields too: `let
  Fiber { tag, r#type, .. } = fiber;` is `const { tag, type } = fiber;`.
  A `let` of one field, `let x = p.x;`, is a statement of its own, `const
  x = p.x;`, however many follow it, each named as its field or not, as
  written
  ([ADR 0598](../../decisions/0598-a-temporary-is-no-part-of-a-names-pattern.md), amended).

## `if` as an expression

An `if` has a value, so it's JS's `?:` as well as its `if`.

```js
const rate = member ? 0.01 : 0.03;
```

```rust
pub fn sign(n: i32) -> &'static str {
    if n < 0 {
        "negative"
    } else if n == 0 {
        "zero"
    } else {
        "positive"
    }
}

pub fn fee(member: bool, amount: f64) -> f64 {
    let rate = if member { 0.01 } else { 0.03 };
    amount * rate
}
```

```js
export function sign(n) {
  if (n < 0) {
    return "negative";
  }
  return n === 0 ? "zero" : "positive";
}

export function fee(member, amount) {
  const rate = member ? 0.01 : 0.03;
  return amount * rate;
}
```

A variable given its value by a chain of more arms, each a value alone
(a string or a number literal, or a name), is one nested conditional, a
`const`; a chain with an arm that's more, a call, an object, a conditional
of its own, is given it by the chain, each arm its own
([ADR 0969](../../decisions/0969-a-variables-value-of-a-chain-of-arms-is-the-chain.md)):

```rust
pub fn color(duration: f64) -> String {
    let color = if duration < 5.0 {
        "light"
    } else if duration < 10.0 {
        "plain"
    } else if duration < 100.0 {
        "dark"
    } else {
        "error"
    };
    format!("{color}!")
}

pub fn shade(duration: f64, hydrated: bool) -> String {
    let shade = if duration < 5.0 {
        if hydrated { "tertiary" } else { "primary" }
    } else if duration < 100.0 {
        "dark"
    } else {
        "error"
    };
    format!("{shade}!")
}
```

```js
export function color(duration) {
  const color =
    duration < 5 ? "light" : duration < 10 ? "plain" : duration < 100 ? "dark" : "error";
  return `${color}!`;
}

export function shade(duration, hydrated) {
  let shade;
  if (duration < 5) {
    shade = hydrated ? "tertiary" : "primary";
  } else if (duration < 100) {
    shade = "dark";
  } else {
    shade = "error";
  }
  return `${shade}!`;
}
```

An `if` condition must be a `bool`. There's no truthiness: write
`!s.is_empty()`, `n != 0`, `opt.is_some()`.

A let chain's value, each branch one value, reads what its `let` binds
first, only where the tests before it hold, and once, then is one `?:`
of it, as an `if let`'s is. A thread-local's is its module's `const`s
([ADR 0870](../../decisions/0870-a-let-chains-value-reads-its-binding-first.md)):

```rust
fn discount(code: &str) -> Option<f64> {
    if code == "SAVE10" { Some(0.1) } else { None }
}

pub fn total(member: bool, code: &str, amount: f64) -> f64 {
    let saved = if member && let Some(rate) = discount(code) {
        amount * rate
    } else {
        0.0
    };
    amount - saved
}
```

```js
function discount(code) {
  if (code === "SAVE10") {
    return 0.1;
  }
}

export function total(member, code, amount) {
  const rate = member ? discount(code) : undefined;
  const saved = rate != null ? amount * rate : 0;
  return amount - saved;
}
```

An `if` statement is the one you wrote: its test as written and its
branches in their order, `if !queue.is_empty() { .. } else { .. }`'s
`queue.length !== 0` first; an `else { break }` after a branch that goes
on stays an `else`; a `!( .. )` you wrote of an `&&` or `||` stays one,
`!(DISABLE && tag == 0)`, but a `!matches!(status, Some("fulfilled" |
"rejected"))` is each alternative's negated test, `status !== "fulfilled"
&& status !== "rejected"`; and so is an `if`'s value, `if a == 0 || a > b
{ a } else { b }` being `a === 0 || a > b ? a : b`
([ADR 0879](../../decisions/0879-an-if-and-a-closures-captures-read-as-written.md)).
A branch that returns needs no `else` after it where the `if` ends the
function, as ESLint's `no-else-return` has it; but an `else` you wrote
after it stays where statements follow the `if`, `place`'s, and an `else
if` after an arm that goes on stays one chain; a function's last `match`
or `if` of arms that give `()` is its arms, `if (current == null) { ..
} else { .. }`, with no `return;` at their ends
([ADR 0879](../../decisions/0879-an-if-and-a-closures-captures-read-as-written.md), amended):

```rust
pub fn describe(kind: u32, ready: bool, dev: bool) -> String {
    let label;
    if kind == 1 {
        label = "element".to_string();
    } else if kind == 2 {
        return "client".to_string();
    } else if dev && ready {
        label = "<jsx>".to_string();
    } else {
        label = "{...}".to_string();
    }
    label
}

pub fn place(tag: u8, log: &mut Vec<u8>) {
    if tag == 5 {
        log.push(tag);
        return;
    } else if tag == 4 {
        // A portal's children are placed on their own.
        return;
    }
    log.push(0);
}
```

```js
export function describe(kind, ready, dev) {
  let label;
  if (kind === 1) {
    label = "element";
  } else if (kind === 2) {
    return "client";
  } else {
    label = dev && ready ? "<jsx>" : "{...}";
  }
  return label;
}

export function place(tag, log) {
  if (tag === 5) {
    log.push(tag);
    return;
  } else if (tag === 4) {
    // A portal's children are placed on their own.
    return;
  }
  log.push(0);
}
```

A branch of only comments is kept, with them, where it says why it does
nothing; one of nothing at all, `else {}`, is left out
([ADR 0753](../../decisions/0753-a-branch-of-only-comments-keeps-them.md)):

```rust
pub fn logged(depth: u32, value: Option<u32>, out: &mut Vec<u32>) {
    if depth >= 3 {
        // Too deep: logged by the caller.
    } else if let Some(value) = value {
        out.push(value);
    }
}
```

```js
export function logged(depth, value, out) {
  if (depth >= 3) {
    // Too deep: logged by the caller.
  } else if (value != null) {
    out.push(value);
  }
}
```

So is a comment after a body's last code, or before the `None` a function
gives at its end, which is no `return;` of its own: it's at the end of the
body, as written
([ADR 0862](../../decisions/0862-a-comment-at-a-bodys-end-is-there.md)):

```rust
pub fn found(n: u32, out: &mut Vec<u32>) -> Option<u32> {
    if n > 3 {
        out.push(n);
        // Logged; found.
        return Some(n);
    }
    // Nothing found.
    None
}
```

```js
export function found(n, out) {
  if (n > 3) {
    out.push(n);
    // Logged; found.
    return n;
  }
  // Nothing found.
}
```

A `catch_unwind` closure's body is a `try`'s block, and an `Err` arm's a
`catch`'s, each with the comments it ends with at its end:

```rust
use std::panic::catch_unwind;

pub fn checked(f: fn(u32) -> u32) -> u32 {
    if let Err(_e) = catch_unwind(|| {
        f(1);
        f(2);
        // Called twice.
    }) {
        println!("threw");
        // Nothing else to do.
    }
    f(3)
}
```

```js
export function checked(f) {
  try {
    f(1);
    f(2);
    // Called twice.
  } catch (_e) {
    console.log("threw");
    // Nothing else to do.
  }
  return f(3);
}
```

## `switch` and `match`

`match` is `switch` that can't fall through, can't forget a case, and has a
value. rustc rejects a `match` that misses one.

```js
function dayName(day) {
  switch (day) {
    case 0: return "Sunday";
    case 6: return "Saturday";
    case 1: case 2: case 3: case 4: case 5: return "weekday";
    default: return "unknown";
  }
}
```

```rust
pub fn day_name(day: u8) -> &'static str {
    match day {
        0 => "Sunday",
        6 => "Saturday",
        1 | 2 | 3 | 4 | 5 => "weekday",
        _ => "unknown",
    }
}

pub fn weekend(day: u8) -> bool {
    match day {
        0 | 6 => true,
        1..=5 => false,
        _ => panic!("no day {day}"),
    }
}

pub fn http_status(code: u16) -> String {
    match code {
        200 | 204 => "ok".to_string(),
        404 => "not found".to_string(),
        c if c >= 500 => format!("server error {c}"),
        c => format!("status {c}"),
    }
}
```

```js
export function dayName(day) {
  switch (day) {
    case 0:
      return "Sunday";
    case 6:
      return "Saturday";
    case 1:
    case 2:
    case 3:
    case 4:
    case 5:
      return "weekday";
    default:
      return "unknown";
  }
}

export function weekend(day) {
  if (day === 0 || day === 6) {
    return true;
  }
  if (day < 1 || day > 5) {
    throw new Error(`no day ${day}`);
  }
  return false;
}

export function httpStatus(code) {
  if (code === 200 || code === 204) {
    return "ok";
  }
  if (code === 404) {
    return "not found";
  }
  if (code >= 500) {
    return `server error ${code}`;
  }
  return `status ${code}`;
}
```

`|` is several cases in one arm, `1..=5` a range, and `if` after a pattern
a guard. A `match` of three cases or more, each constant of its patterns
one and a last `_` one, each arm a constant or a `|` of them
(a literal, a `const`, an enum's variant of no field, the registry's
symbol, or a `Some` of them, [ADR
0634](../../decisions/0634-a-registered-symbol-is-a-const.md)), and at most
a last `_`, or a last name, `current => current`, which is the value, is a
`switch`: `|`'s constants are stacked `case`s, `_` or the name is the
`default`, left out where it does nothing, and each case ends in `break;`
where it'd go on, the case's own where it's left for what follows the
`switch` ([ADR 0731](../../decisions/0731-a-cases-break-past-the-switch-is-its-own.md)),
in a block `{ .. }` where it declares a name. An arm with
a guard is a case that tests it, where what Rust tries when the guard
fails does nothing: each later arm that could match its constants is a
`_ => {}`, as below; or where it's the next arm, which names each of its
constants and more, a case that falls through to the next's, `// Fallthrough`,
a comment written before that next arm before it and its labels, as React
writes one, and no `// Fallthrough` where that comment says it, `//
falls through`
([ADR 0905](../../decisions/0905-a-guarded-arm-falls-to-the-next.md), amended);
or where it's a last `_` that does something, a case that falls through
to the `default`
([ADR 0636](../../decisions/0636-a-guarded-arm-is-a-case.md), amended). Any other `match`, of a range, a binding of a
pattern's part or before the last arm, a guard a later arm goes on from, or
of two cases, is an `if` chain ([ADR 0014](../../decisions/0014-match-lowering.md), [ADR
0629](../../decisions/0629-a-match-of-constants-is-a-switch.md)); and one
whose value is read in place, `f(match n { .. })`, is a `?:`, as JS has
no `switch` expression. An arm that names a `const` tests it by its name,
and a comment before an arm is before its body:

```rust
pub const IMMEDIATE: u8 = 1;
pub const IDLE: u8 = 5;

pub fn timeout(level: u8) -> f64 {
    match level {
        // Times out immediately
        IMMEDIATE => -1.0,
        // Never times out
        IDLE => 1073741823.0,
        _ => 5000.0,
    }
}
```

```js
export const IMMEDIATE = 1;
export const IDLE = 5;

export function timeout(level) {
  switch (level) {
    case IMMEDIATE:
      // Times out immediately
      return -1;
    case IDLE:
      // Never times out
      return 1073741823;
    default:
      return 5000;
  }
}
```

A `match` of an `Option`'s text, `Some("fulfilled")`, or of its number
or `char`, `Some(7)`, is a `switch` of the value: no case takes `None`,
which goes to the `default`, as it does in Rust, with no test that it's
there before it, and no labeled block where an arm leaves one early. A last `_`'s statements are the `default`'s, and what follows the
`match` follows the `switch`
([ADR 0629](../../decisions/0629-a-match-of-constants-is-a-switch.md), amended):

```rust
pub fn resolve(status: Option<&str>, log: &mut Vec<String>) -> u32 {
    match status {
        Some("fulfilled") => return 1,
        Some("rejected") => return 2,
        _ => {
            log.push("waiting".to_string());
            match status {
                Some("pending") => return 3,
                Some("stale") => log.push("stale".to_string()),
                _ => {}
            }
        }
    }
    0
}
```

```js
export function resolve(status, log) {
  switch (status) {
    case "fulfilled":
      return 1;
    case "rejected":
      return 2;
    default:
      log.push("waiting");
      switch (status) {
        case "pending":
          return 3;
        case "stale":
          log.push("stale");
      }
  }
  return 0;
}
```

A guarded arm, as clippy's `collapsible_match` asks an `if` in an arm to
be written, is a case whose statements test the guard:

```rust
pub const FRAGMENT: u8 = 1;
pub const PROFILER: u8 = 2;
pub const TRACING_MARKER: u8 = 3;

pub fn name(kind: u8, tracing: bool) -> Option<&'static str> {
    match kind {
        FRAGMENT => return Some("Fragment"),
        PROFILER => return Some("Profiler"),
        TRACING_MARKER if tracing => return Some("TracingMarker"),
        _ => {}
    }
    None
}
```

```js
export const FRAGMENT = 1;
export const PROFILER = 2;
export const TRACING_MARKER = 3;

export function name(kind, tracing) {
  switch (kind) {
    case FRAGMENT:
      return "Fragment";
    case PROFILER:
      return "Profiler";
    case TRACING_MARKER:
      if (tracing) {
        return "TracingMarker";
      }
  }
}
```

Where a guard fails, Rust tries the next arm: where that arm names each
constant of the guarded one's, and one of its own, the guarded arm's case
is its guard's `if`, and falls through to the next arm's case, as React
writes `getComponentNameFromFiber`. Each guard runs where Rust runs it, in
order, once its constant matched; an arm that does nothing where its
guard holds is a `break`, but in the last case, a `default`'s too, which
JS falls out of. One `match` may have guards of both kinds, as React's
`getOwnerStackByFiberInDev` does; each case's guard is one `if`, `if
(!owned && !info)`, whose branch has no `break;` where the case's
end leaves anyway
([ADR 0905](../../decisions/0905-a-guarded-arm-falls-to-the-next.md), amended):

```rust
pub const VIEW_TRANSITION: u8 = 1;
pub const INCOMPLETE_CLASS: u8 = 2;
pub const CLASS: u8 = 3;

pub fn name(tag: u8, ty: &str, view_transitions: bool, legacy_off: bool) -> Option<String> {
    match tag {
        VIEW_TRANSITION if view_transitions => return Some("ViewTransition".to_string()),
        VIEW_TRANSITION | INCOMPLETE_CLASS if legacy_off => {}
        VIEW_TRANSITION | INCOMPLETE_CLASS | CLASS => return Some(ty.to_string()),
        _ => {}
    }
    None
}
```

```js
export const VIEW_TRANSITION = 1;
export const INCOMPLETE_CLASS = 2;
export const CLASS = 3;

export function name(tag, ty, viewTransitions, legacyOff) {
  switch (tag) {
    case VIEW_TRANSITION:
      if (viewTransitions) {
        return "ViewTransition";
      }
    // Fallthrough
    case INCOMPLETE_CLASS:
      if (legacyOff) {
        break;
      }
    // Fallthrough
    case CLASS:
      return ty;
  }
}
```

A guarded arm just before a last `_` that does something falls through to
the `default`, which Rust tries next, as React writes `describeFiber`
([ADR 0636](../../decisions/0636-a-guarded-arm-is-a-case.md), amended):

```rust
pub const HOST: u8 = 1;
pub const LAZY: u8 = 2;
pub const VIEW_TRANSITION: u8 = 3;

pub fn frame(tag: u8, view_transitions: bool) -> String {
    match tag {
        HOST => "host".to_string(),
        LAZY => "Lazy".to_string(),
        VIEW_TRANSITION if view_transitions => "ViewTransition".to_string(),
        _ => String::new(),
    }
}
```

```js
export const HOST = 1;
export const LAZY = 2;
export const VIEW_TRANSITION = 3;

export function frame(tag, viewTransitions) {
  switch (tag) {
    case HOST:
      return "host";
    case LAZY:
      return "Lazy";
    case VIEW_TRANSITION:
      if (viewTransitions) {
        return "ViewTransition";
      }
    // Fallthrough
    default:
      return "";
  }
}
```

A `break` in an arm leaves the loop, as in Rust: in a `switch` it would
leave the `switch`, so it names the loop, the program's label or `loop`:

```rust
pub fn count(s: &str) -> u32 {
    let mut n = 0;
    for c in s.chars() {
        match c {
            'a' | 'e' => n += 1,
            '.' => break,
            '#' => continue,
            _ => {}
        }
        n += 10;
    }
    n
}
```

```js
export function count(s) {
  let n = 0;
  loop: for (const c of s) {
    switch (c) {
      case "a":
      case "e":
        n = (n + 1) >>> 0;
        break;
      case ".":
        break loop;
      case "#":
        continue;
    }
    n = (n + 10) >>> 0;
  }
  return n;
}
```

A block the program labeled around a `match` is the `switch`'s label,
camelCase as every name, where an arm's inner `switch` leaves it, which a
plain `break` can't; elsewhere it's the case's `break`, and no label is
left ([ADR 0944](../../decisions/0944-a-label-the-program-wrote-labels-the-switch.md)):

```rust
use js::{SymbolFor, Unknown};

const FRAGMENT: SymbolFor = SymbolFor("react.fragment");
const STRICT_MODE: SymbolFor = SymbolFor("react.strict_mode");
const CONTEXT: SymbolFor = SymbolFor("react.context");
const MEMO: SymbolFor = SymbolFor("react.memo");
const LAZY: SymbolFor = SymbolFor("react.lazy");

pub fn tag_of(r#type: Option<&Unknown>) -> &'static str {
    let tag;
    'get_tag: {
        match SymbolFor::from_value(r#type) {
            Some(FRAGMENT) => return "fragment",
            Some(STRICT_MODE) => tag = "mode",
            _ => {
                if r#type.is_some() {
                    match SymbolFor::from_value(js::get(r#type, "$$typeof")) {
                        Some(CONTEXT) => {
                            tag = "context";
                            break 'get_tag;
                        }
                        Some(MEMO) => {
                            tag = "memo";
                            break 'get_tag;
                        }
                        Some(LAZY) => {
                            tag = "lazy";
                            break 'get_tag;
                        }
                        _ => {}
                    }
                }
                tag = "throw";
            }
        }
    }
    tag
}
```

```js
const FRAGMENT = Symbol.for("react.fragment");
const STRICT_MODE = Symbol.for("react.strict_mode");
const CONTEXT = Symbol.for("react.context");
const MEMO = Symbol.for("react.memo");
const LAZY = Symbol.for("react.lazy");

export function tagOf(type) {
  let tag;
  getTag: switch (type) {
    case FRAGMENT:
      return "fragment";
    case STRICT_MODE:
      tag = "mode";
      break;
    default:
      if (type != null) {
        switch (type.$$typeof) {
          case CONTEXT:
            tag = "context";
            break getTag;
          case MEMO:
            tag = "memo";
            break getTag;
          case LAZY:
            tag = "lazy";
            break getTag;
        }
      }
      tag = "throw";
  }
  return tag;
}
```

Where JS uses string constants or a `kind` field to tell cases apart, Rust
uses an `enum`. An enum without fields is its variant's name as a string;
one with fields is an object with a `TAG`:

```js
const next = { Red: "Green", Yellow: "Red", Green: "Yellow" };

function area(shape) {
  switch (shape.TAG) {
    case "Circle": return Math.PI * shape.radius * shape.radius;
    case "Rect": return shape.width * shape.height;
  }
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Light {
    Red,
    Yellow,
    Green,
}

pub fn next(light: Light) -> Light {
    match light {
        Light::Red => Light::Green,
        Light::Yellow => Light::Red,
        Light::Green => Light::Yellow,
    }
}

pub enum Shape {
    Circle { radius: f64 },
    Rect { width: f64, height: f64 },
}

pub fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
        Shape::Rect { width, height } => width * height,
    }
}

pub fn unit() -> Shape {
    Shape::Rect {
        width: 1.0,
        height: 1.0,
    }
}
```

```js
export function next(light) {
  switch (light) {
    case "Red":
      return "Green";
    case "Yellow":
      return "Red";
    case "Green":
      return "Yellow";
  }
}

export function area(shape) {
  if (shape.TAG === "Circle") {
    return Math.PI * shape.radius * shape.radius;
  }
  return shape.width * shape.height;
}

export function unit() {
  return {
    TAG: "Rect",
    width: 1,
    height: 1,
  };
}
```

A variant with unnamed fields, `Text(String)`, keeps them as `_0`, `_1`:
`{ TAG: "Text", _0: s }` ([ADR 0013](../../decisions/0013-fieldless-enums.md), [ADR 0033](../../decisions/0033-enums-with-fields.md)).

## Loops

`for ... of` is `for ... in`. `while` is `while`. `loop` is
`while (true)`, and its `break` can carry a value out.

```js
for (const name of names) console.log(name);
for (const [i, name] of names.entries()) console.log(`${i}: ${name}`);
```

```rust
pub fn print_all(names: &[String]) {
    for name in names {
        println!("{name}");
    }
}

pub fn numbered(names: &[String]) {
    for (i, name) in names.iter().enumerate() {
        println!("{i}: {name}");
    }
}

pub fn countdown(mut n: u32) {
    while n > 0 {
        println!("{n}");
        n -= 1;
    }
}

pub fn first_square_over(limit: u32) -> String {
    let mut n = 1;
    let square = loop {
        if n * n > limit {
            break n * n;
        }
        n += 1;
    };
    format!("{n} squared is {square}")
}

pub fn find_pair(grid: &[Vec<i32>], target: i32) -> Option<(usize, usize)> {
    for (r, row) in grid.iter().enumerate() {
        for (c, &cell) in row.iter().enumerate() {
            if cell == target {
                return Some((r, c));
            }
        }
    }
    None
}
```

```js
export function printAll(names) {
  for (const name of names) {
    console.log(`${name}`);
  }
}

export function numbered(names) {
  for (const [i, name] of names.entries()) {
    console.log(`${i}: ${name}`);
  }
}

export function countdown(n) {
  while (n > 0) {
    console.log(`${n}`);
    n--;
  }
}

export function firstSquareOver(limit) {
  let n = 1;
  while (true) {
    if (Math.imul(n, n) >>> 0 > limit) {
      const square = Math.imul(n, n) >>> 0;
      return `${n} squared is ${square}`;
    }
    n = (n + 1) >>> 0;
  }
}

export function findPair(grid, target) {
  for (const [r, row] of grid.entries()) {
    for (const [c, cell] of row.entries()) {
      if (cell === target) {
        return [r, c];
      }
    }
  }
}
```

- A counting loop, `for (let i = 0; i < n; i++)`, is `for i in 0..n`; `0..=n`
  includes `n`.
- A loop from the end, `for (let i = v.length - 1; i >= 0; i--)`, is `for
  x in v.iter().rev()`, each item `v[i]`, no copy of the array made; and
  `for i in (0..n).rev()` is `for (let i = n - 1; i >= 0; i--)`
  ([ADR 0904](../../decisions/0904-a-loop-from-the-end-counts-down.md)):

```rust
pub fn last_named(names: &[String]) -> Option<&String> {
    for name in names.iter().rev() {
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}
```

```js
export function lastNamed(names) {
  for (let i = names.length - 1; i >= 0; i--) {
    const name = names[i];
    if (name) {
      return name;
    }
  }
}
```

- A loop of an array's first items, `for x in xs.iter().take(LIMIT)`, is
  of that part of it, `for (const x of xs.slice(0, LIMIT))`, and of those
  past some, `xs.iter().skip(1)`, `xs.slice(1)`; with their indices,
  `enumerate().take(LIMIT)`, `xs.slice(0, LIMIT).entries()`
  ([ADR 0748](../../decisions/0748-a-for-of-a-slices-first-items-is-of-its-part.md)).
  Write the iterator, not a counter: clippy's `needless_range_loop` asks
  for it.
- `break` and `continue` work as in JS, labels too: `'outer: for ...`,
  `break 'outer;`.
- A `break` or `continue` is where the program wrote it: `if checked >
  limit { ..; break; }` is `if (checked > limit) { ..; break; }`, the rest
  of the body after it, not an `else` ([ADR
  0730](../../decisions/0730-a-loops-early-break-is-in-the-if-it-was-written-in.md));
  a `continue` that ends an arm of a chain of `if`s too, `else if (..) {
  ..; continue; }`, with no label where the arms meet ([ADR
  0745](../../decisions/0745-a-continue-in-a-chain-is-where-it-is-written.md));
  and guards of `continue`s first in a loop that `break`s, each its own
  `if`, its comment inside it, as React's hydration diffs write a `for…in`
  ([ADR 0684](../../decisions/0684-a-continue-the-program-wrote-is-where-it-wrote-it.md), amended).
- A `while`'s test is the loop's own: an `if` of a `break` first in its
  body is the `if` the program wrote, not joined to the test, and a
  comment before a `break` or a `continue` is before it, inside its `if`,
  of a `while let`'s too, whose test the loop's joins ([ADR
  0702](../../decisions/0702-a-while-is-its-test-and-its-break-where-written.md),
  [ADR 0684](../../decisions/0684-a-continue-the-program-wrote-is-where-it-wrote-it.md)):

```rust
pub fn sentence(words: &[&str], limit: usize) -> usize {
    let mut i = 0;
    while i < words.len() {
        if words[i].is_empty() || i == limit {
            // A blank or the limit ends it.
            break;
        }
        i += 1;
    }
    i
}

pub fn until_tag(words: &[&str]) -> usize {
    let mut i = 0;
    while let Some(word) = words.get(i) {
        if word.starts_with('#') {
            // A tag ends it.
            break;
        }
        i += 1;
    }
    i
}
```

```js
export function sentence(words, limit) {
  let i = 0;
  while (i < words.length) {
    if (!words[i] || i === limit) {
      // A blank or the limit ends it.
      break;
    }
    i++;
  }
  return i;
}

export function untilTag(words) {
  let i = 0;
  while (true) {
    const word = words[i];
    if (word == null || word.startsWith("#")) {
      // A tag ends it.
      break;
    }
    i++;
  }
  return i;
}
```

- A `while` inside a `loop`, its `return`s where they're written and the
  rest of the turn after it, is the two loops, `while (node.sibling ==
  null) { .. }` in `while (true) { .. }`, with no label round either ([ADR
  0702](../../decisions/0702-a-while-is-its-test-and-its-break-where-written.md)).
  A `continue 'outer` written in the inner `while` is where it's
  written, `if (isScope(node)) { continue siblings; }`, and what's after
  the `while` follows it, `if (!(node.flags & PLACEMENT)) { return ..; }`,
  as React's getHostSibling writes them (ADR 0702, amended).
- A `match` arm that returns one value, `Kind::Object(_) | Kind::Array(_)
  => return ArrayKind::Complex`, which an arm before it goes on to, a pair
  the guard of `Kind::Array(&[first, _]) if ..` refused, is `return
  "Complex";` in each branch that goes on to it, with no label where the
  arms meet ([ADR 0754](../../decisions/0754-a-return-arms-go-on-to-is-in-each.md)).
  A `return;` past a value's drop, which MIR shares, is where each is
  written too, and a `match` of a string's literals, `Kind::String(
  "fulfilled")`, is `if (status === "fulfilled")`, no `typeof` before it
  ([ADR 0755](../../decisions/0755-a-shared-return-is-each-breaks.md)).
- A labeled block, `'desc: { .. break 'desc v; .. }`, is a labeled block
  where its JS keeps one, by its label, `desc: { desc = v; break desc; }`
  ([ADR 0742](../../decisions/0742-a-label-the-program-wrote-is-its-name.md)).

A `while` that steps its own variable last, `child = child.sibling`, `i +=
1`, a `let` right before it and read nowhere after it, is the `for` that
steps it, as React walks a fiber's children
([ADR 0974](../../decisions/0974-a-while-that-steps-its-variable-last-is-a-for.md)).
Not where a `continue` would skip the step, the body sets the variable
before, or the step reads what the body declares:

```rust
use std::rc::Rc;

pub struct Fiber {
    pub duration: f64,
    pub child: Option<Rc<Fiber>>,
    pub sibling: Option<Rc<Fiber>>,
}

pub fn self_time(fiber: &Fiber) -> f64 {
    let mut total = fiber.duration;
    let mut child = fiber.child.clone();
    while let Some(current) = child {
        total -= current.duration;
        child = current.sibling.clone();
    }
    total
}
```

```js
export function selfTime(fiber) {
  let total = fiber.duration;
  for (let child = fiber.child; child != null; child = child.sibling) {
    total -= child.duration;
  }
  return total;
}
```

## `null` and `undefined`: `Option`

Rust has no `null`. A value that may be missing is an `Option<T>`: `Some(v)`
or `None`. In JS, `Some(v)` is `v` and `None` is `undefined`, so an
`Option` reads as the optional value you'd write by hand.

```js
const displayName = (p) => p.nickname ?? "anonymous";
const ageNextYear = (p) => (p.age != null ? p.age + 1 : undefined);
const initial = (p) => p.nickname?.[0];
```

```rust
pub struct Profile {
    pub nickname: Option<String>,
    pub age: Option<u32>,
}

pub fn display_name(p: &Profile) -> &str {
    p.nickname.as_deref().unwrap_or("anonymous")
}

pub fn age_next_year(p: &Profile) -> Option<u32> {
    p.age.map(|a| a + 1)
}

pub fn initial(p: &Profile) -> Option<char> {
    p.nickname.as_ref()?.chars().next()
}

pub fn is_adult(p: &Profile) -> bool {
    if let Some(age) = p.age {
        age >= 18
    } else {
        false
    }
}
```

```js
export function displayName(p) {
  return p.nickname ?? "anonymous";
}

export function ageNextYear(p) {
  return p.age != null ? (p.age + 1) >>> 0 : undefined;
}

export function initial(p) {
  const value = p.nickname;
  if (value == null) {
    return;
  }
  return Array.from(value)[0];
}

export function isAdult(p) {
  return p.age != null ? p.age >= 18 : false;
}
```

- `unwrap_or` is `??`. `map` changes the value if there is one. `?` after
  an `Option` returns `None` from the function early, as `?.` stops a
  chain. What it tries is read once, as Rust reads it: a property, a
  `const` first, by its `let`'s name, `const parent = fiber.parent; if
  (parent == null) { return; }`, or `value` where no `let` names it
  ([ADR 0035](../../decisions/0035-results-and-throwing-js.md), amended).
- `if let Some(age) = p.age` is "if it's there, call it `age`". Its value
  is the conditional you wrote, `?:`, its `else { false }` too; what you
  write with `&&`, or `p.age.is_some_and(|a| a >= 18)`, is `&&`
  ([ADR 0879](../../decisions/0879-an-if-and-a-closures-captures-read-as-written.md)).
- An `Option` is tested by what it holds: by its truth where that's never
  falsy, an object, an array, a function, a class's instance or a symbol,
  `if (!owner)` and `while (owner)`; against `null` where it may be, a
  number, text, a bigint or a `js::Unknown` of any value, `x != null` and
  `x == null`, so `Some(0)`, `Some("")` and `Some(false)` are there
  ([ADR 0298](../../decisions/0298-truthy-options.md), amended).
  `is_some()`, `if let Some(..)`, `while let Some(..)` and `?` all test
  it so. A `let` of `is_some()` alone that only tests read is `const
  failed = error != null;`, a `bool` as its name says; one of more,
  `error.is_some() || !ready`, is read by its parts' truth, `const hide =
  error || !ready;`
  ([ADR 0298](../../decisions/0298-truthy-options.md), amended).
- A let chain after a test of its own, `if debug && let Some(value) =
  info.value.get()`, which clippy asks for over two `if`s, reads the place
  once, as a person writes it: `if (debug) { const value = info.value; if
  (value != null) { .. } }`
  ([ADR 0842](../../decisions/0842-a-let-chain-after-a-test-of-its-own-reads-its-place-once.md)).
- A `let` an `if let` then takes apart, as one that ends a `RefCell`'s
  borrow first, `let types = prev.types.borrow().clone(); if dev && let
  Some(types) = types && ..`, is the place read in the test, where only
  the test reads it and runs no code first: `if (dev && prev.types !=
  null && prev.types !== other)`
  ([ADR 0866](../../decisions/0866-a-let-only-its-if-let-reads-is-read-there.md)).
  So is a chain's binding of a field its test read, `if let Some(fiber) =
  &fiber && let Some(task) = fiber.debug_task.get() { return
  task.run(..); }`, where the body reads it first, before anything runs:
  `if (fiber != null && fiber._debugTask) { return
  fiber._debugTask.run(..); }`. Where code runs before it's read, or it's
  read again, it's `const task = fiber._debugTask;`, the value the test
  read
  ([ADR 0866](../../decisions/0866-a-let-only-its-if-let-reads-is-read-there.md), amended).
  And an `if let`'s binding of a thread-local that's its module's
  variable, `if let Some(last_fiber) = LAST_FIBER.with_borrow(Clone::clone)
  && last_fiber.index == fiber.index`, only its test reads: the variable,
  `if (lastFiber != null && lastFiber.index === fiber.index)`, as the
  test runs no code
  ([ADR 0866](../../decisions/0866-a-let-only-its-if-let-reads-is-read-there.md), amended).
  One named as the field it reads, `if let Some(auto_name) =
  &*instance.auto_name.borrow() { return auto_name.clone(); }`, is the
  field read in place where the `then` gives it first, `if
  (instance.autoName != null) { return instance.autoName; }`; of another
  name, `const name = instance.autoName;` (ADR 0866, amended). And a `let`
  that ends a borrow that the statement after it stores, `let child =
  current.child.borrow().clone(); work_in_progress.child.replace(child);`,
  is the place read there, `workInProgress.child = current.child;`, where
  it's named as the field and nothing runs before it; named otherwise,
  given as an argument or tested, it keeps its name (ADR 0866, amended).
- rustc won't let you use an `Option<u32>` as a `u32`: you can't forget the
  missing case.
- An `Option<bool>` that's `Some(true)`, `flag.is_some_and(|&on| on)`,
  `flag == Some(true)` or `flag.unwrap_or(false)`, is tested by its truth,
  as JS tests a flag that may be missing: `if (warned[key])`. As a value
  it's a `bool`, `!!flag`
  ([ADR 0298](../../decisions/0298-truthy-options.md)).

JS's `||` falls back on any falsy value, `""` and `0` too, where `??` falls
back on `null` and `undefined` alone. In Rust, say which values you keep,
those that are truthy, `filter(|v| js::truthy(Some(*v)))`, then the
fallback, `or_else`, `or` or `unwrap_or`: that's `||`.

```js
const nameOf = (type) => type.displayName || type.name || null;
```

```rust
use js::Unknown;

#[rust_js::nullable]
pub fn name_of(ty: &Unknown) -> Option<&'static Unknown> {
    js::get(ty, "displayName")
        .filter(|v| js::truthy(Some(*v)))
        .or_else(|| js::get(ty, "name"))
        .filter(|v| js::truthy(Some(*v)))
}

pub fn label(text: Option<&str>) -> &str {
    text.filter(|t| !t.is_empty()).unwrap_or("untitled")
}
```

```js
export function nameOf(ty) {
  return ty.displayName || ty.name || null;
}

export function label(text) {
  return text || "untitled";
}
```

- Only a `filter` by the truth of what it holds is `||`: any other test,
  `filter(|n| *n > 0)`, is a conditional, `n != null && n > 0 ? n : 1`.
- Text is falsy only where it's empty, so `!t.is_empty()` is its truth.
  So a `match` of what it keeps is a test of that truth: `match
  NAME.with_borrow(Clone::clone).filter(|n| !n.is_empty()) { Some(n) =>
  format!("<{n}>"), None => .. }` is `name ? `<${name}>` : ..`, of a
  module's variable too where the arm that reads it does nothing
  ([ADR 0635](../../decisions/0635-a-filter-by-truth-then-a-fallback-is-or.md), amended).
- A `let … else` of what it keeps is a test of its truth too: `let
  Some(context_types) = js::get(ty, "contextTypes").filter(|v|
  js::truthy(Some(*v))) else { return .. };` is `const contextTypes =
  ty.contextTypes; if (!contextTypes) { return ..; }`, as React's legacy
  context writes it.
- A chain held in a `let`, its fallback given after, is one `||` too:
  `let r#type = ..filter(..); r#type.unwrap_or_else(|| "Object".into())`
  is `const type = a || b || "Object"; return type;`, where the fallback
  is a literal and the `let` read only there
  ([ADR 0646](../../decisions/0646-a-held-chain-and-its-fallback-is-one-or.md)).
- What `||` keeps by its truth is `&&` of its test: `value.filter(|_|
  ok).and_then(|v| js::get(v, key))` before a fallback is `(ok &&
  value?.[key]) || ..`, as React writes it
  ([ADR 0647](../../decisions/0647-type-name-as-react-writes-it.md)).
- A `let mut` set again where it's falsy is one `||` too: `let mut name =
  error.name(); if name.is_empty() { name = "Error".to_string(); }` is
  `const name = error.name || "Error";`, as React writes it, and set again
  where it's `None`, `is_none()`, `??`
  ([ADR 0651](../../decisions/0651-a-let-then-its-fallback-is-or.md)).
- An `if` of a value or its fallback is `||`: `if s.is_empty() { "Object"
  } else { &s }` is `s || "Object"`
  ([ADR 0744](../../decisions/0744-a-value-or-its-fallback-is-or.md)).

A value JS gives you that may be `null` is an `Option` too:
[JavaScript types](../interop/types.md).

## Equality

`==` in Rust is always a comparison by value, and both sides have the same
type. There's no `===` to remember.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub fn same_name(a: &str, b: &str) -> bool {
    a == b
}

pub fn same_point(a: &Point, b: &Point) -> bool {
    a == b
}

pub fn same_list(a: &[i32], b: &[i32]) -> bool {
    a == b
}

pub fn same_ref(a: &Point, b: &Point) -> bool {
    std::ptr::eq(a, b)
}

pub fn is_first(points: &[Point], p: &Point) -> bool {
    std::ptr::eq(&points[0], p)
}
```

```js
import { $eq, $index } from "@rust-js/runtime";

export function sameName(a, b) {
  return a === b;
}

export function samePoint(a, b) {
  return $eq(a, b);
}

export function sameList(a, b) {
  return $eq(a, b);
}

export function sameRef(a, b) {
  return a === b;
}

export function isFirst(points, p) {
  return $index(points, 0) === p;
}
```

- Numbers, strings and `bool`s compare with `===`.
- A struct compares field by field, an array item by item, where JS's
  `===` would ask whether they're the same object. `#[derive(PartialEq)]`
  is what lets you write `==` for your struct.
- "Is it the same object?" is `std::ptr::eq` ([ADR 0582](../../decisions/0582-ptr-eq-of-an-object-is-its-identity.md)),
  of an item of a list too, whose index is checked as `points[0]` is
  ([ADR 0285](../../decisions/0285-ptr-eq.md), amended).

## `typeof` checks

In your own code you don't check types at run time: rustc knows each one.
Where a value can be one of several kinds, it's an `enum`, and `match`
tells them apart:

```rust
pub enum Value {
    Number(f64),
    Text(String),
    Flag(bool),
}

pub fn describe(v: &Value) -> String {
    match v {
        Value::Number(n) => format!("number {n}"),
        Value::Text(s) => format!("string {s:?}"),
        Value::Flag(b) => format!("boolean {b}"),
    }
}
```

```js
import { $debugStr, $displayF64 } from "@rust-js/runtime";

export function describe(v) {
  if (v.TAG === "Number") {
    return `number ${$displayF64(v._0)}`;
  }
  if (v.TAG === "Text") {
    return `string ${$debugStr(v._0)}`;
  }
  return `boolean ${v._0}`;
}
```

`$displayF64` prints an `f64` as Rust does, which differs from JS's
`String(n)` for very large and very small numbers.

A value from JavaScript whose type you don't know, what an `any` API
returns, is a `js::Unknown`, or `None` where it's `undefined` or `null`,
and `js::classify` is `typeof`, of those too:

```rust
use js::{Kind, Unknown, classify};

pub fn describe(value: Option<&Unknown>) -> String {
    match classify(value) {
        Kind::Undefined => "nothing".to_string(),
        Kind::String(s) => format!("text {s}"),
        Kind::Number(n) => format!("number {n}"),
        Kind::Bool(b) => format!("flag {b}"),
        Kind::Array(items) => format!("{} items", items.len()),
        _ => "something else".to_string(),
    }
}
```

```js
import { $displayF64 } from "@rust-js/runtime";

export function describe(value) {
  if (Array.isArray(value)) {
    return `${value.length} items`;
  }
  switch (typeof value) {
    case "undefined":
      return "nothing";
    case "string":
      return `text ${value}`;
    case "number":
      return `number ${$displayF64(value)}`;
    case "boolean":
      return `flag ${value}`;
    default:
      return "something else";
  }
}
```

The `js` crate is in [the crates rust-js ships](../libraries/README.md)
([ADR 0225](../../decisions/0225-unknown-values.md)).

## Modules and imports

Each Rust module is an ES module: `mod money;` in `lib.rs` is
`money.js` beside `lib.js`, and `use` is `import`.

```rust
// lib.rs
mod money;

use money::format_price;

pub fn receipt(item: &str, cents: u32) -> String {
    format!("{item}: {}", format_price(cents))
}
```

```rust
// money.rs
pub fn format_price(cents: u32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}
```

```js
// lib.js
import { formatPrice } from "./money.js";

export function receipt(item, cents) {
  const arg = formatPrice(cents);
  return `${item}: ${arg}`;
}
```

```js
// money.js
export function formatPrice(cents) {
  return `$${(cents / 100) >>> 0}.${String(cents % 100).padStart(2, "0")}`;
}
```

- `pub` is what other modules may use. A private item another module of
  yours uses is exported from its JS file anyway, as JS has no other way to
  share it.
- `mod util { ... }` written inline gets a file too, `util.js`.
- The runtime helpers, the `$` names, come from `@rust-js/runtime`.

How files are laid out is in [Emitting modules](../project/modules.md).
Importing a JavaScript or npm module is
[Write bindings for JavaScript APIs](../interop/bindings.md).
