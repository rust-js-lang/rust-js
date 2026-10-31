# 0750. A temporary is read past a method, and in the block after it

Status: Accepted. Extends [0689](0689-a-moves-decision-is-apart-from-the-move.md):
two more places `moves.rs` reads a temporary in its place. Amended below: a value that's the same anywhere is read past what a target does (#393); one that can't throw is read in the `try` after it (rd233).

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
JS runs the same code in the same order.

## Context

React's `addValueToProperties` and `addObjectDiffToProperties`
(shared/ReactPerformanceTrackProperties.js) push a pair whose second part
is a conditional, and test whether a value is an element before an arm
that ends in `continue`:

```js
properties.push([
  prefix + '\xa0\xa0'.repeat(indent) + propertyName,
  objectName === 'Object' ? (indent < 3 ? '' : '…') : objectName,
]);
if (readReactElementTypeof(nextValue) === REACT_ELEMENT_TYPE) { .. }
```

MIR makes the pair's first part before the blocks of its second, and the
test's `bool` before the blocks its arms' jumps leave. Tidy left both
held, and named them as it names a temporary it can't place:

```js
const value$1 = prefix + "\xA0\xA0".repeat(indent) + propertyName;
properties.push([value$1, objectName !== "Object" ? objectName : ..]);
const value = readReactElementTypeof(nextValue) === REACT_ELEMENT_TYPE;
if (value) { .. }
```

`moved` reads a call's value in its place only where all JS evaluates
before it there passes it. Of `properties.push([_487, ..])` that's
`properties.push`, a read of a property of a variable a call may change,
which it held back. And of `const _183 = ..; bb200: { bb291: { if (_183)`,
the statement that reads it was a block, which `read_of` reads nothing of.

## Decision

- **A method's call is its receiver, then its arguments:** what's
  evaluated before an argument of `out.push(..)` is `out`, and the other
  arguments before it. What JS looks up of the receiver, `push`, is the
  method its type has, which nothing the program does changes; MIR's own
  calls are written so, `out.push(f())`, the receiver read first.
- **A temporary the block after it reads, and nothing after that block,
  is read in the block**, as `moved` says of the block's own statements,
  through blocks it starts with too: entering a block runs nothing.

## Why

- **It's upstream's JS**: the pair's parts in the pair, and the test in the
  `if`, with no `value$1` and no `bool` held under a name the program
  never wrote.
- **One rule of where a value may be read**, `moved`'s, now asked of what
  a block holds as of what a list holds.

## Alternatives

- **Run `inlined` again after the chains**, where the blocks have gone: a
  second run of a pass whose first the passes between rely on, and still
  not the method's.
- **Every member read passes a call**: `out.length` doesn't, which a call
  that pushes changes.

## Consequences

- The port has neither `const`; `test/lowering-moves.test.ts` holds both.

## Amendment: not past a binding's method of a JS value (#187)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change.

"What JS looks up of the receiver is the method its type has" holds of
the methods rust-js writes, `out.push`, but not of a binding's method of
a JS value, `send(obj, effect(), ..)`'s `obj.send(..)`: JS looks it up
before the arguments run, where Rust runs them first, and a getter may
answer the lookup, or an argument replace the method. Read in place,
`obj.send(globalThis.effect(), ..)` looked up the method first, and
called the one an argument had replaced.

- **The binding's lowering says it**: a binding's method of a JS value is
  `js::Known::Getter`, as a binding's property read is (ADR 0672), where
  `binding_calls.rs` writes it.
- **`leads` reads nothing past it**: of a call of such a method, a
  temporary is read in place only where it's the receiver's; a call's
  value made before is held, `const effect = globalThis.effect();
  obj.send(effect, flag ? 1 : 2);`, and what does nothing is read in place
  still. The methods rust-js writes, of its types, are read past as
  before.
