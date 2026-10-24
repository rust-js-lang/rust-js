# 0746. A length held in a variable shows the slice at least so long, and a `push` makes one long

Status: Accepted. Extends [0709](0709-a-signed-counter-below-a-length-stays-below-it.md)'s
ranges, and [0576](0576-what-a-bodys-integers-can-be.md)'s least lengths.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the program can't
observe it. Each index left unchecked is one Rust's check passes, and each
wrap left out one that can't happen.

## Context

React's `addObjectDiffToProperties` and `addValueToProperties`
(shared/ReactPerformanceTrackProperties.js), ported, read an item of a
slice whose length they hold in a variable:

```rust
properties.push((UNCHANGED.to_string() + &key, kind));
let prev_length = properties.len();
let nested_equal = add_object_diff_to_properties(prev_object, next_object, properties, indent + 1);
if !nested_equal {
    is_deeply_equal = false;
} else if prev_length == properties.len() {
    properties[prev_length - 1].1 = "Referentially unequal ..".to_string();
}
// ..
let props_length = props_keys.len();
if indent < 3 || (props_length == 1 && props_keys[0] == "children" && key.is_none()) {
```

rust-js wrote `$index(properties, (prevLength - 1) >>> 0)[1] = ..` and
`$index(propsKeys, 0)`, where upstream reads `properties[prevLength - 1]`
(its `entry`) and `propsKeys[0]`. Each is in bounds, and `prev_length - 1`
can't wrap. The ranges lost both:

- **A `push` of a `Vec` whose length wasn't known left it unknown.** A
  `push` added one to a least length known, and of none, `properties` a
  parameter, knew nothing: so `prev_length`, the length after it, was
  taken as maybe 0, and its `- 1` as one that may wrap.
- **A variable that's a length, tested, showed nothing of the length.**
  `xs.len() == 1` puts `xs` at least one long, but `n == 1` of an `n` that
  is `xs.len()` only narrowed `n`: the least length came of a test of the
  `len()` call itself.

## Decision

**A `push` makes a `Vec` at least one long** (`Known::grown`), what was
known of it or not: of no least length, at least the one item pushed.

**A value at least `n`, and at most a slice's length, shows the slice at
least `n` long** (`Known::at_least`): of an integer `⌊(c·L + b) / 2^s⌋` at
most, by a length `L`, at least `lo`, `L` is at least `⌈(lo·2^s - b) /
c⌉`. It's the one place a least length is learned of a test: of a
variable narrowed (`narrow`), `n == 1` or `n >= 2` of an `n` that's
`xs.len()`, and of the `len()` call tested itself, `xs.len() > k`, as
before.

So React's reads are upstream's:

```js
properties[prevLength - 1][1] = 'Referentially unequal but deeply equal objects. Consider memoization.';
// ..
if (indent >= 3 && (propsLength !== 1 || propsKeys[0] !== 'children' || key != null)) {
```

A length that may be 0 keeps Rust's check: `keys[0]` past `length >= 2`'s
`else` is `$index(keys, 0)`, and panics of none, as Rust does.

## Why

- **It's upstream's JS**: `propsKeys[0]`, `properties[prevLength - 1]`.
- **One rule for each**: a length is learned of what bounds it, a
  variable or the call, by one owner; a `push` grows a length whether its
  least was known or not.

## Alternatives

- **Track which variables are a length, as such**: a second form of what
  a range's bound by the length already says, `n ≤ L`.
- **Write the port's `props_keys.len() == 1`, the call, in place of its
  variable**: Rust bent for the compiler, which AGENTS.md rules out.

## Consequences

- A least length is known of more slices, so a loop's widening, which
  keeps a least length only where each round agrees, may take a round
  more to settle.

## What holds it

- `test/lowering-numbers.test.ts`, "a length held in a variable shows the
  slice at least so long", run in JS, and React's two reads in the port.
- Mutations `push-of-unknown-length-unknown`, `held-length-shows-nothing`
  and `at-least-one-short`.
