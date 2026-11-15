# 0620. An `Option` a test says is there is written as it is

Status: Accepted. Extends [0578](0578-a-nullable-result-where-its-written.md)
and [0564](0564-a-std-body-is-tidied-as-the-apps-are.md)'s decided tests.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: what's written, not
what it does. `x ?? d` is `x` wherever `x` isn't `null` or `undefined`.

## Context

React's `getIteratorFn` returns the iterator method it finds, or `null`:

```rust
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn get_iterator_fn(maybe_iterable: Option<&'static Unknown>) -> Option<&'static Unknown> {
    // ..
    if js::type_of(maybe_iterator) == "function" {
        return maybe_iterator;
    }
    None
}
```

A `#[rust_js::nullable]` result of another `Option` is `null` for
`undefined` where it's written (ADR 0578), so rust-js wrote `return
maybeIterator ?? null;` under `if (typeof maybeIterator === 'function')`,
where upstream returns `maybeIterator`: what the test says is a function
is never `null` or `undefined`.

## Decision

**Where a test is known to hold, or to fail, an `x ?? d` of a variable
it says is there is `x`**: `typeof x === "function"`, of any kind but
`"object"` and `"undefined"`, a test of `x` against a constant, or `x !=
null`, holding; `x == null` failing, after `if (x == null) { return null;
}`. It's what `decided_tests` knows already, the test of an `if` that
holds through its `then` and fails through its `else` and what follows a
branch that leaves (ADR 0564), up to what may change what it reads; now
it decides a `??` there too (`present_in`), of a variable nothing
assigns.

```js
export function callable(value) {
  if (typeof value === "function") {
    return value;
  }
  return null;
}
```

`implies` answers whether one test says another, and gains its one
missing case, `x != null` saying `x != null`.

## Why

- **A value's JS is what it is where it's read**: `?? null` says the
  value may be `undefined`, which the test it's under says it can't be.
- **One place knows what a test decides**, `decided_tests`: a `??` it
  decides is the same knowledge as an `if` it decides.

## Alternatives

- **Not write `?? null` where the result is written** (`nullable_written`):
  lowering doesn't know the tests a value is under. Rejected for asking
  of a value what only the code around it knows.
- **Write the Rust another way**, `maybe_iterator.filter(..)`: what's
  natural is the test upstream writes, and rust-js is what changes.

## Consequences

- A `??` of a variable inside a closure isn't one this decides: none is
  there yet when it runs, and the test of `shadowed` holds that a
  closure's own `value ?? 7` stays.
- Only a variable's `??`: one of a field, `a.b ?? d`, stays.

## What holds it

- `test/lowering.test.ts`, "an Option a test says is there is written as
  it is": `callable`, `logged` after a failed `== null`, and a closure's
  `(value) => value ?? 7`, run in Node; and "a nullable result is written
  where it's made", whose `if (flag && first != null) { return first; }`
  had `?? null`.
- Mutations `present-coalesce-kept`, `present-coalesce-failed-test-unread`.

## Amendment: what's read of a nullable field is never `undefined`

React's `findFirstSuspended` keeps a fiber's `return` in a variable and
stores it in another's, and ReactFiberRoot takes a root's transition
types:

```js
const returnFiber = node.return;     // upstream: the same, then
sibling.return = returnFiber ?? null; //   sibling.return = node.return
const old = root.transitionTypes;    // upstream: const claimed = ..
root.transitionTypes = null;
return old ?? null;                  //   return claimed
```

A nullable place stores `null` for `None` (ADRs 0772, 0841), so what's
read of one is `null` or the value. **A read of a `#[rust_js::nullable]`
field there where it's made is never `undefined` (`Known::Defined`), and a
`const` of it, `x ?? null`, is `x`**: `representation::project` marks the
read, `fixed_place_of` keeps what's known of a place it makes once, and
tidy's `nulled_as_they_are` writes `x` of a `const x` its body declares
once, of what `may_be_undefined` says never is. Not of an optional one
(ADR 0810), absent until it's set, `undefined` read: `banner ?? null`
still. Case A, as above: `x ?? null` is `x` wherever `x` isn't
`undefined`; a field JS made against its declaration, `T | null`, is the
JS's to keep.

- `test/lowering-nullable.test.ts`, "a nullable field's value a variable
  holds is stored as it is", run, an optional one's too.
