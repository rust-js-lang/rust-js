# 1096. How a place holds `None` is `NoneForm`'s

Status: Accepted (2026-11-06, step 1 of 3, #819). ADR 1085's structural
change 6, on [1086](1086-a-mut-refs-js-form-is-mut-ref-forms.md)'s model.
Amends [0791](0791-a-cells-storage-says-what-it-stores.md):
which places store `NullForNone` is one owner's answer.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This changes who
answers, not what's answered: every corpus snapshot is byte-identical, in
both builds, and so is every other test's JS.

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

Each kind of place has one function of the owner's:
`field_none_form`, `local_none_form`, `local_option_none_form`,
`result_none_form`, `param_none_form`, `binding_arg_none_form`,
`binding_entries_none_form` and `foreign_param_none_form`. A writer asks
it, and stores by `NoneForm::storage()`, never `Storage::NullForNone`
itself; `NoneForm::stores_null`, `storage` and `of_some` match every form.
MIR's places ask it through one adapter, `nulled.rs`'s `place_none_form`,
of the field a place is or holds an item of, or of the function's own
parameter (`own_param_none_form`).

## Next

- Step 2: what a reader may trust, `Known::NoneNull` and
  `Known::NoneUndefined`, made by the owner too, of the place and who may
  write it (`NoneForm::trust`), where today's disagreements live; each
  snapshot that changes there is a latent wrong answer, checked against
  native Rust and listed on #819.
- Step 3: an architecture test that makes `Storage::NullForNone` and those
  facts the owner's alone, as ADR 1086's does of `MutRefForm`.
- The JS-boundary test of what each `.d.ts` permits is #822.

## Tests

Byte-identical: the corpus in both builds (`RUST_JS_LIBRARIES=all`), and
every other test file. Mutations: `nones.ts`'s six, one of each arm, and
the 31 of the writers retargeted to the owner's form, each caught.
