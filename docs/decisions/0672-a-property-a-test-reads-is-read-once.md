# 0672. A property a chain tests and binds is read once

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
- **A chain whose failures share an `else` reads it first**, where its
  `if` would otherwise leave for a label: `if (a) { const x = e; if (b) {
  s; break l; } }`, `b` a test of `x` alone that holds only of a value,
  is `const x = a ? e : undefined; if (b) { s; break l; }`, one test, whose
  failures are one `else` (`logicals::read_once_first`). `e` is read once,
  where `a` holds, as it was, and `b` of `undefined` fails, as the chain
  did. react.dev's `TerminalBlock`, `else if let ReactNodeKind::Element(element)
  = kind_of(&children) && let Some(inner) = js::get(element.props(),
  "children") && let Kind::String(text) = ..`, was `else if
  (isValidElement(children) && typeof children.props.children === "string")
  { const inner = children.props.children; ..}`, two reads, and is

  ```js
  } else {
    const inner = isValidElement(children) ? children.props.children : undefined;
    if (typeof inner === "string") {
      message = inner;
    } else {
      throw new Error("Expected plain text.");
    }
  }
  ```

  one read, no label. Chosen over `typeof (inner = children.props.children)`
  in the `else if`, which reads as Rust's let chain does but is an
  assignment in a test, which ESLint's `no-cond-assign` refuses and a reader
  takes for a typo; and over a label, `bb27: { .. break bb27; }`, which no
  person writes. A chain that returns, `getIODescription`'s, has no label
  to reach, and stays two `if`s. React's own reads the property twice;
  the port's Rust reads it once. `test/jsx-types.test.ts` and
  `test/lowering-bindings.test.ts` hold it, with the mutations
  `read-once-not-first` and `read-once-first-of-any-chain`.

## What holds it

- `test/lowering-bindings.test.ts`, "a property a let chain tests and
  binds is read once", whose getters count their reads in Bun.
- `test/lowering-switches.test.ts`, "React's getIODescription is
  upstream's switch of a typeof", whose `request` getters count theirs.
- The mutations `bound-property-read-twice`, `getters-unseen` and
  `bound-rust-data-a-getter`, which `test/lowering-loops.test.ts`'s
  "if-let chains and loops from MIR read as written" holds.

## Amendment: a read nothing after reads is gone (ADR 0701)

`returning_chain` kept each `const` it bound into a later test, though
nothing after the chain read it: `const value = v[i];` after `if (j ===
0 || v[i] !== w[j]) { return 1; }`. It drops those now, by the rule
`and_tests` had, one function both ask, `logicals::unread_bound`
([0701](0701-a-signed-index-tested-not-negative-is-a-js-index.md)).

## Amendment: the read says it's a getter's, not where it's written

Which read a getter may answer was told by where the program calls the
binding: the hints carried each such call's span, `Written::getters`,
and `bound_into` asked whether a property read's span lay in one. So a
pass that made a read anew had to keep its span for the read to stay
protected, a fact carried by where code came from (a review,
2026-10-24).

- **The expression says it**: `js::Expr::known`, `Known::Getter`, set where
  the binding's lowering writes the read, `get x`, `get X.y` and `get []`
  (`binding_calls.rs`, `answered_by_a_getter`). `bound_into` asks the
  expression; `Written::getters` and `bindings::reads_a_property` are gone.
- **One field for what an owner knows of an expression**: `Known` takes
  the place of `Expr::reads`, the `str` or slice method's call that only
  reads, `Known::Read`, so there aren't two. What's rebuilt of new parts,
  `substitute`'s and `negated_test`'s, keeps it, where `reads` was lost.
- The JS is the same, byte for byte. `test/lowering-bindings.test.ts`'s
  "a get binding's property a let chain tests and binds is read once"
  holds `get x`'s, with the mutation `get-getter-unseen`; `getters-unseen`
  holds `js::get`'s.

## Amendment: a getter's read held by MIR isn't written at each use (#189)

`plain_reads`, tidy's, writes a temporary MIR held of plain data, `const t
= q.title;`, as `q.title` where it's read, each time, and took a
binding's read a getter may answer for one: React's `child.type`, which
Rust read once, `classify(Some(element.r#type()))`, was read for the
test of its kind, `typeof child.type === "string"`, and again for what
its pattern binds. It asks `Expr::reads_same` now, which no read a getter
may answer is (ADR 0750): one read, before the test, as Rust's,
`const type = child.type; if (typeof type === "string" && ..)`. Held by
`test/jsx-children.test.ts`'s "a capture of an inlined closure leaves no
const behind" and the mutation `plain-read-of-a-getter`.
