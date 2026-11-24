# 1194. A value that holds others is checked whole, its path in the error

Status: Accepted (2026-11-24, #2403 step 2b(b), the lead's approval of the
deep checks' shapes: one recursive checker per type per module, paths in
the error, extra keys allowed). Amends
[1192](1192-a-value-js-gives-is-checked-shallow-first.md): what holds
other values is checked too.

Case ([0262](0262-when-rust-and-js-disagree.md)): **C**, as 1192's: a
call JS makes with a value outside its parameter's Rust type throws.

## Context

ADR 1192 checks a parameter that's whole in itself, a number or a string.
A struct, a tuple, an array or a `Vec` holds others, any of which JS can
give wrong: an order whose item's `qty` is `"5"` was summed as text. ADR
1193 keeps Rust's own calls free of checks, so a check of a value's size
is paid by JS's calls alone.

## Decision

**A parameter of a value that holds others is checked whole, each value in
it by its own type's check, as deep as it goes.**

```js
function checkOrder(value) {
  $checkObject(value);
  $checkField(value, "id", $checkU32);
  $checkField(value, "items", (value, where, none) => $checkArray(value, where, none, checkItem));
  $checkField(value, "note", $checkStr, "nullish");
}

export function count(order) {
  $checkWhole(checkOrder, order, "count(order)");
  return count$(order);
}
```

- **A struct's checker is its module's function, `checkOrder`**, made once
  for each module whose checks call it (`boundary/deep.rs`). It's recursive
  where the struct is: a `Node` whose `next` is a `Node` checks it by its
  own checker.
  - Its name is the namer's, of the type's, allotted in each module the
    first time one of its checks asks, apart from every word the module
    takes and its other checkers' (`names::fresh_apart`, ADR 1150): shlex's
    `Shlex` is `checkShlex`, its `bytes::Shlex` beside it `checkShlex$1`, a
    library's `Manifest` beside the crate's own `checkManifest$1`. Each type
    is checked by its own checker, never by another's fields; none goes
    unchecked for a function of its name (`checker_name`, held by mutation
    `checker-names-shared`, #2403).
  - It checks that the value is an object, not `null` and not an array.
  - Then each field by its key, as `Shape::Object` keys it: its name, or
    the symbol that names it (ADR 1051), read where the checker is, its
    path `order(reason[Symbol(order)])`.
  - An `Option` field may be either JS's `None`; any other field may not be
    missing.
  - Extra keys are allowed, as TypeScript's structural types allow them.
- **What else is checked whole:**
  - an array, a `Vec` or a slice, item by item (`$checkArray`), and a
    `[T; N]` of exactly `N`;
  - a tuple, an array of exactly its items, each by its own type
    (`$checkTuple`);
  - a `Box` or an uncounted `Rc` by the value it points at;
  - an enum with fields by its module's checker, `checkShape`. `$checkTag`
    gives its tag, then the fields of the variant it names are checked
    (`variant_fields`' keys, `_0` of a tuple variant's). Of a tagged enum, a
    fieldless variant is its name's string and one with fields an object of
    its `TAG`. Of a tag of its own (`#[rust_js::tag]`, ADR 0284), each is an
    object of that key;
  - a `Result`, `{ TAG: "Ok" | "Err", _0 }`, inline, by its two types;
  - a generic struct or enum by one checker of its type, whatever its
    uses, given its type arguments' checkers as parameters:
    `function checkPair(value, A, B)`, each use
    `(value) => checkPair(value, $checkU32, $checkStr)`. One type's uses
    never multiply its checkers. A type parameter's field is checked by the
    checker it's given, and a recursive one passes it on,
    `checkList(value, T)`. One of a `const` parameter isn't checked yet;
  - a map, a `Map`, and a set, a `Set`, of keys a JS `Map` compares as
    themselves (numbers, strings, `bool`s, `char`s): each key and value
    (`$checkMap`), each item (`$checkSet`). A failure is named as JS reads
    it: `prices.get("cake")`, `prices.keys()`, `tags.values()`;
  - a `&mut`: of a value JS can't change in place, its box, `{ value }`,
    its `value` by the value's check (`$checkCell`); of an object, the
    object itself (ADR 0099). Of a generic box, a guard, or what's given as
    it is: not yet;
  - an `Option` in any of these, either JS's `None` (`$checkOption`);
  - a `#[rust_js::flatten]` field (ADR 0204), whose keys are the struct's
    own: its checker checks the struct's object, its failure the struct's
    path;
  - a props field with a `#[rust_js::default]` (ADR 0212), which a JS
    caller may leave out for the component it's passed to: its
    `undefined` too, as Rust reading one of whole props is refused.
- **The error names where in the value it failed:**
  `TypeError: rust-js: count(order.items[1].qty): expected a u32, got "5"`.
  The path is built only as a failure is thrown, each level adding its key
  or index (`$checkAt`), and `$checkWhole` names the parameter. A value that
  checks builds nothing.
- **Not yet:**
  - a type whose checker's name the crate already has, which a module may
    import;
  - an untagged enum, one of an `otherwise` variant, of `#[repr]` numbers or
    of `bool`s;
  - a map or a set of keys a `$KeySet` keeps apart, a tuple's or a
    struct's, which is a conversion at entry, not a check (#2403's open
    question);
  These are 2b(b)'s next slices.

