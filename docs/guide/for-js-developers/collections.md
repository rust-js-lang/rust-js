# Collections: arrays, maps, sets, strings

What you write in Rust for JavaScript's arrays, `Map`s, `Set`s, strings and
JSON, and the JavaScript rust-js writes for it. Read
[Basics](basics.md) first. Each section goes: the JS you'd write, the Rust,
then rust-js's JS.

| Rust | JS |
|---|---|
| `Vec<T>`, `&[T]`, `[T; N]`, `VecDeque<T>` | array |
| `HashMap<K, V>`, `BTreeMap<K, V>` | `Map` |
| `HashSet<T>`, `BTreeSet<T>` | `Set` |
| `String`, `&str` | string |
| `(A, B)` | array, `[a, b]` |

## Contents

- [Arrays: `Vec` and slices](#arrays-vec-and-slices)
- [Array methods](#array-methods)
- [Changing an array](#changing-an-array)
- [Sorting](#sorting)
- [Iterators](#iterators)
- [`Map` and `Set`](#map-and-set)
- [Objects as dictionaries](#objects-as-dictionaries)
- [Strings](#strings)
- [JSON](#json)

## Arrays: `Vec` and slices

A JS array is a `Vec<T>` when you own it and grow it, and a slice, `&[T]`,
when a function only reads it. Both are a JS array. Take `&[T]` in a
parameter: a caller can pass a `Vec`, an array or part of one.

```js
const primes = () => [2, 3, 5, 7];
const zeros = (n) => new Array(n).fill(0);
```

```rust
pub fn primes() -> Vec<u32> {
    vec![2, 3, 5, 7]
}

pub fn zeros(n: usize) -> Vec<f64> {
    vec![0.0; n]
}
```

```js
import { $roomFor } from "@rust-js/runtime";

export function primes() {
  return [2, 3, 5, 7];
}

export function zeros(n) {
  $roomFor(n, 8);
  return new Array(n).fill(0);
}
```

Every item has the same type. An array of mixed values is a `Vec` of an
`enum` ([Basics: `match`](basics.md#switch-and-match)).

An index out of bounds panics, as Rust's does, `$index(keys, 0)`. One the
code before it shows in bounds is JS's own, `keys[0]`: below a length
tested, `i < xs.len()`, past a test of the length, `!xs.is_empty()` or
`n == 1` of an `n` that's `xs.len()`, after a `push`, or after an
index of it read, `xs[i]` then `xs[j]` of a `j <= i`. A `Vec` a
`RefCell` holds, `i < queue.borrow().len()`, is known so until anything
may change it: a call but its reads, or a drop of what runs code. Where
`v.get(i)` is `Some`, `while let Some(x) = v.get(i)`'s, a thread-local's
`with_borrow(|v| v.get(i))` too, `i` is below a length, so its `i += 1`
is `i++`, no wrap
([ADR 0292](../../decisions/0292-known-bounds.md),
[ADR 0746](../../decisions/0746-a-length-held-in-a-variable-shows-the-slice-so-long.md),
[ADR 0881](../../decisions/0881-a-refcells-vec-is-indexed-by-its-tests.md)):

```rust
pub fn only_children(keys: &[String]) -> bool {
    let length = keys.len();
    length == 1 && keys[0] == "children"
}

pub fn first(keys: &[String]) -> String {
    keys[0].clone()
}
```

```js
import { $index } from "@rust-js/runtime";

export function onlyChildren(keys) {
  const length = keys.length;
  return length === 1 && keys[0] === "children";
}

export function first(keys) {
  return $index(keys, 0);
}
```

## Array methods

JS's array methods are an iterator chain in Rust: `iter()`, then the
methods, then `collect()` for a new `Vec`. rust-js writes the array methods
you'd have written.

```js
const names = (items) => items.map((item) => item.name);
const available = (items) => items.filter((item) => item.inStock);
const total = (items) => items.reduce((sum, item) => sum + item.price, 0);
const findNamed = (items, name) => items.find((item) => item.name === name);
```

```rust
pub struct Item {
    pub name: String,
    pub price: f64,
    pub in_stock: bool,
}

pub fn names(items: &[Item]) -> Vec<String> {
    items.iter().map(|item| item.name.clone()).collect()
}

pub fn available(items: &[Item]) -> Vec<&Item> {
    items.iter().filter(|item| item.in_stock).collect()
}

pub fn total(items: &[Item]) -> f64 {
    items.iter().fold(0.0, |sum, item| sum + item.price)
}

pub fn find_named<'a>(items: &'a [Item], name: &str) -> Option<&'a Item> {
    items.iter().find(|item| item.name == name)
}

pub fn position(items: &[Item], name: &str) -> Option<usize> {
    items.iter().position(|item| item.name == name)
}

pub fn any_cheap(items: &[Item]) -> bool {
    items.iter().any(|item| item.price < 1.0)
}

pub fn all_stocked(items: &[Item]) -> bool {
    items.iter().all(|item| item.in_stock)
}

pub fn has_number(xs: &[i32], x: i32) -> bool {
    xs.contains(&x)
}

pub fn has_float(xs: &[f64], x: f64) -> bool {
    xs.contains(&x)
}
```

```js
import { $position } from "@rust-js/runtime";

export function names(items) {
  return items.map((item) => item.name);
}

export function available(items) {
  return items.filter((item) => item.inStock);
}

export function total(items) {
  return items.reduce((sum, item) => sum + item.price, 0);
}

export function findNamed(items, name) {
  return items.find((item) => item.name === name);
}

export function position(items, name) {
  return $position(items, (item) => item.name === name);
}

export function anyCheap(items) {
  return items.some((item) => item.price < 1);
}

export function allStocked(items) {
  return items.every((item) => item.inStock);
}

export function hasNumber(xs, x) {
  return xs.includes(x);
}

export function hasFloat(xs, x) {
  return xs.indexOf(x) >= 0;
}
```

| JS | Rust |
|---|---|
| `map` | `iter().map(..).collect()` |
| `filter` | `iter().filter(..).collect()` |
| `reduce` | `iter().fold(init, ..)`; a total is `sum()` |
| `find` | `iter().find(..)`, an `Option` |
| `findIndex` | `iter().position(..)`, an `Option`, never `-1` |
| `some`, `every` | `iter().any(..)`, `iter().all(..)` |
| `includes` | `contains(&x)`; of floats, which `==` finds no `NaN` of, `indexOf(x) >= 0` ([ADR 0966](../../decisions/0966-a-floats-order-is-its-comparison.md)) |
| `flat` | `iter().flatten()` |
| `slice(a, b)` | `&v[a..b]` |
| `forEach` | `for x in &v` |
| `entries()` | `iter().enumerate()` |
| `join(", ")` | `join(", ")`, of strings |

- `item.name.clone()` copies a `String` in Rust; a JS string can't change,
  so in JS it's the string itself.
- `iter()` gives references to the items, `&Item`; `into_iter()` takes the
  `Vec` and gives the items themselves.
- `find` and `position` give an `Option`, so there's no `undefined` or `-1`
  to forget to check.

## Changing an array

```js
const v = [1, 2, 3];
v.push(4);
const last = v.pop();
v.unshift(0);
```

```rust
pub fn stack() -> Vec<i32> {
    let mut v = vec![1, 2, 3];
    v.push(4);
    let last = v.pop();
    println!("{last:?}");
    v.insert(0, 0);
    v.remove(1);
    v.extend([7, 8]);
    v.truncate(4);
    v
}

pub fn splice(v: &mut Vec<i32>) -> Vec<i32> {
    v.splice(1..3, [10, 20, 30]).collect()
}

pub fn drain_front(v: &mut Vec<i32>) -> Vec<i32> {
    v.drain(..2).collect()
}

pub fn concat(a: &[i32], b: &[i32]) -> Vec<i32> {
    [a, b].concat()
}

pub fn spread(a: &[i32], extra: i32) -> Vec<i32> {
    let mut v = a.to_vec();
    v.push(extra);
    v
}

pub fn middle(v: &[i32]) -> &[i32] {
    &v[1..v.len() - 1]
}

pub fn head(v: &[i32]) -> &[i32] {
    let truncated = v.len() > 3;
    if truncated { &v[..3] } else { v }
}

pub fn get(v: &[i32], i: usize) -> Option<i32> {
    v.get(i).copied()
}

pub fn at(v: &[i32], i: usize) -> i32 {
    v[i]
}
```

```js
import { $drain, $index, $removeAt, $slice, $splice, $truncate } from "@rust-js/runtime";

export function stack() {
  const v = [1, 2, 3];
  v.push(4);
  const last = v.pop();
  console.log(`${last == null ? "None" : `Some(${last})`}`);
  v.unshift(0);
  $removeAt(v, 1);
  v.push(7, 8);
  $truncate(v, 4);
  return v;
}

export function splice(v) {
  return $splice(v, 1, 3, [10, 20, 30]).slice();
}

export function drainFront(v) {
  return $drain(v, 0, 2);
}

export function concat(a, b) {
  return [...a, ...b];
}

export function spread(a, extra) {
  const v = a.slice();
  v.push(extra);
  return v;
}

export function middle(v) {
  return $slice(v, 1, (v.length - 1) >>> 0);
}

export function head(v) {
  const truncated = v.length > 3;
  return truncated ? v.slice(0, 3) : v;
}

export function get(v, i) {
  return v[i];
}

export function at(v, i) {
  return $index(v, i);
}
```

- A `Vec` you change is `let mut`, or a `&mut Vec<T>` parameter: rustc
  won't let anything else change it.
- `pop()` is an `Option`: `None` of an empty `Vec`.
- `v.extend([a, b])`, an array written in place, is `v.push(a, b)`; of
  anything else it's the runtime's `$extend`, which pushes each as it's
  stepped ([ADR 0734](../../decisions/0734-extend-of-an-array-written-in-place-is-push.md)).
- **`v[i]` panics past the end,** with Rust's message, where JS gives
  `undefined`; that's the `$index`. `v.get(i)` is the `Option`, and is
  `v[i]` in JS. `remove`, `insert`, `truncate`, `splice` and slicing check
  their bounds the same way, so they're helpers.
- Spread, `[...a, x]`, is `to_vec()` then `push`; `[...a, ...b]` is
  `[a, b].concat()`.
- `&v[a..b]` is a copy of that part, as `slice(a, b)` is: nothing can
  change `v` while the slice is borrowed, so no one can tell.
- **A range the code before it shows in bounds is JS's own `slice`,**
  `v.slice(0, 3)`: one that ends at most at a length tested, `v.len() >
  3`, or held in a `bool`, `let truncated = v.len() > 3;`, while nothing
  the test reads has changed. Any other is `$slice`, which panics as
  Rust's does ([ADR 0806](../../decisions/0806-a-slices-range-in-bounds-is-js-slice.md)).
  A `drain` of one in bounds is JS's `splice`, `queue.drain(0..i)`
  nothing steps `queue.splice(0, i)`; any other `$drain`
  ([ADR 0881](../../decisions/0881-a-refcells-vec-is-indexed-by-its-tests.md)).

## Sorting

`sort()` sorts in place, as JS's does, but numbers sort as numbers and
strings by code point. To sort by something else, `sort_by_key` or
`sort_by`, whose closure returns an `Ordering`.

```js
v.sort((a, b) => a - b);
players.sort((a, b) => b.score - a.score);
players.sort((a, b) => a.name.localeCompare(b.name));
```

```rust
pub struct Player {
    pub name: String,
    pub score: u32,
}

pub fn sort_numbers(v: &mut [i32]) {
    v.sort();
}

pub fn sort_floats(v: &mut [f64]) {
    v.sort_by(|a, b| a.total_cmp(b));
}

pub fn by_score(players: &mut [Player]) {
    players.sort_by_key(|p| std::cmp::Reverse(p.score));
}

pub fn by_name(players: &mut [Player]) {
    players.sort_by(|a, b| a.name.cmp(&b.name));
}

pub fn top(players: &[Player]) -> Option<&Player> {
    players.iter().max_by_key(|p| p.score)
}

pub fn dedup(v: &mut Vec<i32>) {
    v.sort_unstable();
    v.dedup();
}
```

```js
import { $cmp, $dedup, $maxByKey, $totalCmp } from "@rust-js/runtime";

export function sortNumbers(v) {
  v.sort((a, b) => a - b);
}

export function sortFloats(v) {
  v.sort((a, b) => $totalCmp(a, b));
}

export function byScore(players) {
  const key = (p) => [p.score];
  players.sort((a, b) => {
    const left = key(a);
    const right = key(b);
    return $cmp(right[0], left[0]);
  });
}

export function byName(players) {
  players.sort((a, b) => $cmp(a.name, b.name));
}

export function top(players) {
  return $maxByKey(players, (p) => p.score, $cmp);
}

export function dedup(v) {
  v.sort((a, b) => a - b);
  $dedup(v);
}
```

- An `f64` has no `sort()`: NaN has no place. `total_cmp` gives it one.
- `std::cmp::Reverse` sorts high to low.
- A sort is stable, as JS's is.

JS's own `sort()` orders strings by their UTF-16 units, which differs
from Rust's code points where two strings first differ at a character past
U+FFFF and one from U+E000 to U+FFFF. Rust says that order by comparing
their `encode_utf16()`s, which is JS's `sort()` itself, its unstable sort
too, whose ties are the same string
([ADR 0945](../../decisions/0945-utf16-order-is-js-sort.md)):

```rust
pub fn sorted(mut names: Vec<&str>) -> String {
    names.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    names.join(", ")
}
```

```js
export function sorted(names) {
  names.sort();
  return names.join(", ");
}
```

## Iterators

A Rust iterator is lazy: nothing runs until something takes its items.
rust-js writes an array's methods where no one can tell the difference,
which is most chains:

```rust
pub fn squares_of_evens(xs: &[i32]) -> Vec<i32> {
    xs.iter().filter(|&&x| x % 2 == 0).map(|x| x * x).collect()
}

pub fn indexed(words: &[&str]) -> Vec<String> {
    words
        .iter()
        .enumerate()
        .map(|(i, w)| format!("{i}. {w}"))
        .collect()
}

pub fn flat(groups: &[Vec<i32>]) -> Vec<i32> {
    groups.iter().flatten().copied().collect()
}

pub fn first_three(xs: &[i32]) -> Vec<i32> {
    xs.iter().take(3).copied().collect()
}
```

```js
export function squaresOfEvens(xs) {
  return xs.filter((x) => x % 2 === 0).map((x) => Math.imul(x, x));
}

export function indexed(words) {
  return words.map((w, i) => `${i}. ${w}`);
}

export function flat(groups) {
  return groups.flat();
}

export function firstThree(xs) {
  return xs.slice(0, 3);
}
```

`enumerate()` then `map`, `for_each`, `any`, `all`, `flat_map` or `fold`
is that array method, its callback given each item's index as JS gives
it: `fold(0, |sum, (i, x)| ..)` is `reduce((sum, x, i) => .., 0)`, and
`map(|(i, _)| ..)` is `map((_, i) => ..)`.

Where a closure prints, changes something or may panic, and running it on
every item would show, the chain is a lazy JS iterator instead, run one
item at a time, as Rust runs it. Here `find` stops at the third item, so
"checking 4" is never printed:

```rust
pub fn logged_first(xs: &[i32]) -> Option<i32> {
    xs.iter()
        .map(|x| {
            println!("checking {x}");
            x * 10
        })
        .find(|x| *x > 20)
}
```

```js
import { $lent } from "@rust-js/runtime";

export function loggedFirst(xs) {
  return $lent(
    xs.values().map((x) => {
      console.log(`checking ${x}`);
      return (x * 10) | 0;
    }),
  ).find((x) => x > 20);
}
```

A range is an iterator too. In a `for` it's a counting loop; in a chain
it's a lazy range:

```rust
pub fn range_sum(n: u32) -> u32 {
    (1..=n).sum()
}

pub fn evens(n: u32) -> Vec<u32> {
    (0..n).filter(|i| i % 2 == 0).collect()
}

pub fn pairs(a: &[i32], b: &[&str]) -> Vec<String> {
    a.iter().zip(b).map(|(n, s)| format!("{n}{s}")).collect()
}
```

```js
import { $iter, $lazyRange, $lazyZip } from "@rust-js/runtime";

export function rangeSum(n) {
  return $lazyRange(1, n + 1).reduce((a, b) => (a + b) >>> 0, 0);
}

export function evens(n) {
  return $lazyRange(0, n)
    .filter((i) => i % 2 === 0)
    .toArray();
}

export function pairs(a, b) {
  return $lazyZip($iter(a), b)
    .map(([n, s]) => String(n) + s)
    .toArray();
}
```

Your own iterators, the Rust for a generator function, are in
[Advanced: iterators and generators](advanced.md#iterators-and-generators)
([ADR 0036](../../decisions/0036-iterators-and-sorting.md), [ADR 0139](../../decisions/0139-lazy-chains.md)).

## `Map` and `Set`

A `HashMap` is a JS `Map`, a `HashSet` a `Set`.

```js
function wordCounts(text) {
  const counts = new Map();
  for (const word of text.split(/\s+/).filter((w) => w !== "")) {
    counts.set(word, (counts.get(word) ?? 0) + 1);
  }
  return counts;
}
```

```rust
use std::collections::{HashMap, HashSet};

pub fn word_counts(text: &str) -> HashMap<String, u32> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word.to_string()).or_insert(0) += 1;
    }
    counts
}

pub fn lookup(ages: &HashMap<String, u32>, name: &str) -> u32 {
    ages.get(name).copied().unwrap_or(0)
}

pub fn edit(ages: &mut HashMap<String, u32>) {
    ages.insert("ada".to_string(), 36);
    ages.remove("bob");
    if ages.contains_key("cy") {
        println!("cy is here");
    }
    println!("{} people", ages.len());
}

pub fn print_all(ages: &HashMap<String, u32>) {
    for (name, age) in ages {
        println!("{name}: {age}");
    }
}

pub fn unique(words: &[&str]) -> usize {
    let set: HashSet<&str> = words.iter().copied().collect();
    set.len()
}

pub fn seen(tags: &mut HashSet<String>, tag: &str) -> bool {
    !tags.insert(tag.to_string())
}

pub fn common(a: &HashSet<i32>, b: &HashSet<i32>) -> Vec<i32> {
    a.intersection(b).copied().collect()
}
```

```js
import { $add, $orInsert, $setAlgebra } from "@rust-js/runtime";

export function wordCounts(text) {
  const counts = new Map();
  for (const word of text.split(/\p{White_Space}+/u).filter((word) => word !== "")) {
    const current = $orInsert(counts, word, 0);
    counts.set(word, (current + 1) >>> 0);
  }
  return counts;
}

export function lookup(ages, name) {
  return ages.get(name) ?? 0;
}

export function edit(ages) {
  ages.set("ada", 36);
  ages.delete("bob");
  if (ages.has("cy")) {
    console.log("cy is here");
  }
  console.log(`${ages.size} people`);
}

export function printAll(ages) {
  for (const [name, age] of ages) {
    console.log(`${name}: ${age}`);
  }
}

export function unique(words) {
  const set = new Set(words);
  return set.size;
}

export function seen(tags, tag) {
  return !$add(tags, tag);
}

export function common(a, b) {
  return $setAlgebra(a, b, "intersection");
}
```

| JS | Rust |
|---|---|
| `m.get(k)` | `m.get(&k)`, an `Option` |
| `m.set(k, v)` | `m.insert(k, v)` |
| `m.has(k)` | `m.contains_key(&k)` |
| `m.delete(k)` | `m.remove(&k)` |
| `m.size` | `m.len()` |
| `m.set(k, (m.get(k) ?? 0) + 1)` | `*m.entry(k).or_insert(0) += 1` |
| `if (!m.has(k)) { m.set(k, []); } m.get(k).push(v)` | `m.entry(k).or_default().push(v)` ([ADR 0059](../../decisions/0059-hashmap.md), amended) |
| `s.add(x)` | `s.insert(x)`, `true` if it was new |

- A `HashMap` iterates in insertion order, as a JS `Map` does. Native
  Rust's order is unspecified, so don't rely on it.
- **A struct, tuple or enum key compares by value,** as Rust's does, where
  a JS `Map` would compare objects by identity. Derive `Eq` and `Hash`, and
  the map is a `$KeyMap`:

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub row: i32,
    pub col: i32,
}

pub fn board() -> HashMap<Cell, char> {
    let mut b = HashMap::new();
    b.insert(Cell { row: 0, col: 0 }, 'x');
    b
}

pub fn at(b: &HashMap<Cell, char>, row: i32, col: i32) -> Option<char> {
    b.get(&Cell { row, col }).copied()
}
```

```js
import { $KeyMap } from "@rust-js/runtime";

export function board() {
  const b = new $KeyMap();
  b.set({ row: 0, col: 0 }, "x");
  return b;
}

export function at(b, row, col) {
  return b.get({ row, col });
}
```

- **A key equal only to itself is keyed by itself,** as a JS `Set` keys an
  object. Say so in its own `PartialEq` and `Hash`, of its address,
  `ptr::eq` and `ptr::hash`, and a set of references to it is a `Set` of
  the objects, React's set of the fibers a transition updated, and `==`
  of two is `===`. Its `PartialEq` is then written only where generic
  code is given it, `T: PartialEq`. An `Rc` of one is too, a
  `HashSet<Rc<Fiber>>` a `Set` of the fibers; and another crate's, as its
  library's contract says: the reconciler's set of `react`'s fibers
  ([ADR 0864](../../decisions/0864-a-type-equal-only-to-itself-keys-a-set-by-itself.md)).
  A `PartialEq` of `ptr::eq` alone, with no `Hash`, is `===` too, React's
  update, `a != b` of two `Option<Rc<Update>>` `a != b`; only a set or a
  map of it asks for the `Hash` (ADR 0864, amended):

```rust
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

pub struct Fiber {
    pub lanes: u32,
}

impl PartialEq for Fiber {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Fiber {}

impl Hash for Fiber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

pub fn updated(fibers: &[&'static Fiber]) -> usize {
    let mut updated: HashSet<&'static Fiber> = HashSet::new();
    for fiber in fibers {
        updated.insert(fiber);
    }
    let count = updated.len();
    updated.clear();
    count
}

pub fn same(a: &Fiber, b: &Fiber) -> bool {
    a == b
}
```

```js
export function updated(fibers) {
  const updated = new Set();
  for (const fiber of fibers) {
    updated.add(fiber);
  }
  const count = updated.size;
  updated.clear();
  return count;
}

export function same(a, b) {
  return a === b;
}
```

A map you want in key order is a `BTreeMap`, a set a `BTreeSet`. It's a JS
`Map` too, and Rust code iterates it in key order:

```rust
use std::collections::BTreeMap;

pub fn scores() {
    let mut scores = BTreeMap::new();
    scores.insert("b", 2);
    scores.insert("a", 1);
    for (name, score) in &scores {
        println!("{name}: {score}");
    }
}
```

```js
import { $cmp, $sortedEntries } from "@rust-js/runtime";

export function scores() {
  const scores = new Map();
  scores.set("b", 2);
  scores.set("a", 1);
  for (const [name, score] of $sortedEntries(scores, $cmp)) {
    console.log(`${name}: ${score}`);
  }
}
```

A key whose `Ord` you wrote yourself is found by it, as std's B-tree finds
it: the map is an `$ordMap` of that `cmp`, its entries kept in its order,
and a key it finds `Equal` finds the one the map holds:

```rust
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct Release {
    pub major: u32,
    pub name: String,
}

impl PartialEq for Release {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

impl Eq for Release {}

impl PartialOrd for Release {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Release {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major.cmp(&other.major)
    }
}

pub fn named(releases: &BTreeSet<Release>, major: u32) -> Option<&str> {
    let probe = Release { major, name: String::new() };
    releases.get(&probe).map(|release| release.name.as_str())
}

pub fn releases() -> BTreeSet<Release> {
    BTreeSet::new()
}
```

```js
import { $cmp, $ordSet, $storedKey } from "@rust-js/runtime";

export const releasePartialEq = { eq: releasePartialEq_eq };
function releasePartialEq_eq(release, other) {
  return release.major === other.major;
}

export const releasePartialOrd = {
  PartialEq: () => releasePartialEq,
  partialCmp: releasePartialOrd_partialCmp,
};
function releasePartialOrd_partialCmp(release, other) {
  return releaseOrd_cmp(release, other);
}

export const releaseOrd = { PartialOrd: () => releasePartialOrd, cmp: releaseOrd_cmp };
function releaseOrd_cmp(release, other) {
  return $cmp(release.major, other.major);
}

export function named(releases, major) {
  const probe = { major, name: "" };
  return $storedKey(releases, probe)?.name;
}

export function releases() {
  return new ($ordSet(releaseOrd_cmp))();
}
```

([ADR 0059](../../decisions/0059-hashmap.md), [ADR 0121](../../decisions/0121-value-keys.md), [ADR 0404](../../decisions/0404-a-b-tree-keyed-by-value.md).)

## Objects as dictionaries

In JS you might use a plain object, `{}`, as a dictionary. In Rust, your
own dictionary is a `HashMap`: [above](#map-and-set).

A JS object a JavaScript API takes or gives as a dictionary, TypeScript's
`Record<string, T>`, is a `js::Dict<T>`, and it's the object itself:

```rust
use js::{Dict, dict};

pub fn headers(token: &str) -> &'static Dict<String> {
    let headers = dict::new();
    dict::set(headers, "Authorization", format!("Bearer {token}"));
    dict::set(headers, "Accept", "application/json".to_string());
    headers
}

pub fn header(headers: &Dict<String>, name: &str) -> Option<String> {
    dict::get(headers, name).cloned()
}
```

```js
export function headers(token) {
  const headers = {};
  headers.Authorization = `Bearer ${token}`;
  headers.Accept = "application/json";
  return headers;
}

export function header(headers, name) {
  return headers[name];
}
```

`dict::keys` and `dict::entries` are `Object.keys` and `Object.entries`.
More on JS's own values: [JavaScript types](../interop/types.md).

## Strings

A `String` and a `&str` are JS strings, but Rust counts in UTF-8 bytes,
where JS counts UTF-16 code units. `"héllo".len()` is `6`, not `5`;
`s.encode_utf16().count()` counts what JS does, and is `s.length`
([ADR 0632](../../decisions/0632-a-js-strings-length-is-below-u32-max.md)).

```rust
pub fn lengths(s: &str) -> (usize, usize, usize) {
    (s.len(), s.chars().count(), s.encode_utf16().count())
}

pub fn words(line: &str) -> Vec<&str> {
    line.split(',').map(str::trim).collect()
}

pub fn tidy(s: &str) -> String {
    s.trim().replace("  ", " ").to_lowercase()
}

pub fn first_char(s: &str) -> Option<char> {
    s.chars().next()
}

pub fn reversed(s: &str) -> String {
    s.chars().rev().collect()
}

pub fn is_command(s: &str) -> bool {
    s.starts_with('/') && !s.ends_with(' ')
}

pub fn key_value(s: &str) -> Option<(&str, &str)> {
    s.split_once('=')
}

pub fn prefix(s: &str) -> &str {
    &s[..3]
}

pub fn safe_prefix(s: &str) -> Option<&str> {
    s.get(..3)
}

pub fn build(names: &[&str]) -> String {
    let mut out = String::new();
    for name in names {
        out.push_str(name);
        out.push(';');
    }
    out
}

pub fn index_of(s: &str, needle: &str) -> Option<usize> {
    s.find(needle)
}

pub fn first_word(s: &str) -> &str {
    match s.find(' ') {
        Some(at) => &s[..at],
        None => s,
    }
}

pub fn after<'a>(s: &'a str, key: &str) -> &'a str {
    match s.find(key) {
        Some(at) => &s[at + key.len()..],
        None => "",
    }
}

pub fn without_scheme(url: &str) -> &str {
    if let Some(rest) = url.strip_prefix("https://") {
        return rest;
    }
    url
}

pub fn to_number(s: &str) -> Option<f64> {
    s.parse().ok()
}
```

```js
import {
  $byteLen,
  $find,
  $parseF64,
  $splitOnce,
  $strGet,
  $strSlice,
  $trim,
} from "@rust-js/runtime";

export function lengths(s) {
  return [$byteLen(s), Array.from(s).length, s.length];
}

export function words(line) {
  return line.split(",").map((s) => $trim(s));
}

export function tidy(s) {
  return $trim(s).replaceAll("  ", " ").toLowerCase();
}

export function firstChar(s) {
  return Array.from(s)[0];
}

export function reversed(s) {
  return Array.from(s).toReversed().join("");
}

export function isCommand(s) {
  return s.startsWith("/") && !s.endsWith(" ");
}

export function keyValue(s) {
  return $splitOnce(s, "=");
}

export function prefix(s) {
  return $strSlice(s, 0, 3);
}

export function safePrefix(s) {
  return $strGet(s, 0, 3);
}

export function build(names) {
  let out = "";
  for (const name of names) {
    out += name + ";";
  }
  return out;
}

export function indexOf(s, needle) {
  return $find(s, needle);
}

export function firstWord(s) {
  const at = s.indexOf(" ");
  return at !== -1 ? s.slice(0, at) : s;
}

export function after(s, key) {
  const at = s.indexOf(key);
  return at !== -1 ? s.slice(at + key.length) : "";
}

export function withoutScheme(url) {
  if (url.startsWith("https://")) {
    return url.slice(8);
  }
  return url;
}

export function toNumber(s) {
  const result = $parseF64(s);
  return result.TAG === "Ok" ? result._0 : undefined;
}
```

| JS | Rust |
|---|---|
| `s.length` | `s.encode_utf16().count()` in UTF-16 units; `s.len()` in bytes; `s.chars().count()` in characters |
| `[...s]` | `s.chars()`, each a `char`, a one-character string in JS |
| `s.split(",")` | `s.split(',')`, an iterator |
| `s.trim()` | `s.trim()` |
| `s.replaceAll(a, b)` | `s.replace(a, b)`: Rust's replaces every one |
| `s.indexOf(t)` | `s.find(t)`, an `Option`: `s.indexOf(t)` where it's only a place to cut `s` at |
| `s.slice(a, b)` | `&s[a..b]`: `s.slice(a, b)` at places found in `s`; `s.get(a..b)` for an `Option` |
| `s.startsWith(p) ? s.slice(n) : ..` | `s.strip_prefix(p)`, an `Option`; `strip_suffix` of `endsWith` |
| `s += t` | `s.push_str(t)` on a `String`; two in a row, one a value and one text written out, are one, `s += name + ";"` ([ADR 0652](../../decisions/0652-consecutive-appends-are-one.md)) |
| `Number(s)` | `s.parse::<f64>()`, a `Result` |

- **Offsets are bytes,** so `&s[..3]` of `"héllo"` is `"hé"`. A slice
  that cuts a character in two panics, with Rust's message; `s.get(..3)`
  is `None` instead.
- **A place found is JS's own.** An offset `find` gives that's only a
  place to cut the string it was found in, stepped past what was found by
  its length, is `indexOf`'s, `-1` where there's none, and the slice at it
  `slice`: bytes and UTF-16 units cut the same text there, and nothing sees
  the number. One the program sees, returned, printed, compared with a
  number, or used on another string, counts bytes, by `$find` and
  `$strSlice`, as `index_of` above
  ([ADR 0642](../../decisions/0642-a-place-found-in-a-string-is-js-units.md)).
- **JS's own indexes are `js::string`'s:** `string::slice(text, start,
  end)` and `string::index_of` count UTF-16 units, for an index a JS API
  gives or takes, an input's `selectionStart` say, or one from the end,
  `slice(0, -1)`. To search a Rust string and cut it at what's found,
  write std's, which is JS's there.
- `trim` removes Unicode's white space, as Rust's does, which isn't quite
  JS's set: that's `$trim`.
- `&str` is a borrowed string, `String` an owned one. In JS both are just a
  string; `to_string()` and `clone()` cost nothing.

([ADR 0034](../../decisions/0034-strings-and-chars.md), [ADR 0138](../../decisions/0138-string-byte-counts.md), [ADR 0642](../../decisions/0642-a-place-found-in-a-string-is-js-units.md).)

## JSON

JSON goes through serde: derive `Serialize` and `Deserialize` on your
type, then `serde_json::to_string` and `serde_json::from_str`.
`rename_all = "camelCase"` gives JSON keys their JS names.

```js
const toJson = (todo) => JSON.stringify(todo);
const fromJson = (text) => JSON.parse(text);
```

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
    pub id: u32,
    pub title: String,
    pub is_done: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub fn to_json(todo: &Todo) -> String {
    serde_json::to_string(todo).unwrap()
}

pub fn from_json(text: &str) -> Result<Todo, serde_json::Error> {
    serde_json::from_str(text)
}

pub fn title_of(text: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    value["title"].as_str().map(String::from)
}
```

```js
import {
  $debugJsonError,
  $fromJson,
  $json,
  $jsonIndex,
  $jsonValueAs,
  $toJson,
  $unwrapOk,
} from "@rust-js/runtime";

function todoSerialize_serialize(todo, json) {
  json.beginObject();
  json.key("id");
  json.int(todo.id);
  json.key("title");
  json.string(todo.title);
  json.key("isDone");
  json.bool(todo.isDone);
  json.key("tags");
  json.beginArray();
  for (const item of todo.tags) {
    json.element();
    json.string(item);
  }
  json.endArray();
  json.endObject();
}

function todoDeserialize_deserialize(json) {
  return json.struct(
    "struct Todo",
    [
      ["id", $json.u32],
      ["title", $json.string],
      ["isDone", $json.bool],
      ["tags", $json.vec($json.string), () => []],
    ],
    ([id, title, isDone, tags]) => ({ id, title, isDone, tags }),
  );
}

export function toJson(todo) {
  return $unwrapOk($toJson(todo, todoSerialize_serialize, false), undefined, $debugJsonError);
}

export function fromJson(text) {
  return $fromJson(text, todoDeserialize_deserialize);
}

export function titleOf(text) {
  const result = $fromJson(text, $json.value);
  const value = result.TAG === "Ok" ? result._0 : undefined;
  if (value != null) {
    return $jsonValueAs($jsonIndex(value, "title"), "String");
  }
}
```

- `from_str` checks the JSON against the type: `{"id": -1}` is an `Err`
  that says ``invalid value: integer `-1`, expected u32`` at line 1,
  column 8, as serde_json says natively. `JSON.parse` would hand you
  anything.
- `serde_json::Value` is JSON of a shape you don't know.
- Your crate needs `serde` (with its `derive` feature) and `serde_json` as
  dependencies: [Crates from crates.io](../libraries/crates-io.md).

serde's attributes, `rename`, `tag`, `untagged`, `flatten`, `default`,
`skip`, work as serde's do ([ADR 0077](../../decisions/0077-serde-json.md)).
