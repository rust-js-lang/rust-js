# 0805. A case's `if` is as written, whatever its other branch ends with

Status: Accepted. Amends
[0790](0790-a-break-the-program-wrote-is-where-it-is-written.md), whose
`||` of a `break` the program wrote was as written only where the rest
ended in a `return`, and whose case's `if` the program wrote with no
`else` read as an `else if`.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
which branch of an `if` is first, and whether one has an `else` after a
branch that leaves, are how the JS reads, not what it does.

## Context

ADR 0790 made React's `addValueToProperties` a `switch (typeof value)`
with no label. Two of its JSX branch's shapes still weren't upstream's:

```js
// Upstream:                                // The port's:
case 'object':                              case "object":
  if (value === null) {                       if (value === null) {
    desc = 'null';                              desc = "null";
    break;                                    } else if (readReactElementTypeof(value) === REACT_ELEMENT_TYPE) {
  } else {                                      // ..
    if (readReactElementTypeof(value) ===       if (indent >= 3 && (propsLength !== 1 ||
        REACT_ELEMENT_TYPE) {                       propsKeys[0] !== "children" || key != null)) {
      // ..                                       // ..
      if (                                        return;
        indent < 3 ||                           }
        (propsLength === 1 &&                   desc = "<" + typeName + " … />";
          propsKeys[0] === 'children' &&        break;
          key == null)                        } else {
      ) {                                       // $FlowFixMe[method-unbinding]
        desc = '<' + typeName + ' … />';
        break;
      }
      // ..
      return;
    }
    // $FlowFixMe[method-unbinding]
```

The port's Rust is written as upstream is: `if matches!(..) { ..; if
indent < 3 || (..) { break 'desc ..; } ..; for prop_key in .. { .. } ..;
return; } let object_to_string = ..`, an `if` with no `else`.

- **The `||` was negated by a loop.** MIR's `l: { if (!a) { if (b) {
  break l; } r } } s` is the program's `if (a || b) { s } r`, which
  `or_blocks` writes as written. But `branch ends`, which takes `if (c) {
  s; break l; } t` for an `if` and `else` that both go on to the block's
  end, `if (c) { s } else { t }`, does so where `t` ends in a loop MIR
  made: a `while (true)` nothing leaves, React's `for (const propKey in
  props) { .. }` then the `return` it goes on to. It already kept the
  guard where `t` ends in a `return` or a `throw` (`let … else`'s); a loop
  that never goes on is one more way `t` leaves. So the block was gone
  before `or_blocks` could read it, and the test read `if (indent >= 3 &&
  (..)) { ..; return; } desc = ..; break;`.
- **An `else` no one wrote, then an `else if`.** The object arm's `'desc`
  block ends where the case does. Its JSX branch's `break 'desc`s went on
  to that end, as the code after the `if` did, so the `if` had an `else`,
  MIR's: `if (jsx) { .. } else { .. }`. `blocks ending cases` (ADR 0790)
  made each `break 'desc` the case's `break;`, and the JSX branch then
  never went on, but the `else` stayed; with the block gone, the `if` was
  all the `else` of `if (value === null)` held, and it printed as an
  `else if`, which preparation keeps as a chain (ADR 0683).

## Decision

- **A loop nothing leaves ends a list as a `return` does** (`branch_ends`,
  `exits`): of `if (c) { s; break l; } t` where `t` ends in a `while
  (true)` no `break` leaves, the guard stays, as of one that ends in a
  `return`, and `or_blocks` reads the program's `||` as it wrote it:
  `if (indent < 3 || (propsLength === 1 && propsKeys[0] === "children" &&
  key == null)) { desc = ..; break; }`, of any number of parts.
- **A case's `if` the program wrote with no `else` has none once its
  `then` leaves** (`switches::blocks_ending_cases`, `unwritten_else`):
  where the block that ended a case is the case's `break;`s, an `if (c) {
  a } else { b }` that ends it, `a` now leaving, of an `if` that isn't one
  of the program's with an `else` (`Written::elses`), is `if (c) { a } b`,
  as written. An `else` the program wrote stays, an `else if` of it a
  chain (ADR 0683); so does one whose `b` declares a name already declared
  beside the `if`.

The port reads:

```js
case "object":
  if (value === null) {
    desc = "null";
  } else {
    if (readReactElementTypeof(value) === REACT_ELEMENT_TYPE) {
      // ..
      if (indent < 3 || (propsLength === 1 && propsKeys[0] === "children" && key == null)) {
        desc = "<" + typeName + " … />";
        break;
      }
      // ..
      return;
    }
    // $FlowFixMe[method-unbinding]
    // ..
    return;
  }
  break;
```

## Alternatives

- **Preparation's `unelsed` (ADR 0237) for it**: it keeps an `else if`
  chain whose arm before goes on, rightly of one the program wrote, and
  can't tell this one, which the program didn't, from it. What was
  written is tidy's to know (`Written::elses`).
- **Upstream's `break;` in the `null` branch**, `if (value === null) {
  desc = 'null'; break; } else { .. }`: the port's `match` arm `Kind::Null
  => "null".to_string()` goes on to where the `match` ends, the case's
  end, which the `switch`'s `break;` after the `if` is; a `break` in the
  branch is one the program didn't write. Not taken; the one difference
  left in the case.
- **`or_blocks` of the `if`s `branch ends` makes**: by then `if (!a) {
  if (!b) { r } }` doesn't say which of `r` and `s` the program wrote in
  its branch; the block MIR made, kept, does.

## Consequences

- The guide's `describe` (interop/global-scope) reads as its Rust, `}
  else { if ("name" in value) { desc = ..; break; } desc = "object"; }`,
  not `} else if ("name" in value) { ..; break; } else { desc = "object";
  }`.
- Held by `test/lowering-labels.test.ts`'s "an || before a loop whose
  branch leaves a case is as written" and "a block the program labeled
  that ends a case is the case's breaks", and
  `test/port-performance-track.test.ts`'s "addValueToProperties' JSX
  branch is as written"; mutations of `exits` and `unwritten_else`.
