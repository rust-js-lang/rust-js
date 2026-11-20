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

## Amendment: the `None` matrix (2026-11-08, #855)

Each writer asks a place's `NoneForm` and each reader what it may trust of
it apart, so a writer and a reader of two forms are where they disagree:
nullability had the highest share of regressions (7 of 24, #855). The
space is small enough to run whole. `test/none-matrix.ts` writes a program
for each **place** (a local; a plain, a nullable and a nullable optional
field, its form `Nested` of an `Option<Option<i32>>` and `Value` of a `()`
by its payload; a function's result and its parameter, each plain and
`#[rust_js::nullable]`; a nullable thread-local's `RefCell`; a `Vec`'s
item; a `RefCell`; and a `&mut`), each **payload** (`i32`, `String`, `()`,
`Option<i32>` and a struct) and each **writer** (a literal, through a
local, from a nullable field, from a function and a nullable function,
`take`, `map`, `filter`, a cell's `take`, `pop`), its `Some` and its
`None` each read by each **reader** (`{:?}`, `is_some` and `is_none`, a
`match`, `== None`, `?`, `map_or`, `if let`, given to a nullable parameter
and to a plain field): 580 programs, each in both builds, all a night and
six per push where a `None`'s form changes (`check:fast`'s `nones`). The
two places' payloads rust-js refuses by design, an optional field of `()`
or of a nested `Option`, have no program, with why.

Its first run: #957 (P0), a nested nullable field written from the
crate's own nullable function's `None`, which is `null`, reads it as
`Some(None)`, its conversion testing `=== undefined`; #958 (P1), a
temporary cell's `take`, `Cell::new(x).take()`, crashes rust-js (58
programs); and #959 (P3), the refusal of `optional` of a nested field
names a `#[rust_js::nullable]` field as what it needs, which it is.
`wrong` and `refused` list them, keyed `<build>:<program>`, so they only
shrink.

## Amendment: a local's `None` is decided once (2026-11-19, #2112)

Change C's second step (S1) of the second architecture campaign. A body's
`Option` locals had two answers of their `None`, two analyses of one
local, each read alone: `OptionWrites::null_nones` (`mir_null_nones`,
ADRs 0275, 0861), which locals the body makes `null` where they're made,
and `OptionWrites::nones` (`mir_nones`, ADR 0298), what each read of a
local may trust, stamped on it as `Known::NoneUndefined` or
`Known::NoneNull`, which makes its test `x !== undefined` or `x !==
null`. They disagreed: over the corpus, 70 locals the body makes `null`
were also said to hold only `undefined`, each read stamped
`NoneUndefined`, whose test is true of the `null` stored. Nothing printed
a wrong answer (the corpus ran as native, and no corpus or React port JS
tests a variable made `: null` against `undefined`), because the first
analysis's restrictions, that such a local is read only by a nullable
place or by a test that reads `null` and `undefined` alike, kept the
second's fact unread: a latent wrong answer, held off by another
analysis's rules.

Now one owner answers both, `mir/none_forms.rs`'s `NoneForms`, built once
per body from the two analyses, which are its inputs alone: `OptionWrites`
keeps only `none_forms`, so no consumer reads either half
(`assign.rs`, `nulled.rs`, `operands.rs::none_read`, `places.rs` ask
`made_null` and `read`). A local made `null` is trusted to hold `null`
alone, `NoneNull`, or nothing, never `undefined` alone, by construction:
`NoneForms::of` drops `NoneUndefined` of it. Case (ADR 0262): none; no
output moves, every corpus and snapshot JS and React's port byte-identical.
The unit test `lower::mir::none_forms::tests` holds the invariant; the
mutations `made-null-read-as-undefined` and `made-null-unread` are caught
by it. test/architecture-nones.test.ts reads a `matches!` of a fact as a
read and leaves unit tests' fixtures out. Parameters' forms
(`own_param_none_form`) and fields' (`NonePlace::Field`) stay where they
are, the owner of def-keyed forms; Change C's later steps (#2112) fold
the taint (`js_nones`) and the `Known`-fact halves into this owner.

