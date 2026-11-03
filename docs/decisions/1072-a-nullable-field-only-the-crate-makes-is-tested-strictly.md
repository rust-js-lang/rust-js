# 1072. A nullable field only the crate makes is tested `=== null`

Status: accepted (2026-11-03, rd647 (B), the user's choice (1)). Amends
ADR 0030's `== null` test of a nullable field.

## Context

A `#[rust_js::nullable]` field's `None` is `null` where the crate makes
or sets it (ADRs 0275, 0772, 0810). Its read is still tested `== null`
(ADR 0030), because a JS object may leave the field out. React writes
`clones === null` where the port printed `clones == null`
(measureUpdateViewTransition's `state.clones`). The loose test was printed
everywhere, even on a direct read, `state.x == null`.

## Decision

A nullable field is tested `=== null` / `!== null` where it is `null` or
a value wherever it's read (`mir/nulled.rs`' `null_or_value_field`). That
needs all four of these:

- it's `#[rust_js::nullable]` and not `#[rust_js::optional]`, an `Option`
  or a cell of one (`null_where_made`);
- it's not of a type parameter's value, which generic code stores as it's
  given, `undefined` too;
- one of two things holds:
  - only the crate makes its struct (`pointers().only_the_crate_makes`):
    the crate builds values of it and no code that isn't the crate's
    makes one (`js_may_make`). A struct only JS gives, which may leave the
    field out, is never one;
  - or each constructor of the struct, `Class::of(function::with_this(..))`,
    sets the field first (rd628's `constructed`), so it's there from where
    `new` makes it.

The read says so (`Known::NoneNull`): read whole (`places.rs`), through a
`let` of it, a shared reference, a `RefCell` field's `borrow()` and its
`Deref`, `as_ref()`/`as_deref()`, a `Cell` field's `get()`
(`reads_null_or_value`, `as_ref_of_null_or_value`, consumed by
`mir_nones`), and `is_some()`/`is_none()` through a reference
(`null_or_value_tested`).

So every value the crate makes writes the field `null`. A derived
`Default` and an update's `..Default::default()` now write it `null` too,
as a literal does (`aggregates.rs`' `field_default`, through the
literal's own `nullable_field_made`). Before, they left the key out.

ADR 0262: no observable difference for every value the crate makes, since
each is `null` or a value. The case is a stricter reading of what the
crate already guarantees.

## Why

- It's upstream's test, and it's sound only where no `undefined` can
  reach the field. These are the places that can be proved.
- One fact on the read reuses the existing strict-presence printing
  (ADR 0298), as nullable parameters do (ADR 0861, rd613).

## Alternatives

- **(2) every non-optional nullable field strict, with the `.d.ts`
  required `T | null`**: rd613's promise that JS gives `null`. Rejected:
  a JS caller that leaves the key out, or JSX that omits a prop, would
  read `undefined` as a `Some`.
- **`!js_may_make` alone** (without "the crate builds it"): a struct only
  a JS caller gives, through a `pub fn`'s parameter, isn't
  `js_may_make`, and may leave the field out.

## Consequences

- A struct the crate builds that JS also gives, through a parameter of
  an exported function, is taken to be the program's own data, as ADR
  1022 (rd221) takes it: its nullable fields are tested `=== null`.
- Fields of type-parameter values, optional ones, and those of structs
  only JS gives keep `== null`.

## What holds it

- `test/corpus/nullable_field_tested_strictly.rs` against native Rust:
  - a plain `Option` field, a `Cell`'s and a `RefCell`'s, each made,
    defaulted, updated, `None` and set again;
  - an optional field, which stays loose;
  - a generic cursor whose `push` stores the caller's `None`, `undefined`,
    which a strict test would misread.
- `test/js-constructors.test.ts`, "a nullable field each constructor sets
  first is present": `moduleLoading !== null`, set first, and `inIf !=
  null`, set in an `if`, run with `new Response(undefined, false)`.
- `test/lowering-nullable-reads.test.ts`, "a nullable field is tested ===
  null only where only the crate makes its struct": a struct only JS
  gives, run with `{}`, against a crate-made one.
- Mutations `strict-*` in `lower/mir/nulled.ts`, `lower/mir/dispatch.ts`
  and `lower/mir/places.ts`, and `nullable-field-default-undefined` in
  `lower/aggregates.ts`.

## Amendment: a struct JS may give is never one only the crate makes (2026-11-03, #519, P0)

The first form above took a struct the crate builds as its own data even
where JS gives one too, following ADR 1022's rd221 for getters. That was
a wrong answer. `pub fn has_nonce(options: &Options) -> bool {
options.nonce.is_some() }` printed `options.nonce !== null`, so JS's
`hasNonce({})` read the missing field as a `Some`. Its `.d.ts`, `nonce?:
string | null | undefined`, lets JS leave the field out.

`only_the_crate_makes` now also needs that JS gives no value of the struct
(`FnPointers::js_given`). JS gives one in these positions:

- what an exported function is called with (`library::reachable`), its
  parameters' types;
- what a binding calls a function it's given with: a closure's, a fn
  item's, a fn pointer's or a `dyn Fn`'s arguments.

Each struct those hold counts too, through references, `Option`,
collections and the structs' own fields (`structs_held`). Such a struct's
nullable fields stay `== null`. So the Consequences' first point no
longer holds. ADR 1022's getter rule is unchanged.

Held by `test/lowering-nullable-reads.test.ts`'s strictness test. It runs
each position in Node: an exported parameter `{}`, a field of one, `{
inner: {} }`, and a `dyn Fn` and a closure a binding calls with `{}`. Each
struct is also built by the crate. Mutations `js-given-*` are in
`lower/analysis/fn_pointers.ts`.

## Amendment: such a field is declared there, `field: T | null` (2026-11-03, rd642 part 1)

A field tested `=== null` was still declared `field?: T | null |
undefined`. That told JS it may leave the field out, or write `undefined`
into an object the crate gave it, the same one hole #519 closed for what
JS gives. The `.d.ts` now declares each such field there, `x: unknown |
null`, with no `?` and no `| undefined`. One rule decides both the test
and the declaration, `bindings::null_or_value`. `mir/nulled.rs` asks it
of each read, and `pipeline.rs` collects the fields it holds for, the
`.d.ts`'s `present` set, which a constructor's set-first fields (rd628)
were before.

So the contract is explicit. A JS write of `undefined` into such a field,
or an object JS gives without it, is outside the declared type, as
passing a value of the wrong type is. Rust's answer isn't promised there.
A field rust-js tests loosely, of a struct JS may give, stays `field?: T |
null | undefined`, as JS may leave it out.

Held by `test/lowering-nullable-reads.test.ts`'s strictness test (`Own`'s
`x: unknown | null;`, `Options`' `nonce?: ..`), mutation
`strict-field-declared-optional` (`lower/pipeline.ts`); the rule's own
mutations moved to `lower/bindings/nullable.ts`.

## Amendment: every nullable field is declared there (2026-11-03, the user's decision)

A nullable field of a struct JS may give, React's ServerConsumerManifest
`moduleLoading`, stayed `moduleLoading?: unknown | null | undefined`,
since JS may leave it out. Upstream declares it required, `moduleLoading:
ModuleLoading` (`null | ..`), and `#[rust_js::nullable]` is the program's
promise that JS gives `null`, never `undefined` (rd613).

**Each `#[rust_js::nullable]` field that isn't optional, nor of a type
parameter's value, is declared `field: T | null`, whoever makes its
struct** (`bindings::declared_null_or_value`, which the `.d.ts` asks of
the attributes alone). The test is unchanged: `null_or_value` is that
and a struct only the crate makes, or that each constructor sets first,
so one JS may give stays `== null`. JS that ignores the type and leaves
the field out still reads `None`. The declaration is narrower than what
the code tolerates, so no answer changes. The `.d.ts`'s `present` set,
which `pipeline.rs` collected of `null_or_value`, is gone, with its
mutation `strict-field-declared-optional`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. It's a
declaration's form, and the JS is the same.

- Held by `test/lowering-nullable-reads.test.ts`'s strictness test,
  `Options`' `nonce: string | null`, which it still runs with `{}`, and
  `test/lowering-field-cells.test.ts`'s `return: Fiber | null`. Mutation
  `js-given-nullable-field-declared-optional` (`declarations.ts`); the
  rule's `strict-*` mutations follow it into `declared_null_or_value`.