- Held by `test/lowering-moves.test.ts`'s "a binding's method is looked up
  after its arguments run", a getter's lookup and a method an argument
  replaces, and the mutations `binding-method-lookup-passed` and
  `binding-method-not-a-getter`. The corpus, the React ports and the
  bindings' tests write the same JS.

## Amendment: what a getter answers does something; a declared type's method is its class's (#188)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, for
a value of any shape; rare and harmless, for a declared type's method,
listed among the guide's differences (`semantics.md`, 23).

#187's rule held a call given to a binding's method, but not a getter's
read, `send(obj, value(source), ..)`: `obj.send(source.value, ..)` looked
`send` up before the getter ran, so the method saw the state before it.
Two places let it through: `Expr::has_effects`, the one answer to whether
an expression does something, took a getter's read for one that does
nothing, so tidy moved it as it moves a variable; and MIR, which writes
what it holds where it's read, put the read inside the call.

- **A read a getter may answer does something, and may give another value
  each time** (`js::Known::Getter`): `has_effects` says so, and
  `reads_same` says it isn't the same however often it's read, nested in
  a larger expression too, and MIR's `reads_same_anywhere` agrees: a
  getter's read a pattern binds is held once, not written at each use.
  What asks them, every pass, holds it so.
- **One answer of which method a getter may be**,
  `binding_calls::method_may_be_a_getters`: a binding's method of a
  `js::Unknown`, a value of any shape (`declarations::any_shape`). Its
  lookup is `Known::Getter`, and MIR makes what does something among its
  arguments before the call, `const value = source.value;
  obj.send(value, flag ? 1 : 2);`.
- **A declared type's method is its class's.** `app.append(stepper(..),
  ..)` of webapi's `Element` looks `append` up on the prototype, which
  nothing a program does is expected to change: holding every argument
  call before it, `const value = [..]; app.append(...value)`, would make
  most DOM code read unlike what anyone writes. That a getter or a
  replaced method there would be told apart is the guide's difference 23
  (the user chose this scope, 2026-10-24).
- **Read once still reads well:** `o.and_then(|l| l.item(0))` of a getter's
  read is `input.labels?.item(0)`, as `map`'s `o?.id` is
  (`optional_chain`, through method calls now). A let chain's tuple of reads,
  `(report.height, frame.current())`, reads each once, before its test,
  as Rust does, where it read `frame.current` only after the test, and
  both again.
- **An assignment target's object is read first**, then its key, then the
  value, so what makes it is read there whatever it does:
  `window.document.title = t`, `pick(items, log).count = v`, where `read_of`
  held what did something.
- Held by `test/lowering-moves.test.ts`'s "a getter's read given to a
  binding's method runs before its lookup" (the changed result, a getter
  that replaces the method, one nested in an argument), "a getter's
  Option a combinator reads more than once is read once",
  `test/jsx-children.test.ts`'s "a capture of an inlined closure leaves no
  const behind", and the
  mutations `getter-read-has-no-effects`, `getter-read-reads-same`,
  `getter-pending-past-lookup`, `typed-receiver-method-a-getters`,
  `and-then-chain-held`, `getter-read-in-place` and
  `target-root-acting-held`.

## Amendment: a value that's the same anywhere is read past what a target does (#393)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: what's moved does nothing, and reads only variables nothing
replaces, so it gives the same before the target's object is read as
after.

React's enqueueTask sets `channel.port1.onmessage = () => { callback(); }`.
Under a drop guard each of MIR's calls is in a `try` of its own, so the
closure is a temporary MIR made apart from the setter, and `read_of`
moved nothing into the value of an assignment whose target does
something, a getter's object: `channel.port1` is read first, so a value
that does something must stay where it ran. The closure stayed, and was
declared a function, `function value() { .. } channel.port1.onmessage =
value;`, which nobody writes.

**A `stable` value, one that does nothing and reads only variables
nothing in the body replaces, is read in an assignment's value whatever
its target does**, as it's read anywhere else in the statement that
reads it (`Why::Stable`): `channel.port1.onmessage = () => { .. }`,
`right[$at(right, 2)] = left.length | 0`, `f(left).value = [3, 4]`. What
does something stays where it ran, as before.