## Why

- **Rust never sees an impossible value inside one it's given.** A check
  of the top alone would let `qty: "5"` through.
- **One checker per type.** Two functions given an `Order` call one
  `checkOrder`, which the module declares once.
- **No path where nothing failed.** Building `order.items[1].qty` for each
  value checked would cost an allocation per value.

## Alternatives

- **The checks inlined in each function:** a struct used by ten functions
  checked ten times over in the JS, and no recursion for a type that holds
  itself.
- **A description of the type given to one generic checker,
  `$checkShape(value, { id: "u32", .. })`:** an object made at each call,
  read key by key.

## Consequences

- An exported function of such a parameter starts with `$checkWhole(..)`,
  and its module declares the checkers it calls.
- The cost is a value's size, once per JS call, never Rust's (ADR 1193).

## What holds it

- `test/boundary-deep.test.ts`:
  - a struct's checker and its module's one declaration;
  - a value that checks, and an `Option` field's `None`;
  - failures named by their paths;
  - a missing field, and a value of the wrong kind;
  - arrays of arrays;
  - a struct that holds itself;
  - a tuple struct;
  - an enum by its tag and its variant's fields, of `TAG` and of a tag of its
    own, and a `Result`;
  - a generic type's one checker at two uses, and a recursive one;
  - a field a symbol names, by that key and its path;
  - a map's entries and a set's items;
  - a `&mut`'s box and an object it points at.
- The mutations in `scripts/mutations/lower/boundary/deep.ts` and the
  runtime's `boundary_checks.ts`.

## Amendment: a check reads a field by its descriptor; an accessor's is unchecked (#2403, the lead's decision, 2026-11-24)

A deep check that reads `object[key]` runs a getter, a read Rust never makes. `map(r)` given `r.snap`, a
getter that gives an object and then `null`, saw `null` in its body after the check had read the object:
it gave `undefined` where Rust gives `1`. That breaks ADR 1022's "read as often as Rust reads it", which ADR
1087's getter-reads cases hold. Now the runtime's checkers, `$checkField`, `$checkTag` and `$checkCell`,
read a field by its descriptor (`$checkRead`), its own or its prototypes'. An accessor's value is
`$checkAccessor`: unchecked, read once by the body, as before the checks. Only data properties are
checked. This is the limit the user decided of #1908, "accept and document the limit", not a change of
results, which ADR 0262 forbids. A `Proxy` is checked by what its target holds, which is what Immer's drafts
and Vue's reactive objects wrap, and a `get` trap that answers otherwise isn't seen. The getter-reads
harness's changing answers are real accessors now, as JS writes a getter. Held by
`test/getter-reads/js_given_options.rs` and `getter_enum_tag.rs`; the guide's semantics, 66.

## Amendment: a struct's checker is its table, marked pure (#2403 (B), Tu's decision, 2026-11-24)

A struct's checker of its fields alone is its table's constant, `const checkPoint = /* @__PURE__ */
$checkShape(() => ({ x: $checkU32, next: [checkNode, "nullish"] }))`, each field's check once, a `None`'s
with it. `$checkShape` reads the table where it first checks, so a struct that holds itself, or whose fields'
checkers its module makes after it, names them; it gives back the checker function, so every caller is as
before. `/* @__PURE__ */` (`Known::PureCall`, oxc's own annotation) lets a bundler drop the constant where
nothing reads it, as the tooling net's tree-shaking test shows of an unused export's. A generic struct's
checker, given its type arguments' checkers, an enum's and a tuple struct's stay functions. A library's
struct a consumer's function takes is checked by the consumer's own checker, `checkManifest`, named apart
from its own types by the namer (`Contract::Library`, which had none).
