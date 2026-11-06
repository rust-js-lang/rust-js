# 1096. How a place holds `None` is `NoneForm`'s

Status: Accepted (2026-11-06, #819, its three steps). ADR 1085's structural
change 6, on [1086](1086-a-mut-refs-js-form-is-mut-ref-forms.md)'s model.
Amends [0791](0791-a-cells-storage-says-what-it-stores.md):
which places store `NullForNone` is one owner's answer.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This changes who
answers, not what's answered: every corpus snapshot is byte-identical, in
both builds, and so is every other test's JS, after each step. Steps 2
and 3 found no reader that disagreed with its writers: no snapshot changed.

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

### Step 3: the owner's alone

The predicates that answer "is a `None` `null` here" are asked only by the
owner, which `bindings` defines them for: `is_nullable`,
`null_where_made`, `declared_null_or_value`, `nullable_local`,
`nullable_local_cell`, `nullable_local_option`, `null_result`,
`nullable_result`, `optional_result`, `nullable_params`,
`takes_nullable`, `takes_nullable_entries`, `holds_none_as_null`,
`nullable_init` and a library's `nulled_param`. Their 35 other askers ask
`none_form`: the `.d.ts`'s fields, thread-locals and a collection's
values (`declarations.rs`, a second reader of the question, as #519's
was), the signature's parameters and result, the analysis of nulled
parameters, `patterns.rs`'s inference, the thread-locals' made values and
cells, a class's fields set first and a `RefCell` field's refusal. For them
the owner adds `NonePlace::FnResult` (a function's own `#[rust_js::nullable]`,
not a thread-local `init`'s), `NonePlace::Param` by index with
`null_params`, its indexes, `NonePlace::CollectionValue`, and
`BindingArg`'s payload where it's asked. `null_where_made`,
`declared_null_or_value` and `nullable_init` are gone.

`test/architecture.test.ts`'s "whether a `None` is `null` is
none_form's alone" holds it, as ADR 1086's test does of `MutRefForm`: no
module of `src/lower` but the owner and `bindings` calls those
predicates; none makes `Storage::NullForNone`, `Known::NoneNull` or
`Known::NoneUndefined`, only matches or tests them; and only `cells.rs`,
where a field's kept cell is made, says `Known::NullStored`.

The shape predicates, `is_optional`, `is_held`, `holds_value`,
`holds_nested` and `nullable_option`, answer another question, a
declared payload's shape, and stay `bindings`'. Of `options.rs`'s
type-level answers, `holds_value` and `undefined_payload` consume one
rule, `bindings::undefined_never_null`; `holds_nested`'s refusal,
`nested_refused`, re-derives `may_look_like_none`'s "may this payload
look like `None`" with a peel and a catalog of its own, which disagree
of a `Weak`, a `Pin`, a `MaybeUninit` and a class of no fields: #832.

## Next

- #832: `nested_refused` asks `may_look_like_none`.
- The JS-boundary test of what each `.d.ts` permits is #822.

## Tests

Byte-identical: the corpus in both builds (`RUST_JS_LIBRARIES=all`), and
every other test file. Mutations: `nones.ts`'s six, one of each arm, and
the 31 of the writers retargeted to the owner's form, each caught. Step
2: six more of `nones.ts`, of `trust`'s arms and each call result's form,
and the 15 of the readers and the thread-local cells retargeted, each
caught; `lowering-option-truth`'s test of a binding's undeclared result,
`pattern.exec(text) != null`, which JS gives `null`. Step 3: the 29
mutations of the askers routed to the owner retargeted, each caught (one,
`optional-field-made-null`, removed with `null_where_made`, which
`optional-field-none-as-null` covers); the architecture test, shown to
fail of a predicate asked and a fact made outside the owner.
