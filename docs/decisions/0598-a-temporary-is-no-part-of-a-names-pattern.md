# 0598. A temporary is no part of a named variable's pattern

Status: Accepted. Amends tidy's destructuring
([0293](0293-cells-read-at-once.md)). Amended (2026-10-26): a binding the
program names of a variable rust-js named, `old` of `take()`, names both,
a `let`-`else`'s too;
no pattern binds a name rust-js gave. Amended (2026-10-27, the user's
decision): `let`s side by side are `const`s side by side, as written;
only a pattern destructures, which supersedes "`let`s side by side, each
named as its part, are one destructuring".

Case ([0262](0262-when-rust-and-js-disagree.md)): A: one pattern or two
reads of the same properties, in the same order.

## Context

React's scheduler sets up its channel:

```js
const channel = new MessageChannel();
const port = channel.port2;
channel.port1.onmessage = performWorkUntilDeadline;
```

Its Rust, `let port = channel.port2(); channel.port1().set_onmessage(..)`,
became

```js
const { port2: port, port1: _12 } = channel;
_12.onmessage = ..
```

Two parts of a variable the program names, read in a row, are one
pattern, `let (a, b) = t;`'s. But the second was MIR's temporary for
`channel.port1()`, which the program never named, and would have been read
in place.

## Decision

