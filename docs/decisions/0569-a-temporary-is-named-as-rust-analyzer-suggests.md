# 0569. A temporary is named as rust-analyzer suggests

Status: Accepted; amended (#444): a temporary a drop guard's `try` sets; amended (rd439): one a `const` takes apart; amended (#1105): not a closure's own of its name. Amends [0364](0364-mir-lowering.md), whose tidy named a
temporary after the field it reads or else `value`; extends
[0352](0352-reclaimed-names.md) and [0565](0565-readability-is-a-gate.md),
whose first number to bring down this is.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. A name changes
nothing a program can observe; this decides how the JS reads.

## Context

A temporary MIR makes and JS keeps, because something reads it more than
once, an `if let`'s scrutinee, a `RefCell`'s borrow, a value `println!`
shows, was named after the field it reads, `routes` of `tree.routes`, and
otherwise `value`. A function that keeps several kept them as `value`,
`value$1`, `value$2`, as many as there were, since JS gives one block's
`const`s each a name of its own. ADR 0565 measured what that costs: 951 of
every ten thousand lines of the committed JS named something given apart,
`x$1`, the most of the seven shapes by far, and 2,111 of those names were
`value`'s.

| What the suffixed names were, on main | Declarations |
|---|---|
| `value$N` | 2,111 |
| `tmp$N`, a `let` its branches set | 456 |
| `result$N` | 84 |
| `arg$N`, an argument held | 59 |
| others, a program's own names | 672 |

A Rust developer meets the same question where they extract a variable,
and their tooling answers it: rust-analyzer's `suggest_name`, which names
an expression after the method or function that made it, without a
conversion's prefix, `uppercase` of `to_uppercase()`, and skips the ones
that name nothing, `unwrap`, `clone`, `iter`, `new`; else after its type,
through `Option`, `Box` and `Rc`. That is the name the developer has seen
for that expression already.

## Decision

**A temporary JS keeps is named as rust-analyzer suggests: after the call
that made it, where the source calls it by that name; else after the
program's type it holds; and the first of those no other name in the
function has. Where both are taken, or neither says, it's `value`, `arg`
or `tmp` as before.**

- **The call's name**, camel case: `trailingZeros` of `a.trailing_zeros()`,
  `borrowMut` of `count.borrow_mut()`, `firstKeyValue`, `floor` of
  `x.floor()`; a method without `to_`, `as_` or `into_`, a function without
  `from_`, `with_` or `into_`; none of rust-analyzer's methods that name
  nothing, nor `unwrap_err` and `expect_err`, their `Err` side, nor its
  names that say nothing, `new`, `default`, `str`; nor a keyword, `mut` of
  `as_mut()`.
- **Only a call the source writes by its name** (`fn_span`'s text): not one
  a macro writes, `vec![..]`'s `new_uninit`, `format!`'s `must_use`, nor an
  operator's, `a[i]`'s `index_mut`, nor a closure's, `f(x)`'s `call_mut`.
- **A reference MIR takes to a temporary, or its copy**, `&_5` as
  `format_args!` takes what `println!` shows, has that temporary's names:
  `get` of `println!("{:?}", ages.get("bob"))`.
- **The program's type**, of the program or a library, lower camel case,
  `point` of an `Option<Point>`, `httpServer` of an `HTTPServer`, through a
  reference, an `Option`, a `Box`, an `Rc`. Not a `Result`'s `Ok` type, as
  rust-analyzer's is: JS holds a `Result` as `{ TAG, _0 }`, which
  `order.TAG` would misname. Not std's types, `u32` or `String`.
- **A name taken is passed over**, not given apart: `find(points, 1)`'s
  result, with `find` read and `point` another's, is `value`. A field's
  name, a JSX prop's and a trait object's `receiver` come first, as before.

`mir/hints.rs` answers what each temporary is suggested to be named
(`temporaries_names`), from its MIR; `tidy/bindings.rs`'s
`readable_temporaries` is still the one place a temporary is named.

## Why

- **It's the name a Rust developer already has for it.** The source says
  `a.trailing_zeros()`; its IDE extracts it as `trailing_zeros`; the JS
  keeps it as `trailingZeros`. No word of rust-js's own, and nothing to
  learn: guessable from the call.
- **It's what the gate asked for.** The renames' share falls from 951.2 to
  829.9 in ten thousand lines, 1,046 fewer lines with a name given apart,
  and no other shape's share rises; `test/corpus/temporary_names.rs` is
  the rule, held against native Rust.
- **A name taken falls back, not apart**: `point$1` beside a `point`
  reads worse than `value`, which says it's a value and nothing more.

## Alternatives

- **Name after the JS that's kept**, `$trailingZeros`'s `trailingZeros`:
  tried first. A runtime helper's name is the operation's, and half of
  them read as verbs, `const fromJson = $fromJson(..)`, `const borrowMut`;
  the Rust call is the name the reader knows. Rejected.
- **Only the type**: safe, and it moved the share 1.8%. The calls are what
  a temporary mostly holds. Kept as the fallback.
- **Reuse `value` where the one before it is dead**: a block's `const`s
  can't share a name, and a `let` reassigned reads as state that isn't
  there. Rejected.
- **Name a `Result` after its `Ok` type**, as rust-analyzer does: `order.
  TAG === "Ok"` and `order._0` misread the object. Rejected; a `Result` is
  named after its call, `checkedDiv`, `parseNumbers`.

## Consequences

- 281 files of the corpus and the snapshots read the new
  names: `const floor = Math.floor(x)`, `const next = $next(entries)`,
  `const point = find(points, 3)`, where each was `value`, `value$1`.
- **Some names are verbs**, `next`, `get`, `borrowMut`, `peek`, as
  rust-analyzer's are; they say what made the value.
- **What a closure applied at once holds is still `value`**: `println!`'s
  `{:?}` of a call's result whose closure tidy applies (`applied_values`),
  `$checked(ages.size - 4, ..)`. Its value is no MIR temporary by then;
  naming it is the next step for the share.
- **A temporary in code a macro of the program writes**, `macro_rules!`'s,
  has no name the source writes, and keeps `value`.

## What holds it

- `test/corpus/temporary_names.rs`: names after the method, through a
  reference `println!` takes, after the type, and `value` where both are
  taken, against native Rust.
- `docs/readability.txt`, reblessed: renames at 829.9.
- `test/compiler.test.ts`'s and `test/lowering.test.ts`'s lines that read a
  temporary's name; a unit test of a type's name as a variable's.
- Mutations: the suggestions unused, a name taken given apart, a call the
  source doesn't write named, a keyword named, a method that names nothing
  named, a reference's call unnamed, the type unused, a `Result` named
  after its `Ok`; and the props', arguments' and temporaries' own, kept.

## Amendment: a temporary a drop guard's `try` sets (#444)

A pattern's name takes the temporary it binds where MIR made it
`const _3 = e`; inside a drop guard's scope, whose each call is its own
`try` till the scopes are one, MIR's temporary is `let _8; try { _8 = e;
} catch ..`, and `if let Some(descriptor) = descriptor_of(name) && ..`
was `const descriptorOf = ..`, named for its call once the alias was
gone. A temporary set in the `try` it's first written in, there alone and
never again, reads one value after it, as a `const`'s, so the pattern's
name takes it too: `const descriptor = globalThis.descriptorOf(name)`,
as without the guard (React's describeNativeComponentFrame). So is a
`for` loop's item where what's live drops as it unwinds, `let word; try {
word = $next(iter); } ..`, no `const word = tmp;` after it: seventeen of
the corpus's cases and `@rust-js/std`'s `zip` fold read so now. Not where
another variable of the name is declared, which the `let`'s wider scope
would meet, `max$1`. `docs/readability.txt`, blessed with it, counts the
corpus's `position_index_of` (ADR 1040) too, whose iterator stepped before
its `position` is the one `$iter` more, 85.1 to 85.2 in 10,000 lines.

Held by `test/lowering-drop-guards.test.ts`'s "a field read of what JS
made in a guard's scope is in its one try", tidy's unit test
`set_once_in_a_try` (set again after, or read in the `try` first, keeps
the temporary), and mutation `temporary-set-in-try-unnamed`.

## Amendment: a temporary a `const` takes apart (rd439)

React's Flight client's initializeChunkIfUnblocked leaked MIR's name:
`const { value: _12 } = $borrow($fieldCell(handler, 'chunk', true)); if
(_12 == null) return; const chunk = _12;`, of `let Some(chunk) =
handler.chunk.borrow().clone() else { return; };` of a cell tracked at run
time. `named_temporaries` names a temporary after the variable the program
gives it, `const _3 = e; .. const current = _3;`, but only a `const` or a
`let` of it, and here a `const` takes it apart, one field read out of the
guard (`destructured`). **A `const` that takes one name apart, a
temporary, is named so too**: `const { value: chunk } =
$borrow($fieldCell(handler, "chunk", true)); if (chunk == null) ..`. Case
([0262](0262-when-rust-and-js-disagree.md)): none, a name. Upstream's
`const chunk = handler.chunk;` is the borrow helpers' to give (rd428).
Held by the corpus's `counted_guard_value_named`, against native Rust, and
the mutation `destructured-temporary-unnamed` (`tidy/bindings.rs`).

## Amendment: not after a positional field (run 40)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, a name.

A temporary is named after the field it reads, `routes` of
`tree.routes`; of a positional field, a tuple variant's, that's `_0`:
`match bump(&mut i)?.clone() { .. }`'s subject was `const _0 =
result._0;`, read as `_0.TAG` and `_0._0`, a name that is itself MIR's
form of a temporary's (`is_temporary`), which says nothing of what it
holds. **A positional field names no temporary**: the next name is
taken, what rust-analyzer suggests of its type, `const tok = result._0;`,
else `value`. A field the program names keeps naming it.

Held by the corpus's `match_clone_nested_results` (`tok`) and
`option_combinator_drops` (`value`), against native Rust, and the mutation
`temporary-named-positional` (`tidy/bindings.rs`).

## Amendment: a cell's box, after what it holds (rd413, 2026-11-04)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, a name.

A struct literal's `debug_stack: Cell::new(Some(js::error::new(..)))`,
held apart from a later field whose JS is made first, React's
`debugTask: console.createTask(..)`, was `const value = new Error(..)`:
the field's cell is its box, `{ value: x }`, as it's made, and an object
held apart is each of its fields held, named after its key, the box's
`value`. The same field without its `Cell` was `const error`. **What a
cell's box holds is held apart as the temporary that holds the cell**
(`mir/temporaries.rs`'s `cell_box_value`), and that temporary is named
after what it holds, a `Cell` and a `RefCell` of the value itself seen
through as an `Option` is (`hints.rs`'s `held_type_name`): `const error
= new Error(..)`.

Held by the corpus's `temporary_names` (a `Cell<Option<Point>>` held
apart from a field made of a closure's `map`, `const point`), against
native Rust, and the mutations `cell-box-captured-as-value`
(`mir/temporaries.rs`) and `temporary-cell-unnamed` (`mir/hints.rs`).

## Amendment: a value read in place is named after its call where it's spilled (#769)

#723 (ADR 0800, amended) keeps what's made between calls that unwind to
one cleanup in place, where each call's value had been a temporary
written at its call, `const _16 = $next(r)`, which this ADR names after
its call, `next`. Read in place, such a value was spilled by what reads
it twice, under that reader's name: an `Option`'s subject `option`, an
`Option::map`'s its closure's parameter, `l` or `d`. So `const next =
$next(r)` read `const l = $next(r)`, and `nth`, `nthBack`, `last`,
`find` read `option` or `d`.

The value carries the name its temporary is given, `js::Expr::held_as`,
set where a pending temporary is taken to be read in place, of what a
call the source writes by its name made (`named_by_their_call`, the same
`call_name` and `variable_name` as `temporaries_names`, through one
`calls_named_in_source`, `mir/hints/calls.rs`). `FnCx::spill_subject`, beside it in `mir/temporaries.rs`, the spill of a subject read
more than once, an `Option`'s and `Option::map`'s, alone reads it; a
`Result`'s stays `result`, which its call names worse, `catchUnwind`,
`parse`. A name the program's own has is left to the reader's, never
`make$1` beside its `make`; another of a call it named so is `nth$1`.
`HeldAs` is never part of what an expression is: two alike but for it
are equal, and every expression made of new parts has none, so no shape
decision sees it. And a closure made between calls one `try` covers, set
once there, `let map; try { .. map = (d) => { .. }; .. }`, is written
where its `let` is, `function map(d)`, as it was where the calls unwound
apart (tidy's `finish/closures out of tries`); never one that reads a name
declared in that `try`, which isn't there before it.

93 snapshots read differently, every one a rename, each checked name by
name against the commit before #723's: as good or better there, `d` →
`last`, `option` → `nth`, `getMut`, `splitOnce`, `l` → `next`. Where an
earlier subject now takes a call's name, a temporary named later falls
back, `next` → `value` in `nested_item_drops.js` and `std_rev.js`,
`upgrade` → `tmp$3` in `rc_counts.js`, as before #723: **the user's
decision is to accept these as they are**, with no numbered call name,
`next$1`, tried first. Mutations: `subject-not-named-by-its-call`,
`subject-name-taken-from-the-program`, `in-place-value-unnamed`,
`result-subject-named-by-its-call`, `closures-left-in-tries`.

## Amended (2026-11-05, #784): a closure's parameter before a getter's name, and an `Option`'s own method names nothing

#769 named a value read in place after the call that made it. Two kinds
of call read worse than the pin's, as neither is a call the JS writes:

- **A binding read as a property, `get G` or `prop x`, names it only
  where no closure's parameter does**: `g(REACT_SHARED_INTERNALS).map(
  |on_start_gesture_transition_finish| ..)` is `ReactSharedInternals.G`
  in JS, which calls nothing, so its subject is the parameter that binds
  it, `const onStartGestureTransitionFinish = ReactSharedInternals.G`, as
  upstream writes it, not `const g`; where none does, the property's own
  name, `const count = source.count` of `count(source).filter(..)`. A
  call names what it makes over a closure's parameter, `const next =
  $next(r)` of `r.next().map(|l| ..)`, #769's decision.
- **An `Option`'s or a `Result`'s own method names nothing**: `as_deref()`,
  `filter()`, `map_or_else()` give back the option they had, so
  `props.name.as_deref().filter(..)`'s subject is `option` again, not
  `deref`, and `get_component_name_from_type(c).filter(..)`'s not
  `filter`. Upstream writes no temporary there, `getComponentNameFromType(Component)
  || Component`, which fusing the combinators would give: a P3 of its own.

`named_by_their_call` gives each name, marking a property's read
(`js::Held::read`), and `spill_subject` drops a read's name where its
caller's `base` is a closure's parameter. The user's decision, through
m6-react. Held by corpus `option_subject_named_by_its_binding`, against
native Rust in both builds and by its snapshot, and `lowering-moves`' "a
getter's Option a closure binds is named by its parameter" and "a
getter's Option a combinator reads more than once is read once".
Mutations `option-method-names-its-subject` and
`binding-read-names-its-subject` (`mir/hints/calls.ts`).

## Amendment: not a closure's own of its name (2026-11-09, #1105)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change (in an
intermediate stage only). `named temporaries`, which names a temporary for
the `const` that takes it, `const high = _2;`, renamed each read of `_2`
after it, a closure's too: where the closure takes a parameter of that
name, `(_2) => { const a = _2[0]; .. }`, its tuple's, its reads became
main's `high`, `high[0]` of a number, and the JS after tidy's `in place`
stage threw `Cannot convert NaN to a BigInt` where Rust prints (the
nightly's program stages, seed 12459702). A later pass took the
parameter apart in place, so the finished JS was right. **The renaming is
made where the name is the one the list means: a closure that takes or
declares its own is set apart while it's made** (`js::outside_shadows`,
as `readable_temporaries`' renaming already is), for the temporary and for
each name it takes.

Held by the corpus case `closure_param_shadows_a_named_temporary.rs`, which
the push's `tidy-stages` runs at each stage end, throwing the same
`TypeError` without it, and mutation
`named-temporary-renamed-in-a-closure-of-its-own` (`tidy/bindings.ts`).

## Amendment: no name of the module's beside an item written in the body

A function or a `const` written in a body (ADRs 0308, 0031) is placed
once tidy is done and filled once every body is lowered, so what it reads
is nowhere in the body tidy names. `readable_temporaries` took the first
suggested name the body didn't read or declare, `note` of a `match` one
of whose arms calls `note`, and `const note = id(-1)` hid the module's
function `note` from `function h0(p1) { return note(0, p1); }` written
beside it: `note is not a function` where Rust prints (the nightly's
fuzz and program stages, generated program 12461955; P0, ADR 0262's
third case). **Of a body with an item written in it, a function's or a
closure's, a temporary takes no name of the module's**, its functions,
imports and globals, which a local the program names never takes either
(`pipeline.rs`' `names`): `Hints::items_read`, by `items_written_in`
(`mir/closures.rs`), the one answer of which items a body writes, which
placing them asks too. A body with none keeps the names it gives, as what
it reads is all that's read in it: `entry`, `node` and `pop` of the
corpus, each a name of its module's no item there reads, are unchanged.

Held by the corpus case `temporaries_beside_items_written_in_the_body.rs`
(a function's body, a closure's and a block's), and mutations
`temporary-takes-a-name-items-read` (`tidy/bindings.ts`) and
`items-written-in-a-closure-unseen` (`mir/closures.ts`).

## Amendment (2026-11-12, #890 step 1): one owner of the rule

The rule was spelled in three files: `mir/hints/names.rs` (the call's name,
the held type's name, the casing), `mir/hints/calls.rs` (the call's name
again, for a value read in place, `held_as`) and tidy's
`readable_temporaries` (the fallback words and the pick). It is now
`temporary_names.rs`'s: `call_name` and `call_suggestion`, `held_type_name`,
`suggested` (a suggestion as a JS variable), `pick` (the first no other name
has, else the role's word given apart) and `Role`, the closed set of a
temporary's role words, `value`, `arg`, `tmp`, `receiver`, `children`. Each
of the three asks it; an architecture test holds that none spells the rule
itself. No output changes. #890's next steps give lowering's `spill` a
`Role` in place of its word, carry what made a value on `js::Expr` as
`made_by` in place of `held_as` (the coordinator's decision), and name
spilled temporaries where MIR's are, by this rule.