## Amendment: what an exit gives as it is is MIR's (2026-11-19, #2112)

Change C's third step (S2). An exit that gives `None` as `undefined`
alone (`one_none_given`: a binding's optional argument, a function JS
calls) gives `value ?? undefined` where what's written may hold a `null`
JS gave. It asked `never_null(&Expr)` whether the value's JS was never
`null`, a judgement of the JS's shape and its `Known` facts, beside the
taint's (`none_trusted`). Now `NoneForms`' owner answers by MIR,
`gives_no_null`: a place every `null` of which is a written `js::null()`,
JS's on purpose (`JsNones::holds_written_null`, ADR 0935), or a local a
test proved `Some` there (`proved_present`). A call's write of
`js::null()`, or of `?`'s `None`, Rust's own (`FromResidual`), is given
as it is by `one_none_returned`, which now knows the call's callee. The
taint follows a JS object's `Deref` to the class it extends, the object
itself (`recognition::is_js_object_deref`, ADR 0024), so a `Some` of an
error `new` made, `Some::<&JsError>(js::error::new(..))`, holds no `null`
JS gave. And a prop `jsx!` omitted, a local each write of which is
react's `__omitted()` (`omitted_prop`, `bindings::is_omitted`, ADR
0213), is Rust's `None`, never a `null` JS gave: MIR's answer, as the
taint can't give it, a `Box<dyn Fn>`'s or a `Ref`'s sharing the class of
the props' field another crate's JS may make.

Measured as a differential before the switch, `never_null`'s answer
against the MIR one where the decision is live: zero disagreements over
every suite `check:fast --all` runs and React's port. (A first one, over
the corpus, the snapshots and the port alone, missed 1,061 in the JSX
and Next.js tests, each a prop `jsx!` omitted, which the gate's refusal
of flattened props found: a differential runs over every suite.) and after it, no output
moves, of the corpus, the snapshots, React's port and the vercel-ui SDK.
Each arm stays only where a test catches its removal: arms that the
taint already decided (a constant, a local read `undefined` alone, a
local made `Some`, a payload check, and a `new` result's) went, as their
mutations survived every program. Kept: the written `js::null()` arm
(React's Fizz server's `js::null()` arguments, which a ladder would have
left out, `undefined` for JS's `null`), the proved arm (React DOM's
scroll-end `clearTimeout(existingTimer)`), the per-write `js::null()`
and `?` returns, and the `Deref` passthrough (React's Flight server's
`destroy(Some(js::error::new(..)))`), and the omitted prop's
(test/jsx-props-resolved, whose flattened props it would refuse). `never_null` keeps its two other
callers, a callback's arguments (`ones_given_to_a_callback`) and the
items `function::bind` binds (`ones_bound`), for Change C's later
steps.

Rows: test/one-none-exits.test.ts, 7 tests; test/snapshots/throws.
Mutations `no-null-written-null-unread`, `no-null-proved-unread`,
`returned-js-null-coalesced`, `returned-residual-coalesced` and
`object-deref-unfollowed` and `omitted-prop-unread`, each caught; `js-nones-binding-results-
trusted`, `returned-uses-as-they-are` and `returned-calls-as-they-are`
re-anchored, caught.

## Amendment: a binding's optional parameter has one owner (2026-11-19, #2112)

Change C's fourth step (S3a). Whether a binding's parameter is a native
optional argument, TypeScript's `x?`, had two answers: `bindings::
is_optional_param`, an `Option` but not a setter's value (ADR 0648) nor
a `#[rust_js::nullable]` one, and `nulled_params::optional_argument`, an
`Option`, which `mir/nulled/one_none.rs` and `items.rs` asked. The second
is gone; each asks the owner. Where they differed, a setter's value:
`Error.prepareStackTrace = previous ?? undefined` is now `=
previous`, what the getter gave set back as it is, as ADR 0648 says and
upstream React writes, 18 sites in React's port's 3 files. A Rust-made
`None` still writes `undefined`. ADR 0262's case: none observable, a
read back gives `None` either way, so JS's behaviour. The owner reads a
parameter's name only of a function or a method, which a constructor,
`Option::Some`, given to a callback, isn't (its first form panicked on
one). Mutations `optional-param-name-of-a-constructor` (corpus
`held_filter_map_reversed`) and `none-given-to-any-binding-parameter`,
re-anchored, caught.

