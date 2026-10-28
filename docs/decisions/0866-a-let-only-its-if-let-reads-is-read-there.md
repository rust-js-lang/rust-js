# 0866. A `let` only the `if let` that takes it apart reads is read there

Status: Accepted; amended below: named as its field by their Rust names; a `let` of a cast is no `let` that ends a
borrow; and a let chain's binding of a field its
test read is read again where the body reads it first; and an `if let`'s
binding of a thread-local's value only its test reads; and a `let` that
ends a borrow that the statement after it stores. Extends [0842](0842-a-let-chain-after-a-test-of-its-own-reads-its-place-once.md)'s
and [0780](0780-an-or-patterns-tests-are-as-written-of-a-field-read-in-place.md)'s
places read in a test to a `let` the program names, of a `RefCell`'s value
too, which [0863](0863-a-refcell-of-an-object-is-the-object.md) made the
object.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same value read:
nothing runs between the `let` and the test's reads, and the test runs no
code, so the place holds what the `let` read. A read the test's `&&` skips
is of a property no getter answers, which does nothing.

## Context

React's `startTransition` checks, in its `finally`, that an inner
Transition took the outer one's types:

```js
if (__DEV__) {
  if (
    prevTransition.types !== null &&
    prevTransition.types !== currentTransition.types
  ) {
    console.error(..);
  }
}
```

Its port ends the `RefCell`'s borrow before it takes the value apart, as
Rust must where code that may borrow the cell runs while it's held:

```rust
let prev_types = prev_transition.types.borrow().clone();
if __DEV__
    && let Some(prev_types) = prev_types
    && !Rc::ptr_eq(&prev_types, &current_types)
{
    console::error(..);
}
```

which was `const prevTypes = prevTransition.types; if (__DEV__ && prevTypes
!= null && prevTypes !== currentTypes)`. Without the cell, `if __DEV__ &&
let Some(prev_types) = &prev_transition.types && ..`, MIR's temporary of
the place is read in place (ADR 0672), `prevTransition.types != null &&
prevTransition.types !== currentTypes`, upstream's. The `let` the borrow
asks for kept the `const`: tidy reads in place only what MIR made, never a
name the program wrote.

## Decision

**A `let` an `if let` takes apart, the variable alone its scrutinee, whose
value is a place's read, which only the test of the `if` right after it
reads, is that place, read where the test reads it** (`moves::tested_reads`,
after `and tests` makes the chain one test):

- **taken apart**: THIR says so of the `let` (`LetNames::taken_apart`),
  `let x = ..; if let Some(x) = x`: the `let` only holds the value for
  the pattern, as a borrow or a temporary's life asked, which JS hasn't;
- **a place's read**: a property of a variable, a chain of them, which no
  getter may answer (`Expr::has_effects`);
- **only the test reads it**, once or more, the `if`'s branches and what
  follows reading none of it;
- **the test runs no code** (`Expr::has_effects`), so nothing between the
  `let` and each read can change the place.

```js
if (__DEV__ && prevTransition.types != null && prevTransition.types !== currentTypes) {
  console.error(..);
}
```

