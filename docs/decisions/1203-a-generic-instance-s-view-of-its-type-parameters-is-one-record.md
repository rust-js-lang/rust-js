# 1203. A generic instance's view of its type parameters is one record, converted at five crossings

Status: Accepted (2026-11-25, #2539, the lead's call on Campaign 6's
design, posted on #2539 as comment 7143281010). Step 0, the shadow, is
built; the steps after it land one at a time, each amending this.
Will supersede the "Form" part of [1115](1115-what-generic-code-needs-of-its-type-parameters-has-one-catalogue.md)'s
catalogue, and amend [0061](0061-generic-iterators.md),
[0454](0454-a-mut-given-as-a-type-parameter.md),
[0474](0474-an-iterator-another-bound-reads-is-given-as-it-is.md),
[0552](0552-an-into-iterator-held-as-it-is-is-given-as-it-is.md) and
[0914](0914-a-librarys-generic-function-is-its-programs.md) as each
step lands.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Each conversion
is Rust's semantics; a crossing that can't convert yet refuses, through
[1201](1201-campaign-6-s-stopgap-refusals-say-so-by-the-refusal-api.md)'s
stopgap cells.

## Context

Generic code holds some values in another form than its caller does: a
`&mut` object in a box, a raw pointer as a handle, a write's result as
`{ TAG }` or as a value, an iterator JS holds as no object in a place of
its own, a crate's iterator as a JS iterator. Today each form is a
`generic_facts::Form`, and `form_of` takes the first that differs at a
crossing. The other readers of a type parameter's form decide it again
on their own: the dictionaries, the export contract a library's consumer
reads (`instances::needed`), and the three functions that say whether an
iterator is held as it is.

Campaign 6's matrix (#2539) crossed each form or fact with each crossing:
arguments, results, a dictionary entry's made value, a given iterator's
items, and a generic struct's fields. Every wrong answer it found was at
the last three crossings or across crates, where nothing converts because
nobody asks generic code's view there. The P0s fixed on 2026-11-25
(#2540, #2541, #2545) were second readers of a form that no crossing told.

## Decision

- **`ParamForm`**, one record per generic instance and position, is
  generic code's view. Its fields are: `iterated`, `placed`, `mut_ref`,
  `pointer`, `write_result`, `hint_knowing`, `index_reader` and `cell`.
  Each is filled by asking its owner, never re-derived. `cell` is
  `analysis/cell_form.rs`'s answer, read and not forked.
- **One conversion per crossing**, in `generic_facts/crossing.rs`:
  `cross_in`, `cross_out`, `dictionary_through` over one catalog of a
  dictionary's entries, `items_in`, and `field_in`/`field_out`. They are
  the only writers of the form helpers. A crossing that can't convert a
  field refuses through `refusal::Crossing`.
- **The library contract records each export's `ParamForm`s**, and
  `instances::needed`'s `gives_more` becomes "differs in any field".
- **Steps**, each landing alone and byte-identical where no moves are
  predicted: 0, a shadow measuring where `ParamForm` would decide
  otherwise; 1, the arguments; 2, the results; 3, the dictionaries
  (lifting #2565); 4, the items (lifting #2566, #2535); 5, the fields; 6,
  the library pair.

### Step 0: the `param-form` shadow (ADR 1199's kind)

`--shadow param-form=<file>` writes two kinds of line. No output changes.

- `site`: one for each value `form_given` or `form_back` converts. It
  gives the form taken and the plan, which is every form
  `form_differs` says differs there, in `ParamForm`'s order
  (`generic_facts/param_form.rs`). `agrees` means the plan is the taken
  form alone. `drops` means more forms differ than the one taken, which
  today's single `Form` can't say. `other` means the plan's one form
  isn't the one taken.
- `iterated`: one for each answer of `given_as_it_is` and
  `projection_as_it_is`, beside the bounds of the type that read its
  value (`iterators/held_shadow.rs`). `drifts` means one of those bounds
  meets a JS iterator, where `held_as_iterator`'s one rule would hold it
  as it is: #2540's and #2541's kind.

`bun scripts/param-form-shadow.ts` gathers the corpus's lines in the
binary, program and library builds and counts them; `--lines` counts a
port's (`RUST_JS_SHADOW=param-form=<dir>`).

### Step 0's numbers (the corpus at e886a7778)

In each of the binary, program and library builds, 580 values in 167
cases are converted where they cross, and every one agrees: no crossing
has a second form that differs. The forms taken, of the binary's: a box
given 359 times and given back 92; placed 44 and 13; a handle 12 and 17;
tagged 14 and 12; a held box 7 and 10. So steps 1 and 2 (the arguments
and the results) move no output, as predicted.

The binary asks how an iterator is held 6,190 times: as it is 2,721,
a JS iterator 3,469. One answer drifts. In `nested_item_drops`, `concat`'s
`I::Item`, bound by `IntoIterator` and `Extend`, is a JS iterator to
`projection_as_it_is`. It's given as it is through its `IntoIterator`
(#278), and the case is native-equal, so it's no wrong answer.
`held_as_iterator`'s one rule answers it as it is. The program build
gives the same numbers; the library build asks 6,203 times, also with
one drift.

The shadow can't see where nothing converts today: a dictionary entry's
made value and a given iterator's items (#2559, #2560), steps 3 and 4.
Those are the matrix's cells.

## Consequences

- Step 0's numbers go to the lead before step 1 is built.
- A `drops` or `other` line is a crossing whose conversion `ParamForm`
  would change. A `drifts` line is an iterator held otherwise than a
  reader of it assumes. Each is a candidate wrong answer, checked
  against native Rust before any step moves it.
- Held by test/param-form-shadow.test.ts.
