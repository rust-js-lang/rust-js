# 0277. A variable shadowed by its own value is the variable

Status: Accepted. Amended: another's variable only where nothing runs
before it's read (below); and past a `try`'s start and a binding's call
that runs nothing (below); and set after its last read, in a loop too (below);
and past a way that ends, but not by a `break` (below).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Rust shadows a variable with what's made of it, and every local of a JS
function has a name of its own, so react.dev's errors page read:

```rust
let Type = ElementType::from_unknown(Type);
jsx! { <Type key={key} {...props} /> }
```

```js
const Type$1 = Type;
return <Type$1 key={key} {...props} />;
```

and `let content = content.clone();` before a closure was `const content$1
= content;`. Its original has `<Type ..>` and `content`.

## Decision

**`const n$1 = n;`, a variable shadowed by its own value, is `n`**: each
read of `n$1` reads `n`, and the `const` is gone. Only where `n` holds that
value wherever `n$1` is read: nothing sets `n` after it, nor in a closure,
nor in a loop it's in, where a closure of each turn keeps its own. One of
another name, `let kept = text;`, keeps the name written.

## Why

- **No program can tell**: the two hold one value, the same object or the
  same string, wherever the new one is read.
- **It's the JS a person writes**, and the name the Rust has.
- **It's tested**: a compiler test shadows a captured string and a number
  set before, and keeps one of another name and one a loop sets again,
  whose closures each read their own turn's; mutations alias each.

## Since

- **A closure's own parameter shadowed in it is the parameter too**,
  `(total, item) => total + item.length` of `let item = item.as_str();`,
  as react.dev's createFileMap casts each snippet it's given. Only its
  parameters: nothing outside the closure sets them.

## Amendment: another's variable only where nothing runs before it's read

