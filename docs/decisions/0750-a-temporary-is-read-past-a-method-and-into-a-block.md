# 0750. A temporary is read past a method, and in the block after it

Status: Accepted. Extends [0689](0689-a-moves-decision-is-apart-from-the-move.md):
two more places `moves.rs` reads a temporary in its place.

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