- Mutations `nullable-field-read-may-be-undefined`,
  `optional-nullable-read-defined` (`lower/representation.ts`),
  `fixed-place-facts-dropped` (`lower/mir/assign.ts`),
  `nulled-kept-coalesced` (`lower/mir/tidy/values.ts`).

## Amendment: an `unwrap()` of what a test says is there is its value

React's `findFirstSuspended` takes a fiber's sibling once its loop's test
says it's there, upstream's `node.sibling` read again:

```rust
while node.sibling.borrow().is_none() { .. }
let sibling = node.sibling.borrow().clone().unwrap();
```

Its port's JS was `$unwrap(node.sibling)`, after `if (node.sibling ==
null) { .. }`, whose branch leaves: the `unwrap()` never panics there.

**Where a test is known to hold or to fail (`decided_tests`), a
`$unwrap(p)` of a `p` it says is there, `p != null`, that reads the same,
a variable or a field, is `p`**; of a statement that may change what the
test reads, only the value it reads first, `const sibling =
$unwrap(node.sibling)`. A test through a field is decided where the
variables it reads change elsewhere too, as anything that does something
disturbs it (`disturbs_any`): a `node` the loop assigns later is
`node.sibling` still until then. Case N: the value is the same, and the
panic can't happen.

- Three corpus cases decide a test through a field's tag they couldn't
  before, `if (tmp.TAG === "Ok")` after `tmp.TAG !== "Err"`, their drops
  each once; `loop_assigned_tests`' `out.push(top)`.
- `test/lowering-nullable.test.ts`, "an unwrap of a field a test says is
  there is the field", run, a call between keeping its `$unwrap`.
- Mutations `field-test-undecided-where-its-variable-changes`,
  `unwrap-of-tested-kept`, `unwrap-past-a-call`
  (`lower/mir/tidy/branches.ts`).

## Amendment: a test against a `None` given says it isn't there

A nullable function's `if js::strict_equals(ref_prop, None) { return
ref_prop; }` printed `if (refProp === undefined) { return refProp; }`,
giving `undefined` where the function gives its `None` as `null`: a wrong
answer, a JS caller's `=== null` failing. Decided tests read `x ===
lit`, of a constant not `null` or `undefined`, as saying `x != null`, and
the `None` given to `strict_equals` is a `GivenUndefined` (ADR 0330),
which wasn't counted as `undefined`. **A test of `x` against a `None`
given says nothing of its being there** (`branches.rs`'s `implies`): its
`refProp ?? null` stays. Case R: results changed, now Rust's. Held by
`test/lowering-nullable.test.ts`, "a nullable result tested equal to a
None given is null", run in JS, and the mutation
`implies-given-none-a-literal` (`tidy/branches.ts`).

## Amendment: a `?? null` of a value its test shows defined is the value

Its port's `return if !js::strict_equals(ref_prop, None) { ref_prop } else
{ None };` was `refProp !== undefined ? refProp ?? null : null`, where
React writes `refProp !== undefined ? refProp : null`: a nullable result's
`x ?? null` (`Storage::written`) of a `const` the body declares once, in
a conditional's branch or an `if`'s where its test is `x !== undefined` or
`x != null`, is `x`, which isn't `undefined` there
(`values.rs`'s `nulled_as_they_are`, `shown_defined`). Case N: `x ?? null`
is `x` where `x` isn't `undefined`. Held by
`test/lowering-written-branches.test.ts`, "an if of returns the program
wrote stays an if", `test/lowering-nullable.test.ts`, "a nullable result
tested equal to a None given is null", and the mutations
`tested-defined-still-nulled`, `tested-defined-if-still-nulled` and
`tested-defined-where-it-isnt` (`tidy/values.ts`).

## Amendment: what a nullable function gave is never `undefined`, held or not (2026-10-26)

React's unsafe_markUpdateLaneFromFiberToRoot holds what it finds while more
runs:

```rust
let root = get_root_for_updated_fiber(source_fiber);
mark_update_lane_from_fiber_to_root(source_fiber, None, lane);
root
```

`get_root_for_updated_fiber` is `#[rust_js::nullable]`, and so is the
function returning `root`. A call of a nullable function written straight
to its result, or to a nullable place, was as it is (ADRs 0638, 0841); held
in a variable first, it was `return root ?? null;`, as a variable may be
`undefined`.

**What a call of a function that gives `null` for `None` gives is never
`undefined`, wherever it's kept** (`Known::Defined`, `calls::given_by`):
`const root = lookup(id);` is a `const` of what's never `undefined`, so
`nulled_as_they_are` returns it, and a nullable field stores it, as it is:
`return root;`, `holder.root = root;`. Not a trait method's, whose impl
says nothing of it (ADR 0841, amended).