A local named as a thread-local's module variable, `let count =
COUNT.get(); bump(); count`, was `const count$1 = count; bump(); return
count$1;` until this rule took `count$1` for `count`: nothing in the
function set `count` after it, but `bump()` did, so the function returned
6 where Rust returns 5, a wrong answer. A function's own variables only it
and its closures set, which the rule sees; a module's, or an import, any
code that runs.

**A `const n$1 = n;` of a name the function doesn't declare is `n` only
where each read of `n$1`, in JS's order, is before anything that may run
code**: a call, `new`, an `await`, a JSX element, a binding's getter or
setter. A call's callee and arguments are read before it runs, `if
(holder != null) { bump(holder); }`; a branch's way is its own; a loop, a
`try` or a `switch` that reads it runs nothing. Else it stays its copy,
`const count$1 = count; bump(); return count$1;`, and so does one read
past a binding's property, which a getter may answer, `const saved$1 =
saved; const now = button.onclick;`. The function's own names, its
parameters and what its blocks declare, a `catch`'s, a loop's and a
function's too, are as before; so are a closure's.

The rule is `src/prepare/shadows.rs`'s now, out of `prepare.rs`.

Case "results change" ([0262](0262-when-rust-and-js-disagree.md)):
Rust's answer, preserved. Tested by `test/corpus/shadow_of_module_variable.rs`
against native Rust; the playground's `setTheme` and the bindings test's
`same` keep their copy past a binding's method and getter. Its mutations
are `shadow-of-outer-variable-aliased`, `outer-shadow-read-past-a-call` and
`outer-shadow-read-before-a-call-copied`.

## Amendment: past a `try`'s start, and a binding's call that runs nothing (2026-10-26)

The rule above kept React's reconciler's copies where upstream reads the
variable:

```js
// ReactFiberAsyncAction.js, startIsomorphicDefaultIndicatorIfNeeded
const isomorphicDefaultTransitionIndicator$1 = isomorphicDefaultTransitionIndicator;
if (typeof isomorphicDefaultTransitionIndicator$1 === 'function' && !pendingIsomorphicIndicator) {
  try {
    const cleanup = isomorphicDefaultTransitionIndicator$1();
// ReactFiberHotReloading.js, scheduleFibersWithFamiliesRecursively
const failedBoundaries$1 = failedBoundaries;
if (failedBoundaries$1 && (failedBoundaries$1.has(fiber) || (alternate != null && failedBoundaries$1.has(alternate)))) {
```

Between the copy and each read lies nothing that can set the variable:
in the first, a read of another module variable, then a `try` whose
first act is the call that reads it, its callee read before it runs;
in the second, a weak set's `has(..)`. A module's variable is set only by
its module's code (ADR 0270), so only code of the program's that runs
between can set it; what's in question is what each call may run.

- **A `try`'s block is read in its order, as any block is, and its
  `catch` or `finally` after all of it**: either may run after any of
  the block, which may have run code before it threw. So the indicator's
  call in its `try` is the variable's, `isomorphicDefaultTransitionIndicator()`,
  and a copy read in the `catch` after a call that set the variable, as
  Rust reads what it copied, stays a copy: `const level$1 = level; try {
  return raise(); } catch { return level$1 + 100; }`.
- **A binding's call that runs none of the program's code sets none of
  its variables**, `failedBoundaries.has(fiber)`. Which calls these are
  is the bindings' own answer, ADR 0867's and ADR 0898's, which
  `cell_borrows::binding_call_runs_nothing` gives: one marked
  `#[rust_js::runs_nothing]`, a weak collection's methods, `new
  AggregateError(..)`; or one marked `#[rust_js::runs_only_given]` given
  only numbers and text, `console.error("..")`. Lowering marks its call
  so (`Known::RunsNothing`), and the rule reads the mark; its callee and
  arguments are read as any part is, so a call among them still counts.
- **Any other binding's call may run anything**, given nothing too: JS
  reaches the program through a callback it holds, one given now or
  before, `dispatchEvent` running a listener added earlier, and through
  any function the program exports. Telling which of those may set a
  variable is a whole program's question, which the rule doesn't ask;
  only the binding's author knows it runs none, as they know a property
  is data (ADR 0301). So `before.call(); return failedBoundaries$1.has(fiber);`
  keeps its copy.

Case "results change" ([0262](0262-when-rust-and-js-disagree.md)), as
before: Rust's answer, preserved, the copy elided only where no code that
can set the variable runs before a read. What the rule assumes of a
marked binding is what the borrow check assumes of it already (ADR 0867):
a binding marked wrongly is wrong for both.

**Alternatives:**

- **Every binding's call runs nothing of the program's** unless it's given
  a callback: wrong of `el.click()` and `dispatchEvent`, which run a
  listener given before, and of a host's JS calling an export.
- **The variable's writers, by whether JS can reach them**: a whole
  program's escape analysis, every function given to JS and every export,
  where React's writers, `markFailedErrorBoundaryForHotReloading` among
  them, are exported anyway.

Tested by `test/corpus/shadow_of_module_variable_in_try.rs` against
native Rust, a copy read in a `try` before and after a call that sets the
variable and in its `catch`, whose kept copies, `indicator$1` and
`level$1`, are what it's of: `docs/readability.txt`'s `renamed x$1` counts
10 more with it, 8026 to 8036; and
`test/lowering-shadows.test.ts`, "a copy
of a module's variable read past a call that runs nothing is the
variable", React's `failedBoundaries`, and one past `before.call()`, a
callback that sets the variable, run as JS. Its mutations are
`shadow-catch-read-before-its-block`, `shadow-read-in-try-copied`,
`runs-nothing-runs-code` (`prepare/shadows.rs`),
`binding-call-runs-nothing-unmarked` and `every-binding-call-runs-nothing`
(`lower/binding_calls.rs`).

## Amendment: set after its last read, in a loop too (h4, 2026-10-27)

React's popToNextHostParent walks up from a fiber by a module variable,
`hydrationParentFiber`, set at the end of each turn:

```js
while (hydrationParentFiber) {
  switch (hydrationParentFiber.tag) {
    case HostComponent: .. return;
    default: hydrationParentFiber = hydrationParentFiber.return;
  }
}
```

The port's `while let Some(parent) = HYDRATION_PARENT_FIBER.with_borrow(
Clone::clone)` copies it each turn, and the copy stayed one, `const
hydrationParentFiber$1 = hydrationParentFiber;`, as the rule took any set
of the variable after the copy, or anywhere in a loop around it, for one
its reads might see. A set after the copy's last read can't be seen: what
follows reads nothing of it, and a loop's next turn makes the copy anew,
the copy a `const` of the loop's block.

- **A copy whose variable is set after it is the variable where each of
  its reads, in JS's order, is before each set** (`read_before_set`): a
  branch's ways each their own, a `switch`'s cases one after another, as
  one falls into the next, a store of the variable reading it first,
  `hydrationParentFiber = hydrationParentFiber.parent`; and a loop in what
  follows, which goes round, only where it reads it or sets it, not both.
- **Not one a closure, a function or a handle's accessors read**
  (`read_later`): each reads it when it's called, after the variable may
  be set; a handle's `{ get value() { return list$1.next; } }`, kept on
  the copy, is called later too.
- Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same value
  read.
- Held by the corpus case `loop_over_a_thread_local.rs`, React's loop;
  `test/lowering-shadows.test.ts`, "a copy of a module's variable read
  after the variable is set stays a copy", its stores and `if`s and
  `switch`es after the set, a closure's read, and a loop that reads after
  it sets, each run; and the mutations `shadow-set-after-never-read-in-place`,
  `shadow-read-later-in-place`, `shadow-read-after-set-in-place`,
  `shadow-stored-after-set-in-place`, `shadow-tested-after-set-in-place`
  and `shadow-switched-after-set-in-place`.

## Amendment: a module's variable read, set and stored again is the variable (2026-10-28)

React port gap rd35. react-dom-bindings' FallbackCompositionState caches
its text in a module variable, `if (fallbackText) { return fallbackText;
} .. fallbackText = endValue.slice(..); return fallbackText;`. Its port
reads the thread-local into a local, sets it and stores it back:

```rust
if let Some(fallback_text) = FALLBACK_TEXT.with_borrow(Clone::clone).filter(|text| !text.is_empty()) {
    return fallback_text;
}
let fallback_text = js::string::slice(end_value, 1.0, None);
FALLBACK_TEXT.set(Some(fallback_text.clone()));
fallback_text
```

which printed a second name of it, `let fallbackText$1 = fallbackText; ..
fallbackText$1 = ..; fallbackText = fallbackText$1; return
fallbackText$1;`: a `let`, set again, which this decision's `const`s
aren't.

**A `let n$1 = n;` whose every write is right before its store back, `n
= n$1;`, is `n`** (`written_back`, in `prepare/shadows.rs`): `if
(fallbackText) { return fallbackText; } fallbackText = endValue.slice(1);
return fallbackText;`. Of `n` not the function's own, each read of `n$1`
is before anything that may run code since the `let` or the last store,
each of which makes the two one again (`synced_reads`); nothing else
sets `n`, and no closure sets either nor reads `n$1`. One read past a
call that may set `n`, or set and never stored, keeps its local.

- Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each read reads
  the same value, and each store the same: the two are equal wherever the
  local is read.
- Held by `test/lowering-shadows.test.ts`, "a module's variable read into
  a local and stored again is the variable", run, with one read past a
  call that sets it and one never stored; mutations `written-back-off`,
  `written-back-past-code`, `written-back-unsynced` and
  `written-back-unpaired` (`prepare/shadows.ts`).

## Amendment: a property's object is read before its getter (rd146)

React DOM's ChangeEventPlugin, ported, writes `let Some(active_element) =
ACTIVE_ELEMENT.get() else { return; }; detach_event(active_element, ..);
ACTIVE_ELEMENT.set(js::null());`, and it printed `const activeElement$1 =
activeElement;` and read the copy, where upstream reads `activeElement`
itself. The binding's call is a method's, `activeElement$1.detachEvent(..)`,
whose property read may run a getter (`Known::Getter`), and the rule took
the whole callee for code that runs before the copy is read.

**A property's object is read before its getter may run**
(`read_first`): `activeElement.detachEvent(..)` reads `activeElement`,
then the getter, then calls what it gave with that object as `this`, as
the copy would. A read again after the getter, `name(active_element)`
then `detach_event(active_element, ..)`, keeps its copy, which holds what
Rust read where the getter set the variable.

Case "results change" ([0262](0262-when-rust-and-js-disagree.md)): Rust's
answer, preserved: what each getter sets never changes the receiver.
Held by `test/lowering-shadows.test.ts`, "a copy of a module's variable
read as a property's object is the variable", run with getters that set
the variable; the mutations `shadow-receiver-read-after-getter` and
`shadow-getter-runs-nothing`.

## Amendment: a closure's own of the name stays its own (#457)

React's ReactFlightReplyClient, ported, made each lazy part's retry reject.
Its `serializeModel` writes `let value: &FormData = unsafe { js::cast(value) };`
in one case, lowered `const value$1 = value;`, and in another a `move`
closure, `retry`, whose `catch_unwind` result MIR's temporary named
`value` by default, given apart as `value$1` from the `value` it captures.
The closure's tidy names its temporaries apart from what it reads, not
from what its maker's lowering named, so both were `value$1`. This rule
renamed every read of `value$1` in the function to `value` and removed the
maker's `const`: the closure's declaration kept `value$1` and its reads,
`value.TAG` and `value._0`, read the captured model, so every retry
rejected with `thrown(model._0)`.

**A shadow's reads are renamed only where its name is the shadow**
(`js::outside_shadows`): a closure or function in the body that takes a
parameter of the name, as before, or declares one in its own blocks, a
`const`, `let`, pattern, loop variable, `catch` binding or declared
function's name, reads its own there and is set apart while the reads
are renamed. The one other user, tidy's `readable_temporaries`,
gets the same answer: a temporary renamed in its maker never renames a
closure's own of the name.

Case "results change" ([0262](0262-when-rust-and-js-disagree.md)): Rust's
answer, preserved, as each variable's reads are its own. Held by the corpus
case `shadow_beside_closure_own_of_its_name`, a closure's and a declared
function's own `value$1` beside their maker's shadow, run against native
Rust; the mutations `outside-shadows-own-declaration-not-apart`,
`declares-own-not-const` (`js/stmt_walk.ts`) and
`shadow-renamed-in-closure-own` (`prepare/shadows.ts`). `written_back`
renames as `shadows` did; no program yet gives a closure an own name
that its `let n$1 = n;` is, and none is set apart there.

## Amendment: a way that ends runs nothing what follows sees (rd646)

React's Parcel client tests its registered callback and calls it:

```js
function callCurrentServerCallback(id, args) {
  if (!callServer) {
    throw new Error('No server callback has been registered. ..');
  }
  return callServer(id, args);
}
```

Its port, `let Some(call_server) = CALL_SERVER.get() else { panic!(..) };
call_server(id, args)`, printed `const callServer$1 = callServer; if
(callServer$1 == null) { throw new Error(..); } return
callServer$1(id, args);`. The copy's last read came after the `if`, whose
`then` makes an `Error`, a `new` that may run code, and `read_before_code`
took what either way of an `if` ran for what ran before the statement
after it.

**What follows an `if` sees only the way that goes on** (`after_ways`): a
way that ends, by a `return`, a `throw`, a `continue`, or a call that
never returns (`leaves`), leaves the list the copy is declared in, so nothing it runs, or sets, comes before a read after the
`if`. Both `read_before_code`, of what may run code, and `read_before_set`,
of what sets the variable, ask it. So `if (callServer == null) { throw
new Error(..); } return callServer(id, args);`, as upstream writes it.
A copy read past the way that goes on, where that way runs code or sets
the variable, stays one: `if (reset) { setServerCallback(double); } else
{ return callServer$1(id) + 1; } return callServer$1(id);`, which reads
the callback the copy holds.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The copy and the
variable hold one value wherever the copy is read, as nothing that runs
between them is on that read's way. Held by the corpus case
`copy_read_past_an_exit.rs`, run against native Rust: the Parcel client's
throw, a `return` after a set of the variable in a way that ends, a set
between, a call that sets it between, and a way that sets it while the
other ends; the mutations `shadow-ended-way-runs-on`,
`shadow-way-going-on-runs-nothing` and `shadow-ended-way-sets-on`
(`prepare/shadows.ts`).

### A `break` goes on past its block

At first a way that ended by a `break` left the list too, as `js::ends`
says of one, and that was a wrong answer: a `break` leaves a labeled block,
or a `switch`, that the statements after the copy hold, for what follows
it, where the copy is read. `'done: { if swap { CALL_SERVER.set(Some(triple));
break 'done; } } call_server(id)` printed `done: { if (swap) { callServer =
triple; break done; } } return callServer(id);`, which called `triple`
where Rust calls the `double` it read (P0, found by a hunt for wrong
answers, of e5818acfc).

So a way leaves the list only where it reaches none of its statements
(`leaves`, `prepare/shadows.rs`): it returns, throws, never returns, or
goes round, by a `continue`, the loop that holds the copy, since a loop
after the copy is looked at whole, never way by way. A `break` is never
one. Case: N, as above. Held by `copy_read_past_an_exit.rs`'s
`set_then_break`, and its `set_then_unwind`, a set in a way that throws
in a `try` whose `catch` reads the copy, which stays a copy, run against
native Rust; and the mutation `shadow-break-leaves`.
