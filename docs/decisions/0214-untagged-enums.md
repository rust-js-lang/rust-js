# 0214. An untagged enum is its payload: TS's `string | Blob`

Status: Accepted. Extends [0033](0033-enums-with-fields.md): an enum marked
`#[rust_js::untagged]` has no tag. Amended by [0229](0229-union-parameters.md): a
binding's parameter of one is `impl` a sealed trait of its members, of an
ordinary function (ADR 0039), as an `extern` one can't be generic. Amended
by [0668](0668-a-match-of-what-a-value-is-is-a-switch-of-its-typeof.md): an
`otherwise` variant that's one kind `typeof` tells, `js::Kind::Object`'s
`"object"`, is tested as that kind; a `js::Symbol` is a kind, `"symbol"`.
Amended by [0676](0676-kind-tells-undefined-and-null.md): a variant
without fields may be `undefined`, `#[rust_js::undefined]`, or `null`,
`#[rust_js::null]`. Amended: a variant of a `RefCell` of an object is
the object, of an `Rc` what it points at, below. Amended (2026-10-26):
a `js::SymbolFor` payload is a symbol, and a derived `==` of payloads
JS's `===` compares is `===`. Amended (2026-10-28, #391): a JS object's
class is the one its `#[rust_js::types]` names, and a module's type an
object.

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

TypeScript says "a string or a `Blob`" with a union, `string | Blob`, and
needs no wrapper: a JS value knows what it is, `typeof src === "string"`,
`src instanceof Blob`. The web platform is full of them: `fetch`'s
`RequestInfo`, `new Response(body)`'s `BodyInit` of six, `before(Node |
string)`. Rust erases types, so its sum type, an enum, carries a tag, and
rust-js's enums are tagged objects, `{ TAG: "Blob", _0: b }` (ADR 0033),
which no JS API takes. The webapi crate gave a function per member,
`before` and `before_with_str` (ADR 0024).

How the others do it (checked in local clones):

- **ReScript** (`@unboxed`, since 11): a variant whose value is its payload;
  `switch` tests `typeof`, `Array.isArray` or `instanceof` of std's classes,
  `Blob` and `File` among them. At most one case of each runtime kind, or
  the tests couldn't tell them apart (`compiler/ml/ast_untagged_variants.ml`).
- **Scala.js** (`js.|`): a type only the compiler sees, entered by implicit
  evidence, and taken apart by a cast, `isInstanceOf`, which nothing checks
  is exhaustive (`library/.../js/Union.scala`).

A Rust trait can't stand in: an `extern` function, a binding, can't be
generic, and a struct's field, a prop, can't be an `impl Trait`.

## Decision

**An enum marked `#[cfg_attr(rust_js, rust_js::untagged)]` is its payload,
as ReScript's `@unboxed` is:**

```rust
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Src<'a> { Text(&'a str), Bytes(&'a ArrayBuffer) }

impl<'a> From<&'a str> for Src<'a> {
    fn from(s: &'a str) -> Self { Src::Text(s) }
}
```

| Rust | JS |
|---|---|
| `Src::Text("a.png")`, `"a.png".into()` | `"a.png"` |
| `.map(Src::Text)` | `.map((value) => value)` |
| `match s { Src::Text(t) => .., Src::Bytes(b) => .. }` | `if (typeof s === "string") .. else if (s instanceof ArrayBuffer) ..` |
| `load(src)` of `fn load(src: Src)` bound to JS | `load(src)` |

- **Each variant holds one value**, `Text(&'a str)`. A variant of no
  fields, a string literal of TS's `"lazy" | "eager"`, is to come.
- **A variant's test is its payload's runtime kind:**

  | payload | test |
  |---|---|
  | a string, `&str`, `String`, `char` | `typeof v === "string"` |
  | a number of 32 bits or fewer, a float | `typeof v === "number"` |
  | a 64- or 128-bit integer, a BigInt (ADR 0086) | `typeof v === "bigint"` |
  | `bool` | `typeof v === "boolean"` |
  | a `Vec`, an array, a slice, a tuple | `Array.isArray(v)` |
  | a JS object type, `RegExp` (ADR 0111) | `v instanceof RegExp` |
  | one whose `#[rust_js::test]` names what tells it, react's `ReactElement` (ADR 0226) | `isValidElement(v)` |
  | a closure or a function | `typeof v === "function"` |
  | a struct of the crate's | `typeof v === "object"` |

  A JS object type's class is its `rust_js::name`, else its Rust name, as a
  binding's JS name is. Each test is exact, whatever the arms' order: a
  class's leaves out its subclasses in the enum, `File` of `Blob`, which
  `Deref` says, and a struct's every array and class in it.
- **One variant of each kind**, and of each class, as ReScript requires:
  two strings couldn't be told apart. Nor can a payload be what has no
  kind, `Option` (`undefined`), `()`, a generic `T`, another enum: each is
  an error where the enum is declared.
