# 1096. How a place holds `None` is `NoneForm`'s

Status: Accepted (2026-11-06, steps 1 and 2 of 3, #819). ADR 1085's structural
change 6, on [1086](1086-a-mut-refs-js-form-is-mut-ref-forms.md)'s model.
Amends [0791](0791-a-cells-storage-says-what-it-stores.md):
which places store `NullForNone` is one owner's answer.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This changes who
answers, not what's answered: every corpus snapshot is byte-identical, in
both builds, and so is every other test's JS, after each step. Step 2
found no reader that disagreed with its writers: no snapshot changed.

## Context

Whether an `Option`'s `None` is `undefined`, `null`, a key left out or a
box, and who may trust which, was decided in many places. A study of the
issues found nullability the area with the highest regression share, 7
of 24 (#443 #477 #519 #539 #541 #570 #571), each a writer and a reader, or
two readers, answering "is this `None` `null`" from different predicates
about the same place. `js::Storage::NullForNone` and `Storage::written`
owned how a write stores it (ADR 0791), but which places store so was
chosen at 20 sites in 13 files, each asking `#[rust_js::nullable]`'s
declarations itself: `is_nullable` (14 callers), `nullable_option` (12),
`nullable_params` (10), `nullable_local` (11), `null_where_made`,
`null_result`, `takes_nullable`, `holds_none_as_null`, and a library's
`nulled_param`.

## Decision

How a place holds a `None` is one enum, `NoneForm`, in
`representation/nones.rs` (re-exported from `options.rs`, whose answers of
an `Option`'s payload, whether its `Some` is boxed, stay there):

| form | `None` | of |
|---|---|---|
| `Undefined` | `undefined` | a plain `Option` |
| `Null` | `null`, what may be `undefined` stored `?? null` | a `#[rust_js::nullable]` place |
| `Optional` | left out where a literal's is `None`, else as `Null` | a nullable optional field (ADR 0810) |
| `Nested` | `undefined`, its `Some(None)` `null` | ADR 1017's held form |
| `Value` | `null`, its `Some(v)` `v` | a nullable field of a value never `null` (rd645) |

One function answers each kind of place, `none_form(tcx, NonePlace)`, of
a variant of `NonePlace`: `Field`, `Local`, `LocalOption`, `Result`,
`Param`, `BindingArg`, `BindingEntries` and `ForeignParam`. A new kind of
place is a new variant its match can't miss, not another function. A writer asks
it, and stores by `NoneForm::storage()`, never `Storage::NullForNone`
itself; `NoneForm::stores_null`, `storage` and `of_some` match every form.
MIR's places ask it through one adapter, `nulled.rs`'s `place_none_form`,
of the field a place is or holds an item of, or of the function's own
parameter (`own_param_none_form`).

### Step 2: what a reader trusts

What a reader may trust of a place's `None`, `Known::NoneNull` (tested
`=== null`) or `Known::NoneUndefined` (`=== undefined`), is the owner's
too: `NoneForm::trust(writers)`, of the place's form and who writes it,
`Writers`:

| writers | of |
|---|---|
| `Crate` | the crate's own writes, each by the form's `storage()` |
| `Exact` | each value exactly of the form: a literal `None`, a value `null` or a value already, JS's held to a binding's declared type |
| `Loose` | JS's too, which may leave a field out of a struct it makes (ADR 1072) |

A `Null` place's `None` is trusted `null` but of `Loose` writers; an
`Undefined` one's `undefined` only of `Exact` ones, as a plain write
stores what it's given; an `Optional`, a held (`Nested`, `Value`) and an
`Either` one's nothing. `Either` is the form added for it: a binding's
result it declares neither way, which JS gives `undefined` or `null`
(`regexp.exec(text)`), of `NonePlace::CallResult`, whose `#[rust_js::nullable]`
one's is `Null` and `#[rust_js::optional]` one's `Undefined` (ADR 0298,
amended). Each fact a reader set itself now comes from it: a binding's
call's (`calls.rs`'s `none_given_by`), a field's read
(`field_none_trust`, which replaces `bindings::null_or_value`) and a read
through a reference to one (`read_none_trust`), a literal `None` and a
function's own nulled parameter (`patterns.rs`), and a nulled value
written through a reference (`assign.rs`).

Two writers' bools became the owner's forms: a call of a thread-local's
cell carries its `NoneForm` (`Given::none_form`, of `QuietCells`'
`nullable` and `nullable_lent`, each now of the form of
`NonePlace::Local` or `NonePlace::LocalCell`), not `null_for_none`; and a
field's kept cell, `$fieldCell(x, "f", true)`, carries that it stores
`null` as a fact where it's made, `Known::NullStored`, which the
uncounted cell made of it reads, not its JS arguments.

## Next

- Step 3: an architecture test that makes `Storage::NullForNone` and those
  facts the owner's alone, as ADR 1086's does of `MutRefForm`.
- The JS-boundary test of what each `.d.ts` permits is #822.

## Tests

Byte-identical: the corpus in both builds (`RUST_JS_LIBRARIES=all`), and
every other test file. Mutations: `nones.ts`'s six, one of each arm, and
the 31 of the writers retargeted to the owner's form, each caught. Step
2: six more of `nones.ts`, of `trust`'s arms and each call result's form,
and the 15 of the readers and the thread-local cells retargeted, each
caught; `lowering-option-truth`'s test of a binding's undeclared result,
`pattern.exec(text) != null`, which JS gives `null`.
