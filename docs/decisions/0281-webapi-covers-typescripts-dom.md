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
- MDN links are the measure's key: a binding written without one isn't
  counted.
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
