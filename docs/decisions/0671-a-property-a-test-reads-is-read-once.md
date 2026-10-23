# 0671. A property a chain tests and binds is read once

Status: Accepted. Amends tidy's `and tests` and `returning_chain`, which
ADR 0614's chains of tests are joined by.

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results would
change: a getter that runs twice, where Rust's `js::get` runs it once, is
seen. Paid for in output: a `const` and an `if` of it, where the read was
in the test.

## Context

React's `getIODescription`, ported, tests a property of a property:

```rust
if let Some(request) = js::get(value, "request")
    && let Kind::Object(_) | Kind::Array(_) = js::classify(request)
    && let Some(Kind::String(url)) = js::get(request, "url").map(js::classify)
{
    return url.to_string();
}
```

`js::get(request, "url")` is called once, so a getter of `url` runs once.
It read

```js
if (request != null && typeof request === "object" && typeof request.url === "string") {
  const url = request.url;
  return url;
}
```

which reads `request.url` twice. Tidy's `and tests` joins `if (a) { const
x = e; if (b) { s } }` into `if (a && b) { const x = e; s }`, `b` reading
`e` in place, where `e` reads the same however often it's read
(`reads_same`): a variable, or a property of one. A JS object's property
needn't: a getter may run, as ADR 0356's amendment says of
`el.offsetWidth`, and may give another value, or throw. `returning_chain`,
which makes a guard of such a chain, read a bound value in its tests the
same way, and kept the `const` too.

Not on purpose: nothing says a property a binding reads is a value. ADR
0194 reads a field again only of plain Rust data, never of a JS object,
"a getter's `n.textContent` too"; ADR 0301 says "a getter may throw, as
`localStorage`'s does"; ADR 0356 keeps a `const` of a property nothing
reads, as its getter may do something.

## Decision

**A test reads in place a binding's read of a property bound before it
only where that's the one read of it.** One place answers it, tidy's
`logicals::bound_into`, which `and tests` and `returning_chain` ask:

- which read is a binding's is the binding's to say, `bindings::
  reads_a_property`, `get x`, `get X.y` and `get []`, `js::get`'s; the
  hints carry where the program calls one, `Written::getters`, by the span
  of its call, which its JS has, as the hints carry each `&&`;
- a `const` of anything else, a variable, a Rust struct's field,
  `node.count`, a tuple's item, `pair[0]`, is read in place wherever it's
  read, as before: no getter is Rust's data's;
- one of a binding's read is read in place where the test reads it once
  and nothing after reads it: the `const` goes, its read moved into the
  test, where it was read anyway, in the same order;
- otherwise the chain isn't joined there: `const url = request.url; if
  (typeof url === "string") { return url; }`, one read.

So the port reads

```js
const request = value.request;
if (request != null && typeof request === "object") {
  const url = request.url;
  if (typeof url === "string") {
    return url;
  }
}
```

and `const url = request.url; return url;` doesn't arise: `url` is read
by the test and the `return`, so it's a `const`. A test that binds
nothing, `let Some(Kind::String(_)) = js::get(request, "url")..`, is still
`request != null && typeof request.url === "string"`: one read.

## Why

- **The program's meaning is rustc's** (AGENTS.md): Rust calls `js::get`
  once, and a getter is JS's to run once for it, as a person writes
  `const url = request.url;` to read it once.
- **One place answers it**: both passes that read a bound value in a
  later test asked `reads_same` themselves; each now asks `bound_into`.

## Alternatives

- **Upstream's `typeof value.request.url === "string" ... return
  value.request.url`**: React reads each property twice, and three times
  `value.request`; the port reads each once, as its Rust says. Reading
  again would be the second read Rust doesn't make.
- **Plain values by name** (`Hints::plain`): by the names MIR's lowering
  gave them, which tidy's `named temporaries` has changed by the time
  chains are joined, `_5` of `const value = h > 0 ? [h, h + 1] :
  undefined`; and a struct through a `&mut`, `node.count` of a node the
  loop writes `node.seen = true` of, isn't plain, though no getter reads it.
- **Every read but a variable's, or every read by a name**: `value[0]` of
  a tuple and `node.count` of a struct read twice are Rust's data, as
  `examples/options.rs`'s snapshot and `test/lowering-loops.test.ts` have
  them, `node.count != null && node.count > 1`; they'd have been
  `const`s for nothing.

## Consequences

- The corpus, the snapshots and the guide's examples don't change: none of
  them binds a binding's read in a chain whose later test and body both
  read it. The guide's `interop/bindings.md` says it.
- React's `getIODescription` reads `request.url` and `response.url` once.
- **A chain whose failures share an `else` reaches it by a label** where
  its read is bound: react.dev's `TerminalBlock`, `else if let
  ReactNodeKind::Element(element) = kind_of(&children) && let Some(inner)
  = js::get(element.props(), "children") && let Kind::String(text) = ..`,
  was `else if (isValidElement(children) && typeof children.props.children
  === "string") { const inner = children.props.children; ..}`, two reads,
  and is `if (isValidElement(children)) { const inner =
  children.props.children; if (typeof inner === "string") { ..; break bb27;
  } } throw ..` in a block, one. React's own reads the property twice; the
  port's Rust reads it once. `test/jsx-types.test.ts` holds it.

## What holds it

- `test/lowering-bindings.test.ts`, "a property a let chain tests and
  binds is read once", whose getters count their reads in Bun.
- `test/lowering-switches.test.ts`, "React's getIODescription is
  upstream's switch of a typeof", whose `request` getters count theirs.
- The mutations `bound-property-read-twice`, `getters-unseen` and
  `bound-rust-data-a-getter`, which `test/lowering-loops.test.ts`'s
  "if-let chains and loops from MIR read as written" holds.