A `let` the body reads too stays, one read before the test: the body may
run code that changes the place, `console.error` above, after which the
place holds another value than Rust's variable. So `startTransition`'s
`const currentTypes = currentTransition.types;`, which the body writes to
the outer Transition, where upstream reads `currentTransition.types` again.
A `let` no `if let` takes apart, compared or tested, keeps its name, as
React's `const objectPrototype = Object.prototype; if (object == null ||
object === objectPrototype)` keeps its own.

## Why

- **It's the Rust without the borrow**: the `let` is there for Rust's
  borrow check, and the JS reads as the scrutinee written in place reads.
- **The same value**: nothing runs between, as the test runs no code.
- **Names the program writes for themselves stay**: only a `let` an `if
  let` consumes, whose name the pattern binds again, is let go.

## Alternatives

- **Every `let` of a place's read only a test reads**: React's
  `ReactSerializationErrors`' `objectPrototype`, upstream's own name too,
  was lost.
- **A `RefCell`'s value alone**: the same `let` of a plain field's read,
  `let t = &p.plain; if let Some(t) = t`, would read otherwise than the
  `RefCell`'s, and lowering would have to carry which read was a borrow's.
- **Read it in the body too**: a wrong value where the body's code changes
  the cell first.

## Consequences

- No corpus case, snapshot or guide example but the new case changed:
  such a `let` was only React's.
- A test that runs code keeps the `const` before it, as Rust reads the
  place first: `const types = t.types; if (emptied(t) && types != null)`.

## What holds it

- Corpus case `refcell_let_taken_apart` against native Rust: React's
  check, read in the test; one whose test empties the cell first, one
  whose body reads it, and one no `if let` takes apart, each a `const`.
- Mutations `tested-read-of-any-let`, `tested-read-past-a-call`,
  `tested-read-body-unread` (`lower/mir/tidy/moves.ts`) and
  `let-taken-apart-unseen` (`lower/mir/patterns.ts`).

## Amendment: a let chain's binding of a field its test read

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same value
read: nothing runs between the test's read and the body's.

React's runWithFiberInDEV writes `if (fiber !== null && fiber._debugTask)
{ return fiber._debugTask.run(callback.bind(..)); }`. Its port, `if let
Some(fiber) = &fiber && let Some(debug_task) = fiber.debug_task.get() {
return debug_task.run(move || callback(..)); }`, printed `const debugTask
= fiber._debugTask; return debugTask.run(..);` in the body: `and tests`
joins the chain into one test that reads the field in the binding's place
(`bound_into`), and keeps the binding's `const` for the body, a name Rust
asks for, as a `Cell`'s `get` copies the value out, which JS needn't.

**A let chain's binding that `and tests` read in the test is that place,
read again where the body reads it first** (`moves::read_again`, asked by
`and_tests` of each binding it keeps): the binding read once, in the
statement right after it, where nothing JS evaluates before it there does
anything (`leads`), so nothing can have changed the place since the test
read it; `bound_into` says it reads the same again and no getter answers
it. The test's own code ran before the binding's read either way: a
binding read past a test that runs code isn't joined at all (ADR 0842,
amended). Only of a binding the test read: a pattern's parts no test
reads, `const a = value[0]; const b = value[1];`, keep the program's
names. `test/lowering-branches.test.ts`'s else-if chain of a binding in
its arm reads it twice now, to keep one there; read once, it's
`message = kind._0[0];`.

It stays a `const`, the value the test read, where the body runs code
first, `fiber._debugTask = null; return task.run(..)`, or in the statement
that reads it, `probe(fiber) + task.run(..)`, and where it's read twice,
`task.run(..) + task.run(..)`, whose first call may change the field.

Held by `test/lowering-written-branches.test.ts`, "a let chain's binding of
a field the test read is read again where the body reads it first", run,
with a task whose `run` clears the field; `let_chain_binding_read_again.rs`
against native Rust; and the mutations
`chain-binding-not-read-again`, `chain-binding-untested-read-again`
(`tidy/logicals.ts`), `read-again-twice`,
`read-again-past-code` and `read-again-of-a-later-statement`
(`tidy/moves.ts`).

## Amendment: not a `let` of a cast (2026-10-26)

React's markUpdateLaneFromFiberToRoot names what it tests:

```js
const offscreenInstance: OffscreenInstance | null = parent.stateNode;
if (
  offscreenInstance !== null &&
  !(offscreenInstance._visibility & OffscreenVisible)
) {
```

Its port gives the field's value its type, `let offscreen_instance:
Option<&OffscreenInstance> = unsafe { js::cast(parent.state_node.get())
};`, and an `if let` takes it apart, so it was read in the test's place
twice, `parent.stateNode && !(parent.stateNode._visibility & 1)`, the
program's name gone. **A `let` of a `js::cast(..)` is the program's name for
a value of the type it gives, not a `let` that ends a borrow**: it stays
a `const`, `const offscreenInstance = parent.stateNode; if
(offscreenInstance && ..)`, as upstream writes it. Whether it's tested by
its truth or against `null` is ADR 0298's, by what its payload may be. Of
a variable, `let lazy: &LazyComponent = unsafe { js::cast(children) };`,
it's the variable as before (ADR 0598, amended).

Held by `test/lowering-parts.test.ts`, "a let of a cast an if let tests
keeps its name", run in JS, and the mutation
`cast-let-of-a-place-tested-in-its-place` (`lower/mir/patterns.ts`).

## Amendment: an `if let`'s binding of a thread-local

React's ReactFiberCacheComponent and ReactFiberThenable test a module
variable where it is, `if (lastFiber !== null && lastFiber.index ===
fiber.index)`. The port's `if let Some(last_fiber) =
LAST_FIBER.with_borrow(Clone::clone) && last_fiber.index == fiber.index {
LAST_FIBER.set(None) }` was `const lastFiber$1 = lastFiber; if (lastFiber$1
!= null && lastFiber$1.index === fiber.index)`: the binding is no `let`,
and where tidy saw it, the thread-local's read was `$withBorrow(..)`, a
call, which only module assembly (`plain_statics`) made the variable.

**What an `if let`'s pattern binds of a thread-local's value, `get`'s or
`with_borrow`'s, is taken apart as such a `let` is**, read in the test
where only the test reads it and runs no code, as a copy of a variable is
the variable (ADR 0434): `if (lastFiber != null && lastFiber.index ===
fiber.index) { lastFiber = null; }`. And a `with_borrow` of the module's
own thread-local that's its variable (`quiet_locals`, ADR 0270), whose
borrows nothing counts, is its function given what the cell holds where
it's lowered (`QuietCells`), as `plain_statics` gives it, so tidy reads a
place, `LAST_FIBER.value`, not a call. Another module's is as its import
reads it, `$withBorrow(callbacks, (items) => items.length, true)`.

- **Not a binding of a field**, which a let chain reads once (ADR 0842):
  `const count = counts.count; if (count != null && count > 3)` stays.
- **Not one read after a call**, `if let Some(count) = COUNT.get() &&
  bump() < count`: the test runs code, which may set the variable, so the
  binding is its own `const`, as ADR 0277's amendment holds after it too.
- Upstream tests `!== null` where rust-js tests `!= null`: a nullable
  thread-local's `None` is `null`, but which test a nullable place's
  presence is read by is the nullable rules' (ADR 0586), apart from this.

Case A ([0262](0262-when-rust-and-js-disagree.md)): the test runs no
code, so the variable holds what the binding read. Tested by
`test/lowering-shadows.test.ts`'s "an if let's binding of a thread-local
only its test reads", run, and the module variables test's another
module's `with_borrow`; its mutations are
`quiet-with-borrow-called-late` and `quiet-with-borrow-of-another-module`.

Later (2026-10-28): the pattern's binding taken apart by a rule of its own
is gone. ADR 0277's amendment of 7915789d0, a copy of a thread-local set
after its last read is the variable, gives the same JS of each of these,
the binding read only in the test, after it or past a call, so the rule's
mutations, `thread-local-binding-not-taken-apart` and
`field-binding-taken-apart`, survived; they went with it.

## Amendment: a binding of a field's own name is the field where the `then` gives it

The user's decision (2026-10-27). React's getViewTransitionName writes
`if (instance.autoName !== null) { return instance.autoName; }`; its port,
`if let Some(auto_name) = &*instance.auto_name.borrow() { return
auto_name.clone(); }`, printed `const autoName = instance.autoName; if
(autoName != null) { return autoName; }`: `tested_read` took only a `let`
the test alone reads.

**A `const x = p.x` named as the property it reads, a plain field's
that no getter answers, is read in place in the `if` after it where the
test reads it and the `then`'s first statement gives it alone**, `return
x`, nothing run between: the field holds what the test read there, so
the program's name is no loss, being the field's. Another name, `if let
Some(name) = ..`, keeps the `const`, as does a binding read anywhere
else. Case N ([0262](0262-when-rust-and-js-disagree.md)): a plain
property read twice, nothing between, is one value.

Held by `test/lowering-written-branches.test.ts`'s "an if let of a field
by its own name reads the field in place", run in JS, and the mutation
`own-name-read-never-in-place` (`tidy/moves.ts`), beside the reworded
`tested-read-of-any-let`, `tested-read-past-a-call` and
`tested-read-body-unread`.

## Amendment: a `let` that ends a borrow, stored by the statement after it (2026-10-27)

The user's decision (r07 of the React port's gaps). React's
createWorkInProgress copies a fiber's links, `workInProgress.child =
current.child;`. Its port ends the borrow first, as Rust must:
`work_in_progress.child.replace(current.child.borrow().clone())` holds
`current.child`'s borrow across the `replace`, which panics where
`work_in_progress` is `current`, so rust-js counts the field's borrows
everywhere. The port writes

```rust
let child = current.child.borrow().clone();
work_in_progress.child.replace(child);
```

which printed `const child = current.child; workInProgress.child =
child;`: the `let` only holds the value while the borrow ends, which JS
hasn't.

**A `let` of a `RefCell`'s value cloned, `x.borrow().clone()`
(`LetNames::ends_borrow`, from THIR), of a place's read that no getter
may answer, stored whole by the statement right after it, `target = child`
or a nullable place's `child ?? null`, read nowhere else, where nothing
JS evaluates before that read does anything (`read_again`), is that place
read there**: `workInProgress.child = current.child;`, the name the field's
own or not, `node = node.return;`. Nothing runs between the two reads, so
the place holds what the `let` read. The pass runs in tidy's `written`
stage, after `unread made` drops the `const old` of what `replace` gave.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same value read.

- **Only a store**, as decided. Measured on the React port: it takes 14
  `const`s where upstream reads in place (createWorkInProgress's `child`,
  `sibling` and `_debugHookTypes`, the update queues' `update.next =
  pending.next`, `sibling.return = node.return` three times,
  `hydrationParentFiber = fiber.return`, ReactFiberGestureScheduler's
  `gesturePrev.next = gesture.next`, ..), and none upstream names. The same
  rule of any read in the next statement also took a test's, `const
  alternate = parent.alternate; if (alternate === null && ..)`, an
  object's, `primaryChildFragment.sibling`, and arguments upstream names,
  `const handlerOnStack = ..; push(.., handlerOnStack, ..)`: names
  upstream keeps, so a test, an object or an argument keeps its `const`.
- **Not a `Cell`'s `get()`**, which ends no borrow: `let index =
  current.index.get();` is the program's name for a value it could have
  written in place.

What changes: `rc_assigned_through_mut.rs`'s `update.next = pending.next;`
and `while_exit_continues.rs`'s `sibling.return = node.return;`, and two
tests' expectations of the same stores; `docs/readability.txt`'s renamed
`x$1` share falls, 702.2 to 700.8 in 10,000 lines.

Held by the corpus's `refcell_let_read_in_place.rs`, against native Rust:
a store, a nullable field's, an argument, one past a call that empties
the cell, one read twice, a `Cell`'s; and the mutations `ended-borrow-kept`,
`ended-borrow-passed`, `ended-borrow-nulled-kept` (`tidy/moves.ts`) and
`ended-borrow-of-any-let` (`mir/patterns.ts`).

## Amendment: only of the field's own name (2026-10-27, r1)

The amendment above took every such `let`, and React's own names went
with them: ReactFiberHooks' updateReducerImpl writes `const pendingFirst =
pendingQueue.next; baseQueue.next = pendingFirst;` and dispatchActionState
`const first = last.next; actionNode.next = first;`, which printed
`baseQueue.next = pendingQueue.next;` (r1 of the port's gaps).

**Only a `let` named as the field it reads, `child` of `current.child`,
`next` of `pending.next`, is read in place**, as the previous amendment's
`if let` of a field's own name is: that name says nothing the field
doesn't, and the `let` is there for the borrow alone. One named otherwise
is the program's name, as upstream's `pendingFirst` is.

- What this gives back: `const returnFiber = node.return; sibling.return =
  returnFiber;` (`while_exit_continues.rs`, React's
  ReactFiberCommitHostEffects and SuspenseComponent, which upstream reads in
  place, `sibling.return = node.return`), `const next = node.return; node =
  next;`, and createWorkInProgress's `_debugHookTypes`, whose name is its
  `#[rust_js::name]`. Those ports name the `let` as the field, `let
  r#return = ..`, where they want upstream's form.
- Held by the corpus case's `swap_children`, `const pendingFirst =
  pending.child;`, and the mutation `ended-borrow-of-another-name`
  (`tidy/moves.ts`).

## Amendment: named as its field by their Rust names (lt1, 2026-10-27)

A `let` that ends a borrow is read in place only where it's named as the
field it reads. That was asked of their JS names in tidy, `child` of
`current.child`, so two upstream stores stayed apart: `let r#return =
node.r#return.borrow().clone();`, whose JS name is `return$`, and `let
debug_hook_types = current.debug_hook_types.borrow().clone();` of a field
`#[rust_js::name = "_debugHookTypes"]`, whose names differ in JS only.

**The `let`'s name and the field's are their Rust names**, the binding's
and the field's own, asked where the `let` is found (`mir/patterns.rs`,
`LetNames::ends_borrow`, which holds only those so named), not of the JS
tidy reads: `sibling.return = node.return;`,
`workInProgress._debugHookTypes = current._debugHookTypes;`.

- Case ([0262](0262-when-rust-and-js-disagree.md)): N.
- Held by the corpus case `refcell_let_named_as_its_field.rs` and the
  mutations `ended-borrow-by-js-names` and the moved and amended
  `ended-borrow-of-another-name`, `ended-borrow-of-any-let`.
