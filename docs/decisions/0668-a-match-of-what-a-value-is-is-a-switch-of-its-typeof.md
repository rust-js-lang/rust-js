# 0668. A `match` of what a value is is a `switch` of its `typeof`

Status: Accepted. Amends [0629](0629-a-match-of-constants-is-a-switch.md),
whose `switch` compared a value with constants: it compares a value's
`typeof` with its variants' too, and finds an arm's test wherever MIR
tests it. Amends [0225](0225-unknown-values.md): `js::Kind` has a
`Symbol`, and its `Object` is an object no array is. Amends
[0214](0214-untagged-enums.md): an `otherwise` variant that's one kind
`typeof` tells is tested as that kind. Amended by
[0676](0676-kind-tells-undefined-and-null.md): `Kind::Undefined` is `case
"undefined"`, and `Kind::Null` is in `case "object"`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, and one rare and
harmless. A `switch` compares `typeof value` with each `case` by `===`,
as each arm's test did, and its cases test kinds no two of which a value
is, so their order isn't observed. `Kind::Object`'s `typeof value ===
"object" && !Array.isArray(value)` is what none of the others' tests was,
of a value neither `undefined` nor `null`, which an `Unknown` never is.
Rare and harmless: an arm of `Kind::Array` is tested first, before the
`switch`, so a revoked `Proxy` of a function, which `Array.isArray`
throws a `TypeError` of, throws it where it was a `Function`, under
Consequences.

## Context

React writes what a value is as a `switch` of its `typeof`. Its
`getIODescription` (shared/ReactIODescription.js):

```js
try {
  switch (typeof value) {
    case 'function':
      return value.name || '';
    case 'object':
      ..
    case 'string':
      ..
    case 'number':
    case 'bigint':
      return String(value);
    default:
      return '';
  }
} catch (x) {
  return '';
}
```

A port matches `js::classify(value)`, a `js::Kind`, an untagged enum (ADR
0214) whose variants' tests are `typeof`'s, and rust-js wrote it as an `if`
chain in the variants' order, not the arms': `if (typeof value ===
"string") .. if (typeof value === "number" || ..) ..`. `Kind` had no
`Symbol`: a symbol was a `Kind::Object`, its `otherwise`, whose test was
none of the six others', seven tests, and `"" + value` of one threw. The
user decided (2026-10-23): `Kind` has a `Symbol`, and a `match` of
`js::classify(value)` whose arms test `typeof` kinds is `switch (typeof
value)`, an or-pattern's cases stacked, `_` its `default`.

## Decision

**A `match` of an untagged enum (ADR 0214) whose arms are variants of
kinds `typeof` tells, with no field but their value, bound or not, and a
last `_`, of three cases or more, is a `switch (typeof value)`.** Each
variant is its `typeof`'s case, in the order the arms and their patterns
name them; an or-pattern's are stacked; `_` is the `default`. It's ADR
0629's `switch`, decided by `hints::written_switches` and written by
`tidy/switches.rs`: a variant's `typeof` case is the one answer of
`untagged::untagged_case`, a `hints::Constant::Kind`.

```rust
match js::classify(value) {
    Kind::Function(_) => text_at(value, "name").unwrap_or_default(),
    Kind::Object(_) | Kind::Array(_) => text_at(value, "url").unwrap_or_default(),
    Kind::String(text) => text.to_string(),
    Kind::Number(_) | Kind::BigInt(_) => js::string(Some(value)),
    _ => String::new(),
}
```
```js
switch (typeof value) {
  case "function":
    return textAt(value, "name") ?? "";
  case "object":
    return textAt(value, "url") ?? "";
  case "string":
    return value;
  case "number":
  case "bigint":
    return String(value);
  default:
    return "";
}
```

- **`js::Kind` has a `Symbol(&'a js::Symbol)`**, `typeof value ===
  "symbol"`. A JS type TypeScript names as `typeof` does a primitive,
  `js::Symbol`'s `symbol`, is that kind (`Kind::of_types`).
- **`Kind::Object` is what's left: an object no array is**, `typeof value
  === "object" && !Array.isArray(value)`. It stays `#[rust_js::otherwise]`,
  what the others aren't; of the seven kinds `typeof` tells of a value
  that's neither `undefined` nor `null`, the one no other variant is
  (`untagged::otherwise_kind`). An `otherwise` that's more kinds than one,
  `Json`'s `Object`, `Child`'s `Other`, is still none of the others' tests;
  a literal variant, `"auto"`, is one string, not every one, so leaves
  `"string"` to it.