## Amendment: what a test proved of a read is asked, never carried (2026-11-19, #2112)

Change C's fifth step (S4). What a test proved of an `Option` where it's
read was a fact stamped on the read's JS, `Known::Proved` (`Some` there)
and `Known::ProvedDefined` (never `undefined`, maybe `null`), by
`operands.rs` of a variable's read and `field_read_marked` of a field's
or a slice end's. Copies, presence tests and `of_presence` carried it to
other expressions, past the test that proved it, so tidy had to forget it
as it began (`forget_proved`, #1942) and a contract checked none was
left. It's now MIR's one answer, `NoneForms::presence(state, operand)`:
`Presence::{Present, Defined, Unknown}`, asked where each use is lowered.

- An `unwrap()` or `unwrap_or(d)`'s receiver: `calls::Given::receiver`,
  filled with the other per-call MIR facts (`call_given`).
- A value stored in a nullable place: `stored_given`, an `Option` proved
  there is `null` or a value already, `Given::Nulled`; so a `Cell` set, a
  plain assignment, a `*r = v`. A call that gives its `Option` through,
  `js::cast(found)` or `found.map(js::unknown)`, returned from a nullable
  function, is what's proved of `found` (`call_nulled`, `call_presence`).
- A binding's trailing optional argument, `optional_ladder`'s
  `surely_there`: `may_be_undefined` alone; its answer with the facts gone
  is the same at every site, so it asks nothing more.

`presence` follows a temporary to what it copies (`copied_from`): a copy
or borrow written in the block being read, or a call that goes straight
on to it and gives its `Option` through (`passes_through`). Only
straight-line code: a test edge starts a block, so what's proved where
the use is lowered, a proof no write since its test breaks, held where
the copy was made. A copy taken before a test, `let t = x; if x.is_some()
{ t.unwrap_or(d) }`, isn't followed. The `js::cast` check is one
predicate, `is_its_argument`, which `written_given` asks too.

Proof: before switching, probes compared each decision as it was with
the decision after stripping the facts from every node, OR'd with
`presence`, over every suite of `check:fast --all` and React's port:
`unwrap`/`unwrap_or`, `may_be_undefined` where a nullable place stores
(`Storage::written`'s callers, `store_again`), the `*r = v` branch of
`assign`, the ladder's `surely_there`, and `never_null`. They agreed at
every site once `presence` followed copies, `map` and `js::cast`. With
the facts deleted, every suite's output and the port are byte-identical.
ADR 0262's case: none observable, no output moves. `Known::Proved`,
`Known::ProvedDefined`, `forget_proved`, tidy's "proved forgotten" pass,
the `NOTHING_PROVED` contract and `field_read_marked` are gone, and so is
what only bridged them: `assign`'s `never_undefined` under `*r = v`, which
`stored_given` now answers;
architecture-nones holds that none comes back and that the proofs are
asked by `presence` alone. Mutations moved from `operands.rs` to
`none_forms.rs` with the proofs they break, and a slice end's to
`proved_present`, which asks it; new ones for following copies, `map`,
`js::cast` and `stored_given`. What the first probes took for a proof
read through a reference, `&(*x)`, was a reference stored, no decision:
following one moved nothing, so `presence` doesn't. Two survivors in the
files this touches, `js-null-given-undefined` and `option-identity-map`,
survive before it too (1b3681486), reported, not this change's.

## Amendment: a `Some` its writes make is `presence`'s too (2026-11-20, #2112, #2178)

Change C's sixth step (S5, its first part). "Is this `Option` a `Some`
where it's read?" had two answers: `NoneForms::presence`, of tests that
prove it on every path, and `made_some`, of writes that each make one
(`made_some_or_none`), which `somes_given` stamped on a call's arguments as
`Known::Present` for the call of each count of a function's trailing
`Option`s (#1876) and the `?? undefined` of what's given a callback JS may
have made (`never_null`). `presence` now asks `made_some` with the tests'
proofs, one answer; the stamps are gone, and each call computes what MIR
proves of its arguments once (`presences`, `presences_in_a_tuple`), given
the ladder (`optional_ladder`'s `present`, past what a binding is called
on, `offset`) and `ones_given_to_a_callback`.

Every reader of `presence` sees a `Some` its writes make: an `unwrap()` of
one is its value, `$unwrap` gone; an `unwrap_or(d)`'s default goes; a
binding's trailing `Some(&q)` is given as it is, untested. Ten corpus
snapshots and one bindings test move; React's port doesn't. ADR 0262's
case: none observable. Rust's `unwrap` can't panic there and the default
is never taken, so behaviour is the same, and the JS reads better. A
`Some` its writes make is one whose payload `payload_may_be_null` doesn't
say may be a caught `null` (ADR 0030, #1939); one whose payload may look
like `None`, a null pointer, an `any` value, is boxed, `$some`, and its
`unwrap` still unboxes it (`some_of_what_may_be_nullish`).

## Amendment: a strict test of being set is its own operator (2026-11-20, #2112)

Change C's S5, its second part's first step. An `Option`'s test of being
set where its `None` is only one, `x !== undefined` (ADR 0298, amended),
was a loose `x != null` carrying `Known::NoneUndefined`, which tidy's
passes moved and the printer read to write it strict
(`strict_presence`). It's now its own operator, `Op::NoneNe(OnlyNone)` and
`Op::NoneEq(OnlyNone)`, its right operand `null` as a loose one's, which
the printer writes as that `None`: the decision carried as the JS it is.
A program's own `===`, `Op::Eq`, is never one: `Op::tests_set` is the one
answer of whether an operator tests an `Option`'s being set, which
`null_test` and each pass that folds, merges or moves such a test ask,
and a ternary of `js::strict_equals` never folds into `?.` or `??` (a
lowering-nullable-tests row). A strict test folds as the loose one did:
it's made only of a read whose `None` is only that one, whose payload is
boxed where it may look like `None`, so its operand is never the other
nullish; a `Some` of a caught JS `null` (ADR 0030) is the one question
left, which the loose test's fold met alike (under investigation). Output
is byte-identical; the facts still decide which operator `Expr::presence`
makes, until its callers give MIR's answer.

## Amendment: a `js::cast` holds the `None` it's given (2026-11-20, #2112)

`mir_nones`, MIR's answer of the one `None` a local's writes give, knew
a call's result only of `none_given_by` and `as_ref_of_null_or_value`:
`let prev_cache_pool: Option<&SpawnedCachePool> = unsafe {
js::cast(prev_offscreen_state.cache_pool) }` of React's
`updateSuspenseOffscreenState` held a `None` it couldn't tell, so its test
was `!= null`, where the nullable field it casts, which only the crate
makes, is `null` alone and upstream writes `!== null`. A call that's its
argument in JS, `js::cast(found)` (`is_its_argument`), is that argument:
`gives_its_option` (`mir/nulled/proofs.rs`) answers it once, which
`passes_through`, of what's proved of it, and `mir_nones` both ask; a
local it's given is followed as a copy is, a field read by its trust
(`read_none_trust`).

Case A of ADR 0262: no observable difference, a tighter test of the same
value. React's port moves one line, `prevCachePool !== null`, upstream's
(the pin to `c947516`); the corpus is unchanged. Test
`lowering-nullable-classified`'s "a js::cast of a nullable field held in
a let is tested as the field is"; mutation `cast-none-not-followed`, new,
caught; `passes-through-no-cast` and `passes-through-no-map` follow the
code into `gives_its_option`, caught.

## Amendment: a test of being set is made of MIR's `None` (2026-11-20, #2112)

`Expr::presence`, the one test of an `Option`'s being there, chose its
operator of the read stamps its operand carried, `Known::NoneUndefined`
and `Known::NoneNull`, which lowering set as it read and carried through
what it built. It's now given the one `None` its operand can be, MIR's
(`NoneForms::place_none`): `x !== undefined` or `x !== null` where it says
one, `x != null` where it says neither or a caller has no operand to ask.
A differential of the two answers over check:fast and React's port, each
test logged where they differed, came down from 53 to 25 before the
switch, by making MIR's answer whole:

- `as_ref()` of a shared `Option` is the `Option` itself in JS
  (`Std::Pointee`, ADR 0023): `gives_its_option` answers it with
  `js::cast`, so `o.as_ref().map(..)` of a local `None` reads `o`'s.
- A closure's capture holds what its maker gave it: where a closure is
  made, each capture's `place_none` is noted (`NulledSeen::captures_nones`,
  beside `captures_nulled`), and in its body a read of the capture, or
  through a shared borrow of one (`capture_read`, `capture_lent`, the
  state's owner `mir/nulled.rs`), answers of that. React's Flight servers'
  `debug_channel_readable.is_none()` in `renderToPipeableStream`'s pipe
  closure, `debugChannelReadable === undefined`, upstream's.
- The receiver of `is_none()` and its kind is the `Option` its reference
  is to (`Given::receiver_none`, of `place_none`); what a reference refers
  to is its writes', or of a borrow of a field, the field's trust
  (`read_none_trust`).
- A drop of an `Option` by its variants, one moved out of in part, asks
  the place's `None` (`open_drop_variants`).

Of the 25 left: 21 where MIR knows the one `None` and the read stamps
didn't, each test made stricter, the user's lead's decision with each JS
difference shown: React's port's `updateQueue.events !== null` and
`onStartGestureTransitionFinish !== null`, upstream's own, and
`createTask !== undefined`, ADR 0811's optional property, whose `None` is
`undefined`; DOM's `textContent !== null` and `window.event !==
undefined`, as their declared types say. 4 where only the stamps knew, a
field of a struct built where it's read, `Plain { v: v.clone() }.v` of a
nullable `v`, which MIR reads as a plain field, either: tested `== null`,
the lead's decision, as a loose test is never wrong and seeing through an
aggregate's write and `Option::clone` isn't worth its machinery for a
shape only the none matrix writes.

Case A of ADR 0262: no observable difference, a test of the same value
made tighter or looser. React's port moves three lines (the pin), the
guide's interop/global-scope.md one fence. Corpus case
`option_none_forms_through_captures` runs each against native, its
snapshot holding the tests; mutations `capture-nones-unrecorded`,
`capture-lent-unfollowed`, `deref-field-trust-unasked`,
`receiver-none-of-the-reference`, `as-ref-none-not-followed` and
`drop-variants-none-unasked`, new, caught (`snapshots: true`). The read
stamps themselves go next.

## Amendment: a trailing argument's `null` is MIR's (2026-11-20, #2112)

The first reader of the read stamps moved to MIR's answer: a call of each
count of a function's trailing `Option`s (`optional_ladder`) asked whether
an argument's read was stamped `Known::NoneNull`, `null` where it's
`None`, which JS is given as it is, never left out. It's now told so per
argument, beside what's proved of its presence, `ArgProof { present,
null }` (`NoneForms::arg_proofs`, of `place_none`), the one record the
three call sites build. A differential of the stamp and MIR's answer over
check:fast and React's port found 7 differences, none of which changes a
call: 5 where MIR says `null` of an argument that may be `undefined` and
isn't proved there, already not left out; 2 where a `js::null()` is given
as it is, `null`, which the ladder never leaves out. No output moves.
Test `none-given`'s "a held js::null given to a binding's optional
argument is no argument" holds it; mutation `trailing-null-left-out`, new,
caught; `trailing-some-never-undefined-tested`, `trailing-some-present-
unasked` and `pointer-somes-unseen` follow the code, caught.

## Amendment: whether a closure gives back a binding's call is MIR's (2026-11-20, #2112)

`and_then`'s `None`, where what it's of is `None`, is `null` where its
function gives `null` for `None` and isn't a binding (ADR 0841, amended),
a binding's `T | null` read as JS's `labels?.item(0)` (ADR 0298, amended).
Of a closure, it asked its result's facts: `Known::Defined`, never
`undefined`, and not `Known::NoneNull`, which a binding's call was stamped.
Whether the closure gives back a binding's call is now MIR's: the call its
return place is written by (`closure_returned_callee`), a binding whose
result `none_given_by` says is `null` alone. `Defined` stays, a value
fact: a closure giving back a nullable field's `clone()` or a nullable
thread-local's `get()` is never `undefined` either. A differential of the
two over check:fast and React's port: none.

Ahead of `Known::Defined` carrying what `Known::NoneNull` said of storage
(the lead's decision, option (b)), each reader of it was read: it says
`!== undefined` only, never "not nullish". `may_be_undefined` asks of
`undefined` alone; drops' inlined reads (`drops/inlined.rs`) carry it to
a field's later reads, which a nullable place stores as they are; the
facts' roles list it; and this closure arm gives `null` only where its
function's `None` is `null`. None drops a `!= null` test or folds a `??`
on it.

Mutations `and-then-of-closure-of-nullable-none-undefined` and
`and-then-of-a-binding-nulled` follow the code, caught;
`closure-returned-callee-unknown`, new, caught.

## Amendment: a read whose `None` is `null` alone is never `undefined` (2026-11-20, #2112)

What a nullable place stores as it is, never `made ?? null`, asked
`Expr::may_be_undefined`, which took a read stamped `Known::NoneNull` for
never `undefined`. Most of `Storage::written`'s callers pass no answer of
MIR's, and those after lowering have none to ask: a write through a
handle's setter (`tidy::values::handles`), an applied arrow's
(`js::applied`), 93 such decisions over check:fast and React's port, each
a read of a place MIR says is `null` alone. The lead's decision (option
(b)): "never `undefined`" of such a read is true of the place everywhere,
a value fact, so `Known::Defined` carries it, written beside the `None`
by the one way a read says it, `Facts::with_none`; `may_be_undefined` no
longer reads `NoneNull`. Each reader of `Defined` says `!== undefined`
only (the amendment above). Making it an explicit answer where the write
is lowered is phase 3's.

React's port moves one line, at two places: `suspense_state.and_then(|s|
s.dehydrated)` of a nullable field only the crate makes, now `Defined`, is
`suspenseState != null ? suspenseState.dehydrated : null`, upstream's own
shape, where it was `suspenseState?.dehydrated`: `and_then`'s `None` of a
closure that gives what's never `undefined` is `null` (ADR 0841,
amended). Case A of ADR 0262: the same value's `None`, read through `==
null` only, the local stored nowhere, the lead's decision. The pin moves
to `a4b7a4d`; the corpus is unchanged.

Mutation `null-none-not-defined`, new, caught; `item-read-never-undefined`,
`defined-may-be-undefined`, `present-may-be-undefined` and
`alias-none-forgotten` follow the code, caught.