- **The last variant may be what the others aren't**,
  `#[cfg_attr(rust_js, rust_js::otherwise)]`, of a value of any kind, as
  TS's `string | ReactNode` is told by `typeof children === "string"`:
  its test is none of theirs, `typeof value !== "string"`. React's
  `NodeKind`, `Text(&str)` or `Other`, is one, which `react::kind_of(&children)`
  gives a node as, the node itself, as react.dev's Heading labels its link.
  One before another is an error.
- **A `From` into one is the value itself**, at a call, `"a.png".into()`,
  as in generic code (ADR 0108): its `from` must be the variant of its
  argument, `Src::Text(s)`, which is checked where it's declared, so a
  crate that uses another's needs no body to know.
- **`Clone`, `==`, `{:?}` and drops** test the variant as `match` does, and
  read the payload itself where a tagged one reads `_0`.

## Why

- **The value a JS API takes is the one Rust has**: no wrapper made before
  a call, none taken apart after.
- **Exhaustive, unlike TS's narrowing and Scala.js's cast:** `match` must
  name each variant, and rustc checks it.
- **Exact tests** make the arms' order free, as Rust's `match` is for
  disjoint variants.

## Consequences

- A program's own bindings take one where TS takes a union, and so do
  the webapi crate's (ADR 0215).
- A program's own JS class can say what it extends by a `Deref`, as the
  webapi crate's do: its body, a cast of a pointer, is never lowered, as
  a call of it is the object itself.
- Not yet: a variant of no fields, a string literal; a serde
  representation, which serde's own `#[serde(untagged)]` says.

## Since

- **Two variants of one kind may be made**, an enum never told apart:
  Next.js's `getStaticProps` gives `{ props }` or `{ notFound: true }`,
  each its payload, as react.dev's errors page's. A `match` or `if let`
  of one is refused where it's written, and the enum itself where its
  `Clone`, `PartialEq` or `Debug`, or its drop, would tell them apart. A
  compiler test makes one; the diagnostics test matches one and clones
  another; mutations refuse the made one, and let the matched and the
  cloned be.

## Amendment: a variant without fields is its name's string

A variant of no fields (2026-10-10) is its name's string, its
`#[rust_js::name]` or its Rust name, as TypeScript's `boolean | "blocking"`
has a literal beside a type: made as `"blocking"`, matched by
`value === "blocking"`, compared and shown as one, and declared as the
literal in its union. One beside a variant of any string is refused where
the two are told apart, as two of one kind are: JS can't tell the literal
from that string. Next.js's `getStaticPaths` `fallback`, `GetStaticPathsFallback::Blocking`,
and a cookie's `sameSite`, `SameSite::Lax`, are typed as Next.js types
them, where they were a string of any text. A lowering test makes,
matches, compares and shows one, and refuses one beside a string; a
declarations test declares one; mutations match any value, allow the
string, refuse the variant, leave it unmade, compare it equal to any, and
declare it `undefined`.

## Amendment: a variant of a fieldless enum

A fieldless enum is its variant's name (ADR 0013), so a variant holding
one, `One(Weight)` beside `Many(&[Weight])`, is a string, told by
`typeof` as any string is, as next/font's `"400" | Array<"400">` needs.

## Amendment: a plain object's type is an object

