# 1091. A function's declaration prints its `JsSignature`

Status: Accepted (2026-11-04, the user's approval of the structural fix).
Amends [1077](1077-a-function-js-cant-call-as-declared-isnt-declared.md):
what a function takes after its arguments, and which of it a JS caller
can't give, is lowering's `evidence_slots`' list, not one the
declarations make again.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This changes who
answers, not what's answered: every corpus case's `.d.ts` is
byte-identical, and so is every declaration test's.

## Context

A study of 83 wrong answers (ADR 1085) found five where a `.d.ts` said
other than its JS did, the F-DTS family: what a function takes after its
arguments, a fact of a type (#549), a dictionary, a caller's location and
a const parameter's value (#560), each declaration path's own (#559); a
`&mut` that's a box (#619); a `dyn` that's a pair (#620).

The mechanism was one: lowering decides what a function's JS takes, and
`src/lower/declarations/` decided it again from Rust's `FnSig`. Its
`evidence` re-read the owners `evidence_slots` reads (`takes_location`,
`const_params`, `bounds`, the type facts), in an order it kept itself;
and five declaration paths, a free function, a namespace's function, an
object of methods, a class and a trait's dictionary, each said which of
its Rust parameters its JS takes: four removed a `self` themselves, by
`params.remove(0)` or `skip(1)`. `1639d621b`'s `ungiven` reached two of
those paths, and `707d7eea0` had to add the other three (#559). Where
the rule changes, five places had to.

## Decision

What a function's JS takes and gives, as a JS caller calls it, is one
value lowering makes, `JsSignature` (`src/lower/signatures.rs`,
`FnCx::js_signature`):

```rust
struct JsSignature<'tcx> { params: Vec<JsParam<'tcx>>, result: Form<'tcx> }
struct JsParam<'tcx> { origin: Origin<'tcx>, name: String, form: Form<'tcx> }
enum Origin<'tcx> { Rust(usize), Evidence(Slot<'tcx>) }
enum Form<'tcx> {
    Value { ty: Ty<'tcx>, optional: bool }, // a trailing `Option` may be left out
    Nullable(Ty<'tcx>),                     // `T | null`
    Rest(Ty<'tcx>),                         // `...args: T[]`
    Dictionary(ty::TraitRef<'tcx>),         // a bound's dictionary
    Unknowable,                             // a type's fact, `TSize`
    LeftOut,                                // a drop, `undefined` of none
    Void,                                   // a result of `()`
}
```

Each part is its owner's:

- which Rust parameters it takes: all but a `self` its JS takes as `this`
  or not at all, `classes::receiver`'s, which `receiver_as_this` asks of a
  body too;
- what it takes after them, and in which order: `evidence_slots`'
  (`Scope::Whole`), the list each call gives and its body names;
- which `Option` is given `null` (`nullable_params`), a rest parameter
  (`takes_rest`), a nullable result (`nullable_result`), and which
  trailing parameters may be left out (`optional_from`, which a function
  type's declaration asks too);
- each parameter's name: `parameter_name`'s (`js_parameter_name`).

The declarations get it through `SignatureForms::signature`, beside the
`&mut` and `dyn` answers (ADR 1086), and every path prints it: no path
removes a parameter, adds one, or asks what a function takes on its own.
Which of it a JS caller can't give is `JsSignature::ungiven`: a type's
fact first, then a bound's dictionary of a trait the declarations don't
declare, as ADR 1077 says, of the same list.

A TypeScript type is the representation owners' still: a part's `&mut`
`MutRefForm::ts_type`, a `dyn` its pair (ADR 1077, amended), each asked
in the context of the item declared.

## Why

- One rule, one place: a new kind of evidence, or a new form of `self`,
  is added to `evidence_slots` or `classes::receiver`, and every
  declaration path says it.
- The paths print; they don't decide. An architecture test holds that no
  declaration file names the owners a signature is made of, nor removes a
  parameter from one.

## Rejected

- **A helper the five paths call.** `ungiven` was that, and each new
  path had to remember to call it (#559).
- **Printing from lowered JS functions' parameters.** A `js::Function`
  has names, not types: the declaration needs the Rust type of each, and
  which owner made it.

## Consequences

- `declarations::module` no longer takes the type facts, the bounds'
  dictionaries given or the functions that take their caller's location:
  `evidence_slots` reads them.
- The namespace's and the trait of classes' paths removed a first
  parameter wherever there was one; `classes::receiver` says it of a
  method alone. A namespace's functions are its `&self` methods alone
  (ADR 1037), so no declaration changes.
- `classes::receiver` is in `classes/receivers.rs`, with
  `body_receiver`, the part `receiver_as_this` asks of a body: a trait
  of classes' default is lowered of its first parameter into its impl's
  dictionary, and `defaults::prototype_defaults` takes it as `this`.

## Amended (#661): a type parameter bound by `Fn` is its function

Case ([0262](0262-when-rust-and-js-disagree.md)): none for the JS, which
doesn't change; the declaration was looser than it (AGENTS.md's ports
rule): `twice<F: Fn(u32) -> u32>(f: F)` was `twice<F>(f: F)`, which a
TypeScript caller could give any value its JS then calls.

A `JsSignature` says which of a function's own type parameters it's given
a JS function of, `callables`: each bound by `Fn`, `FnMut` or `FnOnce` of
what lowering makes a JS function (`is_callable`), with what its bound
takes and gives. The declaration erases each to that function type, as
it did an `impl Fn(..)`'s, and leaves it out of its type parameters:
`twice(f: (value: number) => number, n: number): number`. One rule now
answers both, where `erased` had answered the `impl Fn` alone, of a
synthetic parameter. An impl's `F: Fn()` is its type's parameter,
`Holder<F>`, declared as its type takes it, so it's no function's own.

## Amended (#662): a trait of `&mut self` methods is declared

Case ([0262](0262-when-rust-and-js-disagree.md)): none for the JS, which
doesn't change; a TypeScript consumer of such a trait's dictionaries, or
of a `dyn` of it, had no type for them, `any`.

`traits::method_dictionary` refused every trait with a `&mut` in a
method's signature, "which may be a box". What a `&mut` a method takes is
in its entry is `MutRefForm`'s answer in the trait's terms (ADR 1086),
the one generic code gives it: a `&mut self` is a type parameter's, a
box, `{ value: Self }`, which each impl's entry takes and unboxes,
`bump: (arg0) => clicksCounter_bump(arg0.value)`; a `&mut u32` a box, a
`&mut Vec<u32>` the array, in its trait's terms as in each impl's, since
neither names `Self`. The declaration asks that owner through
`SignatureForms`, so it says what the entry takes:

```ts
export interface Counter<Self> {
    bump(self: { value: Self; }): void;
    add(self: Self, total: { value: number; }): void;
}
export const clicksCounter: Counter<Clicks>;
export function bumpTwice<C>(c: { value: C; }, CCounter: Counter<C>): number;
export function bumpDyn(c: { value: unknown; impl: Counter<unknown>; }): number;
```

Still refused, where no owner says what an entry takes: a method that
gives a `&mut`, `fn slot(&mut self) -> &mut u32`, and one that takes a
`&mut` to what's of its `Self` besides its receiver, `fn swap_with(&mut
self, other: &mut Self)`. Held by `test/declarations-ungiven.test.ts`'s "a
trait of `&mut self` methods is the type of its dictionaries", whose
TypeScript caller type-checks and gives what Rust's calls give.

## Amended: a generated TypeScript caller holds each declaration to its JS

Case ([0262](0262-when-rust-and-js-disagree.md)): none; a test.

The architecture test holds who decides a declared signature; it can't
hold that what's decided is what the JS does. `test/declarations-conformance.test.ts`
does, of a set of corpus cases (`declared_signatures.rs`, written for it,
and seven that export functions): for each exported function, it makes a
value of each parameter's declared type, a `&mut`'s box or object as the
`.d.ts` says it is, and calls the function from a generated TypeScript
caller, which `tsc` checks under `strict` and Node runs. The same calls,
made of the same values in a native program of the case, must print the
same: each result, and each `&mut`'s value after. A declaration that says
a box where the JS takes the object, or the object where it takes a box,
fails one or the other (`mut-ref-ts-type-its-pointee` is its mutation).

TypeScript 7 has no JS API, so the test reads the `.d.ts` as rust-js
prints it, and each case's Rust signatures from their text: of the types
it has values of, numbers, `bool`, strings, `Vec`s and slices, `Option`s,
tuples, `&`, `&mut`, a case's structs of those, `impl Fn` and a type
parameter bound by `Fn` or by std's `Clone`, `Copy`, `PartialOrd` and the
like. A function of another type, or one a declaration leaves out (ADR
1077), is skipped; a case of none fails. It found no wrong answer in its
first 70 calls. It runs in the nightly's suite (ADR 1030), as a regression
net, and in each mutation that names it.