**Of a variable the program names, a pattern takes the parts it names**:
a temporary among them ends the run, and is read where it's used,
`channel.port1.onmessage = ..`. A temporary's own parts, `const [a, b] =
f();`, are as before.

## What holds it

- `test/lowering.test.ts`, "a temporary is no part of a named variable's
  pattern".
- Mutation: a temporary's part kept in the pattern.

## Amendment: a `let` of a name alone is its own statement, and a cast's is the variable

React's `mapIntoArray` (ReactChildren.js) reads a lazy component's parts
of `children` as it is, `const payload = (children as any)._payload;
const init = (children as any)._init;`. Its Rust,

```rust
let lazy: &LazyComponent = unsafe { js::cast(children) };
let payload = &lazy.payload;
let init = lazy.init;
```

was `const lazy = children; const { _payload: payload, _init: init } =
lazy;`: two parts read in a row were one pattern, though the program
wrote two `let`s, and the cast a name JS has no use for.

**A pattern takes the parts its program's pattern names**: a `let` of a
name alone, `let x = p.x;`, ends the run, a statement the program wrote as
one, `const x = p.x;` (`LetNames::named`, of THIR's `let`s). MIR's
temporary of a part, as `channel.port1()`'s, followed only a `let`'s
name, so that rule is this one now. A `let`, a parameter's or an arm's
pattern is one destructuring, as before. **A `let` of a variable cast**,
`js::cast`'s or any binding that's its argument (`JsForm::This`,
`LetNames::casts`), **is that variable** where nothing reads it after and
nothing after sets either: `children._payload`, as TypeScript reads it;
else it keeps the name the program gave. Case N: the same reads, in the
same order.

- `test/lowering-parts.test.ts`, "a let of a part is its own statement,
  and a cast's variable is the variable", run.
- Mutations `named-let-part-of-a-pattern` (`tidy/destructuring.rs`),
  `cast-variable-read-after` (`tidy/bindings.rs`), `cast-let-named`
  (`mir/patterns.rs`).

## Amendment: a binding the program names of a variable rust-js named

React's ReactFiberLane, `markRootFinished`, takes each lane's hidden
updates:

```js
const hiddenUpdatesForLane = hiddenUpdates[index];
if (hiddenUpdatesForLane !== null) {
  hiddenUpdates[index] = null;
```

Its port's `if let Some(hidden_updates_for_lane) =
hidden_updates[index].take()` printed `const old = hiddenUpdates[index];
hiddenUpdates[index] = undefined; if (old) { for (const update of old)`:
`take()` holds what it gives in a variable rust-js names `old`, the
binding is a copy of it, `const hiddenUpdatesForLane = old;`, and tidy's
`copied bindings` read the copy as the variable, by the variable's name.

**Where the variable is rust-js's, no local the program names (`Named::
user`), declared in the statements the `if` is in, and the binding's name
is the program's and nowhere else in them, both are the binding's**:
`const hiddenUpdatesForLane = hiddenUpdates[index]; .. if
(hiddenUpdatesForLane) { for (const update of hiddenUpdatesForLane)`
(`bindings.rs`'s `named_for_bindings`). One variable renamed in its own
scope, to a name nothing else there has: case N, nothing that runs changes.

Test: `lowering-shadows`'s "a binding of what take() gives names it", run.
Mutation `taken-binding-named-old` (`lower/mir/tidy/bindings.ts`).

**A `let`-`else`'s binding too** (amended 2026-10-26): React's
`getSuspendedThenable` is `const thenable = suspendedThenable;
suspendedThenable = null; return thenable;`, and its port's `let
Some(thenable) = SUSPENDED_THENABLE.take() else { panic!(..) }; thenable`
printed `const old = suspendedThenable; .. return old;`: the `let`-`else`'s
binding, after the `if` that leaves, read as the variable by the
variable's name. Where the variable is rust-js's, declared before the
`if`, and the binding's name is the program's and read only after it,
both are the binding's, `const thenable = suspendedThenable; ..
if (thenable == null) { throw .. } return thenable;`, as an `if let`'s
are; a variable the program named keeps its name. Case N, as above.
Test: `lowering-shadows`'s "a let else's binding of what take() gives
names it", run. Mutation `let-else-binding-unnamed`
(`lower/mir/tidy/copied.ts`).

## Amendment: `let`s side by side, each named as its part, are one destructuring

React's getComponentNameFromFiber writes `const {tag, type} = fiber;`; its
port reads each `Cell`, `let tag = fiber.tag.get(); let r#type =
fiber.r#type.get();`, each a field that's its property (ADR 0863), and
printed two `const`s, as a `let` of a name alone is its own statement
(above). **A run of `const`s side by side in a body, each named as the
property it reads of one variable, is one destructuring of it**, `const
{ tag, type } = fiber;`, as React writes it and as a module's constants
side by side are (ADR 0031's `read_together`, `prepare.rs`'s
`read_together_in`), a plain field's and a cell's alike: the JS each is
is a property read. One named otherwise stays its own statement, `const
returnFiber = node.return;`, as mapIntoArray's `const payload =
children._payload;` does, which a pattern would rename. Case N: each
property is read once, in its order, either way. `sum`'s `let x = p.x;
let y = p.y;` is `const { x, y } = p;` now; std's `rfold`s' `const { iter,
f } = map;`, `math_values.rs`'s `const { atan2, hypot } = Math;` and
`static_mut.rs`'s `const { calls, last } = STATS;` read so. Held by
`test/lowering-field-cells.test.ts`, "lets side by side of an object's
cells, each by its name, are one destructuring", run in JS,
`test/lowering-parts.test.ts`, and the mutations `body-reads-apart` and
`body-reads-renamed-together` (`prepare.ts`).

## Amendment: a tuple assigned to variables is a destructuring assignment (#331)

`(a, b) = if a > b { (a - b, b) } else { (a, b - a) };` read `const [_18,
_19] = a > b ? [a - b, b] : [a, b - a]; a = _18; b = _19;`, MIR's
temporaries by their names. **A pattern of temporaries each assigned, in
its order, to a variable the program has, and read nowhere else, is one
destructuring assignment**, `[a, b] = a > b ? [a - b, b] : [a, b - a];`,
as trekhleb/javascript-algorithms' `euclideanAlgorithmIterative` writes it:
JS makes the right side whole before it sets any, as the temporaries did.
After "destructured declared", which makes the pattern ("assigned parts").
Held by the corpus case `tuple_assigned_to_variables` and the mutation
`assigned-parts-left-apart`.

## Amendment: no pattern of names rust-js gave (2026-10-26)

React's enqueueCapturedUpdate clones each update in place, `const clone =
{ lane: update.lane, tag: update.tag, payload: update.payload, .. }`. Its
port writes `Cell::new(update.lane.get())` of each field, and MIR's
lowering reads the first two ahead of the third's read into `const`s it
names after the `{ value }` each cell is (`capture_object`): tidy's
destructuring, which takes two parts of a variable the program names in a
row as one pattern, made them `const { lane: value, tag: value$1 } =
update;`, a pattern of names no one wrote, and one case printed `_22`
the same way. This page's rule, that a pattern binds only names a reader
would, was `read_together`'s alone.

**A pattern of a variable the program names binds only the program's own
names**: each part, `let (a, b) = t;`'s `a` and `b`, a local the program
names (`Locals::user`). Where one isn't, a temporary or what
`capture_object` named, the parts end there, and each `const` stays what
it is, which tidy's moves read where it's used: `{ lane: update.lane, tag:
update.tag, payload: update.payload }`, as React writes it. Where it's
read more than once and kept, it's its own `const`, which preparation's
`read_together` still destructures where each is named as its property.

- Case A still: each property is read once, in its order, either way.
- What changes: todomvc's `Edit` reads `const captured = action._0;` as
  its `Destroy` does (and so `todos.map(..)`, which the pattern had
  kept from reading in place); `mut_ref_local.rs`'s `const list$2 =
  h.list; const node = h.node;`, `replace_held_given_iterator.rs`'s
  `captured`s and std's `enumerateIterator_fold`'s `count` are two
  `const`s, as each is named otherwise than its property.

Held by `test/lowering-field-cells.test.ts`, "a struct literal of cells
another's read makes reads each in place", run in JS, and the mutation
`rust-js-names-in-a-pattern` (`lower/mir/tidy/destructuring.ts`).

## Amendment: `let`s side by side are `const`s side by side, as written (2026-10-27)

The user's decision (r10 of the React port's gaps). React's
createFiberFromElement writes each part of the element as its own
`const`:

```js
const type = element.type;
const key = element.key;
const pendingProps = element.props;
```

Its port's three `let`s printed `const { type, key } = element; const
pendingProps = element.props;`: preparation's `read_together_in` made a
run of `const`s side by side, each named as the property it reads, one
destructuring (the amendment above, for getComponentNameFromFiber's
`const {tag, type} = fiber;`). The program wrote three statements.

**A destructuring is a pattern the program wrote, `let Fiber { tag,
r#type, .. } = fiber;`, its `Cell` fields' too (ADR 0863); `let`s side by
side are `const`s side by side, each its own statement, however they're
named.** `read_together_in` is gone. A module's constants side by side,
`const P = MDXComponents.p;` (ADR 0031's `read_together`), are as before:
they aren't `let`s.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, each property is
read once, in its order, either way.

- **Why**: one rule a reader can see in the Rust: a pattern is a
  destructuring, a `let` of a name a `const`. Measured on the React port
  before deciding: of the 17 runs `read_together_in` destructured, upstream
  writes 13 as separate `const`s (ReactFiberLane's `const pendingLanes =
  root.pendingLanes; ..` six times, createFiberFromElement,
  ReactFiberCommitEffects, ReactFiberCommitHostEffects, ..) and 4 as one
  destructuring (getComponentNameFromFiber, ReactFiberHotReloading,
  ReactFiberScope twice), which the port now writes as patterns.
- **Alternatives**: keep the run's destructuring, and accept 13
  differences from upstream; or tell them apart by something the Rust
  doesn't say.
- What changes: the corpus's and the guide's runs of `let`s each named as
  its field, `sum`'s `let x = p.x; let y = p.y;`, `math_values`'
  `const sqrt = Math.sqrt; const atan2 = Math.atan2;`, std's `rfold`s'
  `const iter = map.iter; const f = map.f;`, and `static_mut.rs`'s reads,
  are `const`s side by side again.
- Held by `test/lowering-field-cells.test.ts`, "lets side by side are
  consts as written, a struct's pattern one destructuring", run in JS,
  and `test/lowering-parts.test.ts`, which `named-let-part-of-a-pattern`
  (`lower/mir/tidy/destructuring.ts`), a `let` of a name joining a pattern,
  fails both. The mutations `body-reads-apart` and
  `body-reads-renamed-together` went with `read_together_in`.