- **An array is an `if` before the `switch`**: `typeof` says an array is
  an object, and `Array.isArray(value)` tells it, so an arm of `Kind::Array`
  alone is `if (Array.isArray(value)) { .. }` before the `switch`, its
  `else` where the arm goes on; `case "object"` is then `Kind::Object`'s.
  `Kind::Object(_) | Kind::Array(_)` is one `case "object"`.
- **Its arms' tests anywhere**: MIR tests an untagged enum's variants in
  their order, so each `if` of the chain is its arm's, of those left,
  wherever it is; the cases are written as the arms are.

**Not a `switch`, an `if` chain:**

- **Of two cases**, as ADR 0629's: `Kind::Object(_) | Kind::Array(_) => ..,
  _ => ..`, or `String`, `Array` and `_`, which is `"string"` and a
  `default` beside the array's `if`.
- **An arm of `Kind::Object` where arrays are `_`'s**: `case "object"`
  would take an array, and an `if` before the `switch` would be `_`'s
  statements twice.
- **An arm of an array and a kind `typeof` tells**, `String | Array`: it's
  a case and an `if` both.
- **A variant `typeof` doesn't tell**: a class's, `instanceof`, a test
  function's, `isValidElement`, a literal's, or an `otherwise` that's more
  kinds than one.

## Why

- **It's what React writes**, `getIODescription`, and what a JS developer
  writes to branch on a value's type: one `typeof`, a `case` of each.
- **One rule**: a `match` of what a value is reads as ADR 0629's `match`
  of constants does, of the same three cases, the same `default`, the same
  stacked or-patterns; its constants are `typeof`'s words.
- **The array before**: `typeof` doesn't tell one, and an `if` before is
  what JS writes; a `case "object"` that tests `Array.isArray` inside
  can't give a `_` its arrays without writing `_`'s statements twice.
- **`Kind::Symbol`**: TypeScript's `typeof` has seven answers, and `Kind`
  had a variant of six. A symbol isn't an object: `Object.keys` of one
  throws, and `"" + symbol` throws a `TypeError`.

## Alternatives

- **`case "object": if (Array.isArray(value)) { .. } else { .. }`**: reads
  as JS does where `Kind::Object` has an arm, but a `match` that leaves
  arrays or objects to `_` has no `switch`, where the `if` before has one.
- **`Kind::Object` without `otherwise`**: it holds a `&Unknown`, any
  value, which tells no kind; a type of its own, an object's, would take
  `js::get`'s `&Unknown` from each arm that reads one.
- **`Kind::Undefined` and `Kind::Null`**: `classify` takes a `&Unknown`,
  which is never either (ADR 0030); an `Option<&Unknown>`'s `None` is,
  tested before, `value == null`, as React's `case 'object': if (value ===
  null)` is.
- **Ordering MIR's tests as the arms are, wherever there's a `_`**: an
  `Option`'s `matches!(p, None | Some(Bottom))` became `if (p != null && p
  !== "bottom") { return false; } return true;`.

## Consequences

- **A `match` of `js::Kind` without `_` is an error until it names
  `Kind::Symbol`**: rustc's exhaustiveness, as intended. This repository's
  three, in `test/lowering-bindings.test.ts`, `test/lowering-switches.test.ts`
  and the guide's `interop/types.md`, name it.
- **A symbol is no longer a `Kind::Object`**: a port that read a symbol's
  properties through `Kind::Object` matches `Kind::Symbol`.
- **A revoked function `Proxy` throws** where an arm of `Kind::Array`
  comes before the `switch`: `Array.isArray` of one throws a `TypeError`,
  where it was a `Function`. `typeof` of one is `"function"`, and nothing
  else tests it so.
- Four of the guide's pages write a `match` of `js::Kind` as a `switch`;
  the corpus and the snapshots have none.
- An arm of `Kind::Object | Kind::Array` in an `if` chain was
  `Array.isArray(value) || (typeof value === "object" &&
  !Array.isArray(value))`, which says `typeof value === "object"` in more
  words. (Amended below: it's `typeof value === "object"`.)

## What holds it

- `test/lowering-switches.test.ts`, "a match of what a value is is a
  switch of its typeof": `getIODescription` in its `catch_unwind` (ADR
  0660), each shape and each `if` chain, and each run in Node with a
  symbol, a throwing getter and each kind.
- `test/lowering-structs-enums.test.ts`, "an untagged enum's otherwise is
  the kind left, which a literal doesn't take".
- Mutations in `scripts/mutations/lower/untagged.ts`,
  `scripts/mutations/lower/mir/hints.ts` and
  `scripts/mutations/lower/mir/tidy/switches.ts`.

## Amendment: an arm that returns early is still its case

React's `getIODescription`, ported whole, has a string arm that tests its
length first, `if text.encode_utf16().count() < 5 || .. { return
String::new(); }`, and an object arm of a dozen statements, each an early
`return`. It was no `switch`, but

```js
if (typeof value !== "string") {
  if (typeof value === "number" || typeof value === "bigint") { .. }
  if (typeof value === "function") { .. }
  ..
  return "";
}
if (..) { return ""; }
return value;
```

Not the arms' bodies, the `let .. else` before the `match`, nor their
order: MIR tests `Kind::String` first, its variants' order, and tidy's
`written first`, which puts first an arm MIR made the fall-through though
it's written before the one tested (`if let Text(..) = k { b } else { a
}`), took the `Function`, `Object` and `Number` arms, all written before
the string's, for that arm: it negated the string's test and put them in
it. A `switch` reads an untagged enum's tests as MIR makes them, never
negated, so it found no chain.

**An `if` of an arm of a `match` that's a `switch` stays as MIR tests
it**: `written first` asks `switches::switched`, the one answer to whether
an `if` tests a switch's arm, as `branch values` does, and leaves it. The
`switch` writes its cases as the arms are written anyway, so the order
`written first` was for is kept. A `match` that isn't a `switch`, of two
cases, is put in its written order as before.

- Case: N, as the Decision's: the same tests in another order, of kinds no
  two of which a value is.
- It holds of every `switch`, ADR 0629's of constants too: the corpus's
  `flatten_from_its_back.rs`, `match self.n { 1 => Some(vec![1]), 2 =>
  None, 3 => .., _ => None }`, was `if (flaky.n !== 1) { .. } return [1];`,
  `1`'s arm put last, and is `switch (flaky.n) { case 1: return [1]; ..
  }`. The snapshots and the guide don't change.
- Held by `test/lowering-switches.test.ts`, "an arm that returns early is
  still its case of a switch of a typeof", run in Node, and "React's
  getIODescription is upstream's switch of a typeof", the port whole, run
  in Node beside upstream's answers; and the mutation
  `written-first-of-a-switch`.

## Amendment: an object or an array is `typeof value === "object"`

The port's `getIODescription` tests a property that's an object, as
upstream's `typeof value.request === "object" && value.request !==
null` does, with `if let Some(request) = js::get(value, "request") &&
let Kind::Object(_) | Kind::Array(_) = js::classify(request)`. It read

```js
request != null &&
  (Array.isArray(request) || (typeof request === "object" && !Array.isArray(request)))