Four corpus snapshots read their value in place
(`generic_deref_mut`, `mut_assoc_through_dictionary`, `slice_views`,
`untagged_enums`). Held by `test/lowering-moves.test.ts`'s "a closure
given to a setter under a drop guard is written where it's given", which
runs it, the getter, then the guard's drop, then the closure, and the
mutation `stable-value-held-past-target`.

## Amendment: a value that can't throw is read in the `try` after it (rd233)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: entering a `try` runs nothing, and a value that can't throw
can't reach its `catch` or end its body early, so it gives the same, in
the same order, inside the `try` as before it.

React DOM's setFocusIfFocusable calls `(element.focus ||
HTMLElement.prototype.focus).call(element, focusOptions)` while a guard
that removes its listener is live. Each of MIR's calls is in the guard's
`try`, but `unwinding` writes what only makes values before it, as it
needs no `catch`: `element.focus`, which MIR held apart from the class
it's `||` of, became `const focus = element.focus;` before the `try`,
which `inlined` read nothing into, as `entered` went into a labeled block
only.

- **A temporary the `try` after it reads, and neither its `catch` or
  `finally` nor what follows does, is read in the `try`**, as `moved` says
  of its body, where its value can't throw (`Expr::may_throw`, the one
  answer `unwinding` asks too): `try { (element.focus ||
  HTMLElement.prototype.focus).call(element, focusOptions); } finally {
  .. }`. One that may throw stays where it was made, before the guard it
  ran before, which a panic in it doesn't drop: `const value =
  made(fail); try { return after(value); } finally { .. }`.
- Twenty-six corpus snapshots and std's `iter.js` read a value in its
  `try` (`const count = counter(5); try { .. count .. }`, `const _0 =
  branch._0; try { result = f(_0, v); }`), where nothing else changed.
- Held by `test/lowering-is-function.test.ts`'s "js::method of a path
  reads its method as a value" (its guarded form), the corpus case
  `value_read_in_a_guards_try` (a panic in a value made before its guard,
  and a value the `finally` drops, run natively), and the mutations
  `try-not-entered`, `try-entered-of-what-throws` and
  `try-entered-read-in-catch`.

## Amendment: a call isn't read past a getter's read (rd178, 2026-10-31)

React DOM's canHydrateInstance, as its port writes it, compares
`element.getAttribute("href")` to a choice of `js::get(any_props,
"href")`, `any_props` a struct of its own. MIR holds the call, then the
choice, `const _127 = element.getAttribute("href"); const _131 = .. ;
if (_127 === _131)`, and `moved` read the call where it's tested, past
the choice's `const`: a field of plain data reads the same past a call
(`read_past_calls`), so the choice passed it. But `js::get`'s read of a
name the struct doesn't declare is a getter's, which does something
(`has_effects`, #188): the getter ran first, and the call saw what it did.
`const unknown = anyProps.href == null || .. ; if (element.getAttribute(
'href') === unknown && ..)`, the `||` chain split around it, was the port's
JS; a getter that changes the attribute gave `true` where Rust's order
gives `false`.

**What does something isn't read past a `const` of what does something
too**: each may see what the other did. The choice stays after the call,
and is read where it's tested in turn, so the chain is as written:
`rel !== anyProps.rel || element.getAttribute("href") !== (anyProps.href
== null || anyProps.href === "" ? null : anyProps.href) || ..`. What
does nothing still passes a call where the call can't change it. Case
([0262](0262-when-rust-and-js-disagree.md)): C, results change. Held by
`test/lowering-moves.test.ts`' "a call isn't read past a getter's read
made after it", run with a getter that changes what the call reads, and
the mutation `call-moved-past-a-getters-read` (`mir/tidy/moves.ts`). No
corpus case's JS changes: a getter is JS's, which a native program has
none of.
