# 0866. A `let` only the `if let` that takes it apart reads is read there

Status: Accepted; its first decision, a `let` the program wrote read in its `if let`'s test, withdrawn (rd226, the user's decision, below); amended by [1022](1022-a-field-of-a-struct-js-made-is-read-as-rust-reads-it.md): a place's read is plain only of a struct the program built; amended below: named as its field by their Rust names; a `let` of a cast is no `let` that ends a
borrow; and a let chain's binding of a field its
test read is read again where the body reads it first; and an `if let`'s
binding of a thread-local's value only its test reads; and a `let` that
ends a borrow that the statement after it stores; and a `let` the program
wrote read first by the next statement's `||` or `??` (rd664, the user's
decision); and one read only as the receiver and `this` of the next
keyed method call (rd226b, the user's decision); and a let chain's
binding read again only where it's named as its field (rd706). Extends [0842](0842-a-let-chain-after-a-test-of-its-own-reads-its-place-once.md)'s
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

## Amendment: a `let` the program wrote stays (rd226, the user's decision)

The user's rule (2026-10-30): **a `let` the program wrote stays; a binding
that exists only in a pattern is read in place.** React's prepareForCommit
writes `const focusedElem = selectionInformation.focusedElem; if
(focusedElem !== null)`, which the port's `let focused_elem = ..; if let
Some(focused_elem) = focused_elem` read in the test, its name lost, as
this ADR's first decision said of a `let` an `if let` takes apart. So
`tested reads` takes no `let` the program wrote (`LetNames::named`), and
`LetNames::taken_apart`, which only it asked, is gone: startTransition's
`let prev_types = prev.types.borrow().clone(); if DEV && let
Some(prev_types) = prev_types && ..` is `const prevTypes = prev.types; if
(DEV && prevTypes != null && prevTypes !== currentTypes)`. A pattern's
binding named as its field, getViewTransitionName's `if let Some(auto_name)
= &*instance.auto_name.borrow() { return auto_name.clone(); }`, is still
`if (instance.autoName != null) { return instance.autoName; }`; and the
let chain's binding read again, and the thread-local's, are patterns'
too. The `let` of a cast, which the first decision left out, is one the
program wrote, so it stays without a rule of its own. The differences the
user listed: prepareForCommit's and createTextInstance's pattern bindings
read twice where upstream holds a `const`.

Held by `test/corpus/refcell_let_taken_apart.rs` (`finish`'s `const
prevTypes`, `first`'s written `let` of the field's own name kept; `own`'s
pattern binding read in place, `ownTwice`'s and `ownChecked`'s held),
mutations `tested-read-of-a-written-let`, `tested-read-past-a-call`
(`tidy/moves.ts`, snapshots), `own-name-read-never-in-place`; the
patterns.ts mutations of `taken_apart` and a cast's `let` are gone with it.
`test/lowering-moves.test.ts`'s ADR 1022 tests write the program's own
structs' reads as pattern bindings now, and hold a JS-made struct's by a
`let` that ends a borrow, `const child = current.child;`, which still reads
in place where the field is the program's.


## Amendment: a `let` read first by the next statement's fallback is read there (rd664, the user's decision)

The user's decision (2026-11-03), an exception to rd226's rule: **a `let`
the program wrote, named as the property it reads, whose one read is the
left side of a `||` or a `??` in the statement right after it, and the
first thing that statement evaluates, is the property read there.** React's
readAsDataURL writes `const mimeType = blob.type ||
'application/octet-stream';`. Its port, the idiomatic `let r#type =
blob.r#type(); let mime_type = if r#type.is_empty() { .. } else { &r#type
};`, printed `const type = blob.type; const mimeType = type || ..`: Rust
holds the value because its `if` reads it twice, its test and its `else`,
and borrows it; JS's `||` reads it once, so the `let` has no job left in
the JS. Across the port's JS, each `const k = o.k;` read once, as the left
side of the next statement's `||` or `??`, was `o.k` read in place
upstream, 12 of 12: readAsDataURL's three, ReactFlightClient's `ioInfo.name
|| 'unknown'`, EnterLeaveEventPlugin's `relatedTarget ||`,
ReactDOMEventReplaying's `ownerDocument ||`, the bundlers' `transferSize ||
0` and react-art's `scale ?? 1`.

ADR 0262's case N: what the program does is the same. The property, a
getter's too, is read once either way, as the `||`'s left side is
evaluated once, and where it was, as nothing at all runs between the
`const` and that read; it throws where it threw. `moves.rs`'s
`fallback_reads` (pass "fallback reads", after "ended borrows") takes only
that: a `const` of `LetNames::named`, of a `Member` whose property is its
name, mentioned once in what follows, by the next statement, as a `||`'s
or `??`'s left side, that `leads` says is read before anything else there
is evaluated, not even a variable. It stays where it's read twice,
`const type = ..; const mimeType = type || ..; return mimeType + type;`;
after something else, `note() + (type || ..)`; named otherwise, `const t =
blob.type;`; past the next statement; and where what reads it first is no
fallback, `name != null`, rd226's case.

Held by `test/corpus/let_read_in_its_fallback.rs` against native Rust and
`test/lowering-moves.test.ts`'s "a let read first by the next statement's
fallback is its getter read there", whose getters count their reads and
their order, mutations `fallback-reads-unread` (snapshots),
`fallback-read-twice`, `fallback-read-of-another-name`,
`fallback-read-after-a-call` and `fallback-read-of-any-read`.

## Amendment: a `let` read only as the receiver and `this` of the next keyed method call is read there (rd226b, the user's decision)

The user's decision (2026-11-03), a second exception to rd226's rule: **a
`let` the program wrote, not `mut`, whose only two reads are the receiver
and the `this` of the keyed method call that the statement right after
it is, `function::call(js::method((x, key)), Some(&x), args);`, is that
call of the value read in place.** react-art calls the functions it sets
on art's nodes as their methods, `props.fill.applyFill(instance)` and
`instance._subscriptions[type]()`. Their port names the value, as rd334's
form must, the path's value and the `this` the same variable
([1025](1025-a-method-read-as-a-value-is-js-methods.md), amended):

```rust
let fill = js::get(props, "fill");
js::function::call(js::method((fill, "applyFill")), Some(&fill), (instance,));
```

which printed `const fill = props.fill; fill.applyFill(instance);`. The
`let` is there only because the form names the value twice; JS's method
call reads its receiver once and gives it as `this`, so it has no job left
in the JS: `props.fill.applyFill(instance);`, as upstream writes it.

ADR 0262's case N: what the program does is the same. Rust reads
`props.fill` once, into the `let`; then `js::method` reads `applyFill` of
that value, then its key's and its arguments' code runs, and the call gets
the same value as `this`. JS's `props.fill.applyFill(instance)` reads
`props.fill` once, then `applyFill` of it, then the key and the arguments,
and calls it with that value as `this`. So the same getters run, the same
number of times, in the same order, and a member that's no function
throws JS's `TypeError` where it threw.

`moves.rs`'s `receiver_reads` (pass "receiver reads", after "fallback
reads"), a sibling of rd664's `fallback_reads`, takes only that: a
`const` of `LetNames::named`, not of `LetNames::mutable` (THIR's `let
mut`), mentioned twice in what follows, both by the next statement, an
expression statement that is the call `Expr::own_keyed_method_receiver`
names it the receiver of: rd334's call `x.key.call(x, ..args)`
(`Known::OwnMethodCall`) whose method is read in place of the variable,
as its `this` is that variable, of a key (`Known::KeyedMethod`, which
`mir_keyed_method` marks). Read so, the receiver is the first thing the
call evaluates, before the key and the arguments, as rd334 orders its
keyed read before them: `leads` has nothing to ask. `Expr::own_method_called`,
`own_method_call`'s rewrite (`js/make.rs`, the one owner of the method's
call), writes it with the value in the variable's place and drops
`OwnMethodCall`, which a member named `call`, `props.fill.call(props)`,
would otherwise take for another `call` to write.

It stays a `const` where it's read a third time, `fill.applyFill(instance);
return fill;`, or by an argument, `fill.applyFill(fill)`; where it's `mut`;
where it's no `let`, a `match` arm's binding, which the decision isn't of;
past the next statement, `noted("between"); fill.applyFill(instance)`;
where a `return` gives the call, beyond the statement the decision is of;
and of a binding's method, `function::call(js::method((element,
HTMLElement::focus)), Some(&element), ..)`, whose own call `element.focus(..)`
keeps the program's `const` as rd226 has it. Another `this` of a key is
refused (ADR 1025); of a binding's method it's `element.focus.call(other)`,
as before.

What changes: ADR 1025's consequence that a value that isn't a variable
is held in one, `const subscriptions = instance._subscriptions;` before
`subscriptions[type]()`, is now `instance._subscriptions[type]();`; the
guide's types page and the rd334 test read so. No corpus snapshot or
readability line moved.

Held by `test/lowering-is-function.test.ts`'s "a let read only as the
receiver and this of the next keyed method call is read there", run in
JS, whose getters count the value's and the member's reads and their
order against the key's and the argument's, and whose call records its
`this`; mutations `receiver-reads-unread`, `receiver-read-again`,
`receiver-read-by-an-argument`, `receiver-read-mutable`,
`receiver-read-of-any-let`, `receiver-read-past-a-statement`,
`receiver-read-of-any-statement` (`tidy/moves.ts`),
`own-keyed-method-of-a-binding`, `own-method-called-again` (`js/make.ts`)
and `keyed-method-unmarked` (`mir/calls/method_paths.ts`).

## Amendment: an `if let`'s binding of a field's own name, read again where the next statement reads it first (rd416, 2026-11-04)

Decided on the user's behalf (2026-11-03, the user away). This widens the
2026-10-27 amendment above, "a binding of a field's own name is the field
where the `then` gives it", which read the field in place only where the
`then`'s first statement gives it alone, `return x`.

React's ReactFlightClient writes `if (info.key != null) { add("key",
info.key); }`; its port, `if let Some(key) = info.key.clone() {
add("key", Some(js::unknown_of(key)), &mut properties); }`, printed `if
(info.key != null) { const key = info.key; add("key", key, properties);
}`: the test reads the field itself, the binding first in the `then`,
which no rule read in place, as a let chain's is (the amendment "a let
chain's binding of a field its test read").

**A plain `if let`'s binding named as the field it reads, `const x =
p.x` first in the `then` of an `if` whose test read `p.x`, is the field
read again where the statement after it reads it first, nothing JS
evaluates before it there doing anything**, as a let chain's binding is
(`moves::tested_binding`, `read_again`): `if (info.key != null) {
add("key", info.key, properties); }`. The place holds what the test read
there: the test does nothing, no getter answers the field (`place_read`),
and nothing runs between. Another name keeps its `const`, as the
2026-10-27 amendment says; so does a binding read after something runs, or
read again, and a `let` the program wrote (rd226). Case N
([0262](0262-when-rust-and-js-disagree.md)): a plain property read
twice, nothing between, is one value.

Held by the corpus's `if_let_field_read_again` (read in place, `name` and
`props`; kept, `id` of another name, `depth` read after a call, `owner`
read twice, `key` a clone held before its test), against native Rust, and the mutation
`tested-binding-never-read-again` (`tidy/moves.ts`).

## Note: a nullable store of an ended borrow is the fallback rule's too (2026-11-04)

The mutations `ended-borrow-kept` and `ended-borrow-nulled-kept` survived
on main: corpus `refcell_let_read_in_place` stored only a nullable field,
`work_in_progress.child.replace(child)`, whose `child ?? null` rd664's
`fallback_reads`, two passes on, reads in place as well, `const child =
current.child; workInProgress.child = child ?? null;` becoming
`workInProgress.child = current.child;`. So `stored_whole`'s `name ?? null`
arm gives what the fallback rule gives, and its mutation,
`ended-borrow-nulled-kept`, is equivalent and removed. The corpus case
now stores a field that isn't nullable too, `copy_sibling`'s
`workInProgress.sibling = current.sibling;`, which only `ended_borrows`
reads in place, and which holds `ended-borrow-kept`.

## Amendment: a let chain's binding read again only where it's named as its field (rd706)

React's ReactFiberCompleteWork writes, in completeDehydratedActivityBoundary
and completeDehydratedSuspenseBoundary,

```js
if (current !== null && current.memoizedState !== null) {
  const prevState = current.memoizedState;
  prevState.hydrationErrors = hydrationErrors;
}
```

and its port `if let Some(current) = current && let Some(prev_state) =
unsafe { js::cast::<Option<&ActivityState>>(current.memoized_state.get())
} { prev_state.hydration_errors.replace(hydration_errors); }`. Since
1a167522f made an unread `replace` the write alone, `prevState` is read
once, so the let chain's binding read again (the amendment above, of
runWithFiberInDEV) took it: `current.memoizedState.hydrationErrors =
hydrationErrors;`, the program's name gone; a `Cell`'s `set`,
`prev_state.retry_lane.set(1)`, already printed `current.memoizedState.retryLane
= 1;`. That breaks rd226's rule, which the read again was never an exception
to: the bindings it was decided of, runWithFiberInDEV's `debug_task` of
`fiber.debug_task`, are named as their field, as getViewTransitionName's
`auto_name` is.

**A let chain's binding the test read is read again in its place only where
it says nothing the field doesn't: named as the field its scrutinee reads,
by their Rust names (`LetNames::own_field`, `mir/patterns/let_names.rs`),
`debug_task` of `fiber.debug_task.get()`, a `#[rust_js::name =
"_debugTask"]` field's too, or a temporary the compiler made, `_5` of
`let Some(Lanes { retry }) = holder.lanes.get()`.** One named otherwise is
the program's name, a `const` though it's read once: `const prevState =
current.memoizedState; prevState.hydrationErrors = ..`, as upstream writes
it. The Rust names are THIR's: the binding of the scrutinee's whole payload
(through `Some(..)`, a struct's or a reference's pattern of one part), and
the field the scrutinee reads through borrows, derefs and a call's first
argument, a `get()`'s receiver or a cast's value. This is rd226 restored in
`and tests`, no new choice; rd664's `fallback_reads` and rd226b's
`receiver_reads`, its two exceptions, are `moves.rs`'s and stay as they are.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The JS is the same
program either way: the test reads the place, then the body reads it once,
through the `const` or in place, nothing run between; the field is a
struct's the program built, which no getter answers (ADR 1022). Held in a
`const`, it's read once, where Rust reads it.

What changes: the corpus's `let_chain_else_inside_if.rs`, `const item =
a[0];` of `let Some(item) = items.first()`, and `test/lowering-loops.test.ts`'s
`const c = node.count;` of `let Some(c) = node.count`, each a name the
Rust gives. A binding by reference of a field that isn't a cell, `if let
Some(prev_state) = &current.state`, is MIR's handle on the field's place,
which lowering reads where it's used before tidy sees it (ADRs 0672 and
0863), not this pass's: it stays so, apart from this amendment.

Held by `test/lowering-moves.test.ts`'s "a let chain's binding named
otherwise than its field keeps its const", React's two sites and a
temporary still read in place, run in JS; the corpus's
`let_chain_binding_read_again.rs`, `named_otherwise` and `reset`, against
native Rust; and the mutations `chain-binding-of-another-name-read-again`,
`chain-temporary-kept` (`tidy/logicals.ts`), `own-field-binding-of-any-name`
and `own-field-binding-not-through-calls` (`mir/patterns/let_names.ts`).
The test's read of a binding asked apart, which kept `const a = value[0];
const b = value[1];` of a pattern's parts, is gone with its mutation
`chain-binding-untested-read-again`: a part is no payload, so it's no
`own_field`, and a payload binding or a temporary is read by the test
right after it, which the mutation, surviving, showed. `LetNames`' THIR
reading moved from `mir/patterns.rs` to
`mir/patterns/let_names.rs`, with its mutations, for the file cap.