```

each variant's test, `||`: MIR sends both to one block, and its lowering
joined their tests, where each holds of its own values only. Together they
are an object, any.

**The test that a value is one of an or-pattern's variants is what they
are together**, as their owner, `untagged.rs`'s `union_parts`, tells: of
each variant, what its test leaves out but what another of them is whole,
which the two take in together; and an array beside an object, any, none,
as `typeof` says an array is an object. So `Kind::Object(_) |
Kind::Array(_)` is `typeof request === "object"`:

```js
request != null && typeof request === "object"
```

- **One owner, of a test and of a case.** MIR's lowering asks
  `variants_test` of the variants it sends to one block, which asks
  `untagged_test`, of each of them; a `switch`'s hints ask
  `untagged_cases` of an or-pattern's, which reads the same parts. So `case
  "object":` of `Object | Array` is the test it reads, one `typeof`, where
  it was two tests, the array's an `if` it found to leave out.
- **What's left out stays where another of them isn't it**: `Object |
  String` is `typeof value === "string" || (typeof value === "object" &&
  !Array.isArray(value))`, and `Array | String` keeps `Array.isArray`.
- Case: N. `Array.isArray(v)` holds of a value only where `typeof v ===
  "object"` does, so the two tests hold of the same values.
- The corpus and the snapshots don't change; the guide's `interop/types.md`
  says it.
- Held by `test/lowering-switches.test.ts`, "a test of an object or an
  array is typeof object", run in Node, with "a match of what a value is is
  a switch of its typeof"; and the mutations `union-keeps-whole-exclusions`,
  `union-keeps-arrays`, `or-pattern-tests-each` and `or-pattern-cases-each`.