A payload of a JS object type is told apart by its class, `instanceof`;
but one TypeScript types as an object literal, `Dict`'s `{ [key: string]:
T }`, has no class: it's an object, `typeof value === "object"`. A legacy
URL's query, its text or a `ParsedUrlQuery`, was tested `instanceof Dict`.

## Amendment: a `&mut` to one is a place (#347)

An enum each of whose variants has fields was an object, a `&mut` to it
the object, `*r = v` writing `v` into it in place (ADRs 0033, 0147). An
untagged enum's value is its payload, which needn't be an object, nor one
`v` can be made into: trekhleb/javascript-algorithms' Matrix, `Cell(f64)`
or `Rows(Vec<Matrix>)`, wrote a cell through `*cell = Matrix::Cell(v)`,
which was `$assign(0, v)`: nothing changed, a wrong answer; and an array
made a plain object, `$assign([..], { .. })`, was a RangeError. A class's
instance would keep its prototype, so `instanceof` told the old variant.
**A `&mut` to an untagged enum is a place**, a box or a handle, as a
`&mut f64`'s, `*r = v` writing the place: `values[$at(values, 8)] = []`.
ADR 0262's case: results change, so Rust's. Held by
`test/corpus/untagged_written_through_mut.rs` and the mutation
`untagged-written-in-place`.

## Amendment: a `RefCell` of an object is the object, an `Rc` what it points at

React's `ThenableState` is an array in production and `{
didWarnAboutUncachedPromise, thenables }` in DEV, its port an untagged
`enum State { Prod(RefCell<Vec<T>>), #[otherwise] Dev(StateDev) }`, which
was refused: "JS can't tell a `RefCell<Vec<f64>>` from another variant's
value".

**A payload's `RefCell` of what holds itself, an array, a map, a set, a
struct's object, is that object, and an `Rc` what it points at**, as each
is in JS (ADRs 0844, 0863): `Prod(RefCell<Vec<f64>>)` and
`Many(Rc<RefCell<Vec<f64>>>)` are arrays, told by `Array.isArray`:

```js
function items(state) {
  return !Array.isArray(state) ? state.items : state;
}
```

- **Where the crate has one as a box it's refused**: a `RefCell` its
  generic code has of any `T`, `{ value }`, or an `Rc` it counts, `{
  value, strong, weak }`, which no test of its kind tells. Each is known
  only once the crate's bodies are seen, so it's refused then, by the
  owners of each, `drops/ref_cells.rs` and `analysis.rs`'s counts, of
  what `untagged::held_payloads` says each variant holds.

Case D ([0262](0262-when-rust-and-js-disagree.md)): a variant is told
from the value it is; where it can't be, a compile error.

Tested by `test/corpus/untagged_ref_cells.rs` against native Rust, and
the diagnostics test's "variant of a RefCell the crate boxes" and "an Rc
the crate counts"; its mutations are `untagged-ref-cell-refused`,
`untagged-rc-refused`, `untagged-boxed-ref-cell-unrefused` and
`untagged-counted-rc-unrefused`.

## Amendment: a symbol is a symbol, and `==` of primitives is `===` (2026-10-26)

shared/ReactTypes' `ReactKey`, `string | ReactOptimisticKey`, ported as
an untagged enum of `Text(String)` and `Optimistic(SymbolFor)`, showed two
gaps:

- **A wrong answer**: a `js::SymbolFor` payload was told by
  `Array.isArray`, as the tuple struct Rust holds its key in, where JS's
  value is the registry's symbol (ADR 0634). A `match` whose first arm was
  that variant read every symbol as the next variant's. It's a symbol now,
  `typeof value === "symbol"`, which a `switch (typeof value)` tells too.
- **`==` was `$eq(a.key, b.key)`**, a structural comparison, where React
  writes `child.key === key`.

**A derived `==` of an untagged enum whose every payload JS's own
comparison compares as Rust's `==` does is that comparison**
(`std_impls.rs`'s `js_eq`, of `untagged_js_eq`): its value is its
variant's payload, or a unit variant's literal, `null` or `undefined`,
each of a kind no other variant's is, which the enum's declaration's
check holds of one a `==` tells apart, so two values are one variant
where JS finds them alike, and equal where their payloads are. `===`,
SameValueZero of `includes` and a `Map`'s keys too, but of a float's
payload, whose `NaN` only `===` finds unequal. Of an `Option` of one,
`(a ?? null) === (b ?? null)`, as of a value of any shape (ADR 0612,
amended): `==` would make a string's `"7"` of a number's `7`. A nullable
field's `None` is `null` already, so React's is `a.key === b.key`. An
enum of a payload compared otherwise, a `Vec`'s, or of a hand-written
`==`, is compared as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same answer;
the symbol's kind, C, Rust's, where the JS was wrong.

Held by the corpus's `untagged_eq_by_value.rs`, against native Rust,
`test/lowering-structs-enums.test.ts`, "an untagged enum's symbol is told
by typeof, and its == is JS's ===", run as JS, and mutations
`untagged-eq-structural`, `untagged-eq-of-a-float-same-value`,
`untagged-option-eq-loose`, `untagged-eq-of-a-payload-compared-otherwise`
(`std_impls.rs`) and `untagged-symbol-for-an-array` (`untagged.rs`).

## Amendment: a JS object's class is the one TypeScript names (#391, 2026-10-28)

A payload of a JS object type was tested `instanceof` its Rust name, or
its `#[rust_js::name]`, whatever its `#[rust_js::types]` said. Two wrong
answers came of it. A type declared as the guide declares one, a
module's class, `#[rust_js::types = "chart.js#Chart"]`, was tested
`instanceof Chart` of a `Chart` its module never imported, which throws.
And one named apart from its class, `pub struct Url` of `types = "URL"`,
was tested `instanceof Url`, of no class JS has, or another class than
TypeScript's.

**The class is what `#[rust_js::types]` names, where it names one**
(`Recognition::untagged_kind`), as `js::class::<T>()`'s is (ADR 0911):

- a global's, `types = "URL"`: `value instanceof URL`;
- a module's type, `types = "chart.js#Chart"`: an object, `typeof value
  === "object"`, the others' tests left out, as no `instanceof` can name
  a class a module doesn't import; two such in one enum are refused as
  any two objects are, where they're tested;
- `object`, or an object literal's type (above): an object; an array's
  type an array, a function's a function;
- `unknown`, a union, a literal or another keyword: none JS can tell,
  refused as a payload of no kind is, but as the `otherwise` variant.

One TypeScript names nothing is the global class of its name, its
`#[rust_js::name]` where given, as before: js's `RegExp`, `ArrayBuffer`,
`Intl.Collator`. So a binding of a module's class declares its `types`,
as the guide's do.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, Rust's answer,
where the JS threw or tested another class.

Held by `test/compiler.test.ts`, "an untagged enum's JS object is tested
by the class TypeScript names", run as JS, and mutations
`untagged-class-by-rust-name` and `untagged-module-class-unimported`
(`untagged.rs`).
