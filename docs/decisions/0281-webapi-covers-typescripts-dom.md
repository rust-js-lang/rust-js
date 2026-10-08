# 0281. The webapi crate covers TypeScript's DOM, and a test holds it

Status: Accepted. Extends [0024](0024-web-crate.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

rust-js says it types the web on par with TypeScript and ReScript
([type-foundations.md](../research/type-foundations.md)). The react.dev port
kept finding what the `webapi` crate lacked, `history`,
`URL.createObjectURL`, `HTMLScriptElement.supports`, one at a time. Measured
([web-platform-coverage.md](../research/web-platform-coverage.md)), the
crate had 194 of TypeScript's 701 DOM classes and 13.5% of their members;
ReScript's @rescript/webapi has about twice as many. Nothing measured it,
so nothing said when it fell behind.

## Decision

**The `webapi` crate's goal is every class and member TypeScript's
`lib.dom.d.ts` has, and a test measures how much it has.**

- **The measure** (`webapi/coverage.ts`) reads `@types/web`, the
  `lib.dom.d.ts` TypeScript generates, pinned, with TypeScript's own parser:
  each class, its members, those of its mixins (`ParentNode`'s), its statics
  and its constructor. A member is bound where a function of src/lib.rs
  names it in its MDN link, as the generator writes on each.
- **It's a ratchet**: `webapi/coverage.txt` lists each member bound, and
  `test/webapi-coverage.test.ts` fails when one isn't any more, or when a new
  one isn't blessed (`BLESS=1`), so a change to the coverage is a diff.
- **The rest comes in this order**: every class TypeScript has, from the
  same specs; the WebIDL types the generator skips (`float`, sequences and
  unions in results, `long long`, `record`, constants, static attributes);
  callbacks and `on*` handlers; then the `js` crate's `Date`, `Intl` and
  typed arrays.

## Why

- **A promise needs a number.** "On par" was a claim; a count against the
  same declarations TypeScript users get is checkable, and the ratchet
  keeps it from slipping.
- **TypeScript's DOM is the reference users know**, and it's generated from
  WebIDL as the crate is, so matching it is matching what it reads.

## Alternatives

- **Add what a port needs, as it needs it** (as before): a smaller crate,
  but every port pays for each gap first, and nothing says how far behind
  it is.
- **Measure against WebIDL itself**: no extra package, but it isn't what
  "on par with TypeScript" means; TypeScript leaves out some of it and adds
  to it by hand.

## Consequences

- `@types/web` is a dependency of the generator's package, pinned.
- A binding counts by its link name in its type's module (by its MDN link,
  at first: see Since).
- Generating every class makes src/lib.rs about three times its size.

## Since

- **Every class TypeScript's DOM has is generated**, 699 of its 701 (the
  two WebIDL doesn't define), from whichever spec has it. The specs the
  crate read before give all their members; the others give those
  TypeScript has, not one it leaves out as an experiment. Members went from
  13.5% to 46.6%; no function was renamed or lost. A keyword's `_` is
  dropped before a form's suffix: `continue_with_key`.
- **The WebIDL types it skipped**: a sequence a function gives is a `Vec`,
  a new array each time; a frozen array a slice, `&'static [String]`, the
  same array, which JS won't let change. A `long long` is an `f64`: WebIDL
  makes it a JS number, which an `i64`, a `BigInt` (ADR 0086), isn't, so
  it's exact to 2^53, as JS's is (case C would be a `BigInt` the browser
  rejects). A `float` is an `f32`. A dictionary a function gives has its
  optional fields as `Option`s, renamed ones by their JS names, and its
  parent's; one a function also takes is the parameter's struct, which
  borrows, so not the result's. A constant is a Rust `const`,
  `node::ELEMENT_NODE`, its value written where it's read (ADR 0031).
  Members: 46.6% to 75.0%.
- **An event handler property, `onclick`, is a getter of what it holds, a
  function or none, and a setter of a closure**, given the event its name
  is on the target, as `add_event_listener`'s is (ADR 0223): a button's
  `set_onclick` takes `Option<Box<dyn FnMut(&PointerEvent)>>`, whose `None`
  is `null` (`rust_js::nullable`, ADR 0275). Members: 75.0% to 83.5%.
- **A constructor counts where WebIDL has one JS can call**: not where
  TypeScript declares `new()` and WebIDL has none, `new Node()`, nor an
  `[HTMLConstructor]` element's, which only a custom element's `super()`
  calls (a TypeScript loophole, not parity). The other specs' constructors
  are generated where TypeScript has them, `new WebSocket(url)`.
- **A dictionary a function gives and another takes is one struct**, the
  result's, which owns what it holds, taken by value: `CookieStoreGetOptions`.
  Members: 89.2% of 8,488.
- **Each CSS property of a style is a getter and a setter of its text**,
  `css_style_properties::set_background_color(style, "red")`, from the
  names TypeScript lists, as CSSOM says each property has one and WebIDL
  can't list them; each documented by its CSS page.
- **The measure reads each function's link name** in its type's module, not
  its MDN link, so a binding documented otherwise counts; both gave the
  same numbers. Members: 95.5%.
- **An iterable, a maplike and a setlike give `for_each`**, of what JS gives
  its callback, an array's `(value, index, list)` or a map's `(value, key,
  map)`, and a map's or a set's `get`, `has`, `size`, and where it can
  change, `set` or `add`, `delete` and `clear`. Not yet `keys`, `values` and
  `entries`, which give JS iterators the js crate has no type of. 95.8%.
- **An iterable's `keys`, `values` and `entries` are `Box<dyn Iterator>`s**,
  the JS iterators they are (ADR 0140): `for (name, value) in
  headers.entries()` is `for (const [name, value] of headers.entries())`,
  an adapter is JS's iterator helper, `next()` is `$next(it)`. An array's
  key is its index, a `u32`; a set's its value. No type of the js crate's
  was needed. 96.4%.
- **An `[HTMLConstructor]` element's constructor isn't counted**: HTML
  marks the constructor, not its interface, so 69 that JS throws of,
  `new HTMLDivElement()`, were counted missing. 97.2% of 8,419.
- **A typed array is the js crate's** (ADR 0283), each of its element's
  Rust number: `getChannelData` gives a `&Float32Array`, and
  `ArrayBufferView` is a union of each, where only a `Uint8Array` was
  known. **A class's `LegacyWindowAlias` names it**, as SVG's IDL still
  does: an `SVGPoint` is a `DOMPoint`, an `SVGRect` a `DOMRect`, an
  `SVGMatrix` a `DOMMatrix`. **A union is named as it's written**, a
  typedef in it by its name, `StrOrBufferSource` of `(DOMString or
  BufferSource)`, as TypeScript's `string | BufferSource` is, not by every
  member it flattens to. 97.7%.
- **A stringifier is `to_string()`**, JS's `toString()`: of `stringifier;`
  and of a `stringifier attribute`, `url.to_string()` its `href`. 97.8%.
- **A static attribute is a function of its class's module**,
  `notification::permission()`, read at each call (ADR 0024); **a static
  method beside an instance's of its name is its module's too**,
  `response::json(data)` beside `response.json()`, as a method and a
  module function don't clash. 97.9%.
