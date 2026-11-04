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