- Case A: `null ?? null` is `null`, and a value is itself.
- Held by `test/lowering-nullable.test.ts`, "a variable of what a nullable
  function gave is returned and stored as it is", run in JS, and the
  mutation `nullable-call-kept-undefined` (`lower/mir/structure.ts`).

## Amended (2026-11-01): what a `return` reads first (rd443)

React's resolveCurrentlyRenderingComponent tests its module's variable,
throws where it's `null`, and returns it. The port's `CURRENT.get().
unwrap()` after the `is_none()` test's panic kept its `$unwrap`: a
module variable isn't a variable alone, so the `return` that unwraps it
disturbs the test, and a `return` wasn't one of the statements whose first
read is looked at. **A `return`'s value is what it reads before what it
does, as a `const`'s is: `return $unwrap(p)` where the test says `p` is
there is `return p`**. Where something runs between the test and the
`return` that may set the variable again, a call, the `$unwrap` stays,
as Rust's `unwrap()` is checked there. Case
([0262](0262-when-rust-and-js-disagree.md)): N. Held by
`test/lowering-early-exits.test.ts` ("a module variable returned after its
test panicked is the variable") and the mutation `present-return-unseen`.

## Amendment: what a test of `js::truthy` proved there (#1902)

Case ([0262](0262-when-rust-and-js-disagree.md)): A, as above. Where
`if (x)` held, `x` is neither `null` nor `undefined`, so `x ?? null` and
`x ?? undefined` are `x`.

A faithful SDK port's `composedClosest` tests an optional call's result
JS's way, `if js::truthy(found) { return unsafe { js::cast(found.map(js::unknown)) }; }`,
upstream's `if (found) { return found; }`, and its nullable result was
`return found ?? null;` there. The decided tests above read the JS's
`!= null` and `typeof`, and the test MIR makes of a truthy test is a
call's `bool`.

**The nulled owner proves it from MIR (`proved_present`, mir/nulled.rs):
an `Option` local is there where every path to the block takes the true
edge of a `switchInt` of `js::truthy(local)`'s result, the binding known
by its `DefId` (`is_js_truthy`), through the copies MIR makes of it for
the call; of a parameter nothing writes, or of a local with one whole
write, never lent as a `&mut`.** A read of it there is `Known::Present`
(`mir_operand`), the fact `Storage::written` and an exit's `??
undefined` already read, so `js::cast` and `map(js::unknown)` of it keep
it. A value written again, read where the test failed or where another
value's test held, or under `!js::truthy(x)`, keeps its `??`.

Held by `test/lowering-nullable-truthy.test.ts` (the SDK's
`composedClosest` with a parameter's like it, run with the method there,
gone, and giving `undefined` and `null`; a value written again, an
`else`, another's test and a negated test, each keeping `?? null`; and
a plain `Option`'s exit of a binding's `T | null`, given as it is where
proved, `?? undefined` where not), and mutations
`proved-present-of-any-test`, `proved-present-written-again`,
`proved-present-copies-unfollowed` (`lower/mir/nulled.ts`),
`proved-present-unread` (`lower/mir/operands.ts`) and
`js-truthy-unrecognized` (`lower/recognition/js_items.ts`). The guide's
nullable example shows it (semantics.md).

## Amendment: MIR's tests of an `Option` own it, and what they prove is of the read alone (2026-11-15, #1933, #1942)

Two owners answered whether a test says an `Option` is there: the
nulled owner from MIR (`proved_present`, #1902), of `js::truthy` alone,
and tidy's decided tests and `nulled_as_they_are`, from the printed JS.
Counted (decisions `tidy-present-in` and `tidy-nulled-as-they-are`,
every corpus case as a program and a library, React's and
alien-signals' ports), tidy's answers fired in 6 corpus cases and 35
of React's hunks; none was of code with no MIR. Step one of moving them
to MIR, of a local's tests:

**`proved_present` reads each test MIR makes of an `Option` local:
`js::truthy(x)` and `x.is_some()`, their true edge; `x.is_none()`, its
false one; `js::type_of(x) == "function"`, of a kind neither `None`'s
`"undefined"` nor `null`'s `"object"` is, its text a constant through
the locals and the promoted reference MIR holds it in; and a `match` or
`if let` of it, its `discriminant`'s `Some` edge.** Each where every
path to the read takes that edge, entered from the test alone, a
`match`'s false edges, which no run takes, aside; of a parameter
nothing writes or a local with one whole write, never lent `&mut`, as
before. The argument is followed through copies, the reference a test
takes, `&x`, and its reborrows, `&(*r)`.

**A read so proved is `Known::Proved`, not `Present`.** It's never
`undefined` to a nullable place's storage (`may_be_undefined`), given
as it is at a plain exit (`never_null`), and its own value to an
`unwrap()` (`$unwrap(currentProps)` under `js::truthy(current_props)
&& ..` is `currentProps`). Present is what a value is by how it was
made, which tidy's passes take for true wherever they move it: given
to a proved read, it went where its test doesn't hold. React's `match
deps { None => js::null(), deps => deps }` printed `const nextDeps =
deps;`, tidy's `conditional` making `deps == null ? null : deps` the
`deps ?? null` that a `Present` `deps` drops, so `undefined` for
`None`; and on main #1902's own, `let kept = if js::truthy(x) { x } else
{ x }`, two arms alike made one, returned `undefined` (#1942, a wrong
answer only JS sees: `js::truthy` is the js crate's, which no native
program runs). **Tidy forgets `Proved` as it begins
(`Expr::forget_proved`)**, and a contract checked between its stages
holds that nothing's left (`NOTHING_PROVED`); `.proved()` is made only
of a variable's read in `mir_operand`. Lowering moves no proved read
past its test: the one read lowering takes elsewhere is a temporary of
one write, inlined where it's read, which that write dominates. A proved
read is no other knowledge of it (`known_as`): `kind.object()`'s `t ?
v : undefined` of one is still tested by `t` (ADR 0910).

Case of [0262](0262-when-rust-and-js-disagree.md): none, each value the
same. Tidy's rules stay until each kind they answer is MIR's (#1933's
field paths and the `None` a read already is), then go. Held by
`test/lowering-nullable-tests.test.ts` (each test beside `&& flag`, no
tidy answer counted; `is_none()`'s true edge, `"object"` and a value
written after, each keeping `?? null`; a proved read moved by tidy, of a
program and a library; an object made of one) and
`test/architecture-nones.test.ts`; corpus case
`catch_unwind_value_read_first.rs`, `wordAt(text, 1) ?? "-"`; mutations
`proved-present-*` and `proved-kept-into-tidy` (`lower/mir/nulled.ts`,
`lower/mir/tidy.ts`), `unwrap-of-a-proved-option-checked`
(`lower/std_types/option.ts`), `proved-read-may-be-undefined` and
`proved-read-known-as-nothing-more` (`js/make.ts`) and
`proved-read-given-undefined` (`lower/mir/nulled/one_none.ts`).

## Amendment: a nullable `RefCell` field's clone is `null` or a value, MIR's to say (2026-11-15, #1933 step 1(c), first part)

Tidy's `nulled_as_they_are` drops `x ?? null` of a `const` it reads as
never `undefined`: 104 of its answers in React's port, 3 in the corpus,
each a value read from a nullable field and stored in another, React's
`let child = current.child.borrow().clone();
work_in_progress.child.replace(child);`. The nulled owner didn't see
through the guard: **what a `RefCell`'s guard derefs to, of a guard
`borrow()` gave, is what the field it borrowed holds**, `null` or a
value of a field that stores `null` for `None`; and **an `Option<Rc<T>>`'s
or `Option<Arc<T>>`'s clone is the value it cloned**, as a reference's,
a number's or a string's was. Stored, it's as it is. What's left of
tidy's answers of it, 44 in React, are other reads (a `Cell`'s
`replace`, a `get()` of a field set before, a binding's field), its
next parts.

Case of [0262](0262-when-rust-and-js-disagree.md): none. React's port
reads better where tidy didn't see it, `initializingHandler =
handler.parent;`, `parent: suspendCallSiteStack.parent,` (9 hunks), and
a test of such a value is the strict one its `None` allows, `fiber !==
null` (ADR 0298, amended). Held by test/lowering-nullable-tests.test.ts's
"a nullable RefCell field's clone" (no tidy answer counted; a field that
isn't nullable keeping `?? null`; run), mutations
`refcell-guard-deref-unknown` and `rc-clone-not-itself`.

## Amendment: what a nullable cell field held, read whole, is the representation's answer (2026-11-15, #1933 step 1(c), second part)

Of tidy's 44 answers left in React's port, most were a cell field's old
value restored, React Fizz's `let prev_key_path =
task.key_path.replace(Some(..)); .. task.key_path.replace(prev_key_path);`
(`replace` and `take()` of a `RefCell` or a `Cell`), a `Cell` field's
`get()`, `let parent_preamble = task.blocked_preamble.get();`, and a
`let mut` written twice, `let mut active = js::null(); if .. { active =
get_closest_instance_from_node(..); }`. The nulled owner now says each:

- **What a struct's `Cell` or `RefCell` field holds, read whole**, its
  `get()`, `take()` or `replace(v)`'s old value, or its `borrow()` guard's
  deref, is one answer, `nulled::cell_field_held`: `null` or a value where
  a read of the field is never `undefined`, which is the representation's
  answer, `cell_field_read_defined` (a nullable field, `NoneForm::Null`,
  of a struct no getter answers the fields of), or where each constructor
  of a struct JS makes sets it first (rd628); maybe absent of an optional
  one (ADR 0810). The same question the read's `Known::Defined` answers
  (`project`), so the two can't disagree. The first part's guard deref,
  which took any nullable field, takes this answer too: a field of a struct
  only JS makes, whose getter may answer `undefined`, keeps its `?? null`.
- **A cleanup's write of a local isn't counted beside the body's**: as an
  old value's drop unwinds, MIR writes the new value in the cleanup too,
  the same write, which the body's count of writes leaves out; counted, no
  variable written twice was ever found `null` or a value.

Tidy's answers of never-`undefined` left: React 18 (10 sites: a `&mut`
of an `Option` matched, a `Some` written to a `Vec`'s item through its
guard, thread-locals read through `with`'s closures, a captured local
restored in a closure, an untagged enum's constructor mapped, a binding's
getter, `ReactSharedInternals.H`, the brief's binding-result trust); the
corpus none.

Case of [0262](0262-when-rust-and-js-disagree.md): none. React's port
reads better, 7 hunks: `debugChunk._debugChunk = previousChunk`,
`thenableState = prevThenableState`, and `const parent =
task.componentStack;`, `const prevThenableState = task.thenableState;`
(×3) where a `const old` was read into each, as upstream writes them.
Held by test/lowering-nullable-tests.test.ts's "a nullable cell field's
old value and get()" (no tidy answer counted; a struct only JS makes and
a `let mut` of `None` keeping `?? null`; a stack cursor's read in a
closure, the representation's; run) and the corpus case
`nullable_cell_fields_saved_and_restored` (native, both builds, as a
library too, its metamorphic rewrites); mutations `cell-field-held-unknown`,
`cleanup-write-counted` and `cell-field-getter-read-defined` new,
`optional-cell-get-unknown`, `constructed-get-unknown` and
`nullable-field-read-may-be-undefined` retargeted.

## Amendment: a `RefCell` field's read where a test of the same field proved it, MIR's to say (2026-11-15, #1933 step 1(b))

A test of a struct's `RefCell` field and a later read of it are two
borrows, two MIR temporaries, so `proved_present`, of a local's own tests,
never said `while node.sibling.borrow().is_none() { .. }` then
`node.sibling.borrow().clone().unwrap()` is there; tidy's `present_in`
read it from the printed JS, `while (node.sibling == null)` then
`$unwrap(node.sibling)`. The nulled owner now proves it,
`nulled::field_reads::field_read_proved`, the lead's approved design:

- **The read**: a local one clone writes of what the field's std guard
  derefs to, `borrow()`'s `Ref`.
- **The test**: `is_some()`'s true edge or `is_none()`'s false one of the
  same field, the same path from the same variable (a field, a shared
  reference's deref, an `Rc`'s or an `Arc`'s `Deref` on the way), whose
  edge every path to the read takes, entered from its switch alone.
- **Nothing between**: on each way from the test's call to the read, not
  back through the test, which reads the field anew, only that field's
  own reads run: its `borrow()`, the std guard's deref and drop, an `Rc`'s
  or an `Arc`'s deref, a test of an `Option`, a clone of an `Option` of a
  reference, a number, a string, an `Rc` or an `Arc` (std's, no code of
  the crate's), and assignments of locals; never the variable the path
  begins at, nor a write through a place.
- **Of the program's own data**: not a struct JS gives or makes, whose
  getter may answer each read anew (`fields_read_once`, rd204, #1908).

The read is `Known::Proved`, which tidy forgets as it begins, as a local's
is; read where it's made, `$unwrap(node.sibling)`'s temporary, too.
Corpus: tidy's presence answers 0 (the field-path case `while_exit_continues`
the MIR's now).

**Tidy's own reading of a field path goes** (#1950, P0): its `present_in`
took `$unwrap(node.sibling)` after `node.sibling != null` as the field, of
any struct, where one JS gives, whose getter answers `Some` and then
`None`, threw a `TypeError` in place of Rust's panic. `present_in`'s
unwrap now takes a variable alone, a module's too, never a read through a
property (`Expr::reads_a_field`); ADR 0957's rule likewise. React's 8
`$unwrap(node.sibling)` sites are of `Fiber`, a struct its exported
functions take, which the owner says JS may give, so MIR doesn't prove
them: `$unwrap` there, the lead's decision on Tu's behalf, correctness
first; until Tu decided that a struct only the crate constructs is plain
data (ADR 0262's difference 60, ADR 1022, amended), which gives them their
plain reads back.

Case of [0262](0262-when-rust-and-js-disagree.md): results change of a
struct JS gives, Rust's kept (#1950); of the program's own data, none.
Held by test/lowering-nullable-tests.test.ts's "a RefCell field tested and
read again" (the loop's exit, an `if`'s, no tidy answer counted; a call
between, the variable set to another and a clone that may run the crate's
code keeping `$unwrap`; a struct JS gives keeping it, its getter answering
`Some`, then `None`, panicking as Rust does; run) and the corpus
case `field_read_tested_then_unwrapped` (native, both builds: a user's
`Clone` and a call between that empty the field, each panicking as Rust
does); mutations in lower/mir/nulled/field_reads.ts, and
`field-path-unwrap-tidied` (lower/mir/tidy/decided.ts) new.

## Amendment: a thread-local's `with(f)` gives what `f` returns (2026-11-16, #1933 step 4 (C))

The nulled owner says what `KEY.with(f)` gives is `null` or a value where
each of `f`'s returns is (`returns_null_or_value`, rd632): React's
popProvider's `RENDERER_CURSOR_DEV.with(|cursor| *cursor.current.borrow())`
of a nullable cursor's field, stored in a context's nullable field after
the cursor is popped, `context._currentRenderer = currentRenderer`, which
tidy's never-`undefined` answered. React's port unchanged; tidy's answers
of it 14 to 12. Held by test/lowering-nullable-tests.test.ts's "a
thread-local's with(f)" (no tidy answer counted) and the corpus case
`thread_local_with_gives_what_it_returns` (native, both builds, as a
library, metamorphic); mutation `local-with-unknown`.

## Amendment: a variable written more than once is proved where nothing writes it after its test (2026-11-16, #1933 step 4 (A); #1952)

`proved_present` took a local of one whole write alone. It now takes one
written more than once, each write whole and never lent as a `&mut` nor a
raw pointer (`whole_writes`, so a closure that captures it by `&mut` is
none), where nothing writes it on any way from where its test is written
to the read, nor after the test in its own block (`unwritten_since`):
React's getIteratorFn's `maybeIterator`, a `match`'s in each arm,
performUnitOfWork's `next`, getClosestInstanceFromNode's `targetInst`. The
ways are one owner's, `ways_from_test`, which `field_reads` asks too. And
`js::is_function`'s true edge proves what it tests, as `js::truthy`'s does
(`is_js_presence_test`), React's scheduler's `continuationCallback`.

Tidy's if-shown and cond-shown took a `let` a test showed isn't `null` as
the `const` they're written for: one written again in the branch, `found =
begin(0)` of `if (found != null) { .. return found ?? null; }`, was returned
`undefined` where a nullable function gives Rust's `None` as `null`
(#1952, P0). They take a `const` alone now.

Case of [0262](0262-when-rust-and-js-disagree.md): none of (A); #1952's
results change, Rust's kept. React's port: `workInProgress = next;`, as
upstream writes it, where it was `next ?? null`. Corpus:
while_test_continues_of_chain's `$index(nodes, at)` after `at != null`.
Held by test/lowering-nullable-tests.test.ts's "a variable written more
than once" (written again between, a closure that captures it called
between, each keeping `?? null`; run), "a let written again after its
test" and "a variable MIR proves nothing of"; corpus case
variable_written_twice_then_tested (native, both builds, as a library,
metamorphic); mutations proved-many-writes-never, js-is-function-unrecognized,
shown-defined-of-a-let new, proved-present-written-again,
field-read-back-through-the-test, present-coalesce-failed-test-unread and
field-test-undecided-where-its-variable-changes retargeted.

