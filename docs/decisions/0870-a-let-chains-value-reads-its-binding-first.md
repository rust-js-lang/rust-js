# 0870. A let chain's value reads its binding first, then is a conditional

Status: Accepted. Amends [0601](0601-a-module-variable-an-if-makes-is-a-let.md),
whose thread-local's closure that declares at its top stayed a call, and
[0672](0672-a-property-a-test-reads-is-read-once.md), whose chain read
first was a getter's alone. Extends [0657](0657-an-if-of-one-value-each-is-a-conditional.md)'s
`if` of one value each to a let chain's.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: the same code runs, in Rust's order, once each.

## Context

React's `createTask` is upstream's

```js
const createTask =
  // eslint-disable-next-line react-internal/no-production-logging
  __DEV__ && console.createTask
    ? // eslint-disable-next-line react-internal/no-production-logging
      console.createTask
    : () => null;
```

and its port's a thread-local of a let chain, `console.createTask` read
once, as Rust does, through a binding a getter may answer:

```rust
thread_local! {
    static CREATE_TASK: Box<CreateTask> =
        // eslint-disable-next-line react-internal/no-production-logging
        if __DEV__ && let Some(create_task) = console::create_task() {
            // eslint-disable-next-line react-internal/no-production-logging
            Box::new(move |name| Some(create_task(name)))
        } else {
            Box::new(no_task)
        };
}
```

MIR's chain is two `if`s whose failures share one `else`, the `let` they
assign a value made in a block, which a thread-local's initializer, a
closure, returns:

```js
const createTask = (() => {
  // eslint-disable-next-line react-internal/no-production-logging
  let tmp;
  if (__DEV__) {
    const createTask = console.createTask;
    if (createTask) {
      // eslint-disable-next-line react-internal/no-production-logging
      tmp = (name) => createTask(name);
      return tmp;
    }
  }
  tmp = noTask;
  return tmp;
})();
```

A plain `if let`'s value is a conditional already, `const k = lookup(n);
const v = k != null ? k * 2 : 7;` (ADR 0657): one test, one value each.
The chain's isn't, as its `else` is reached from two tests, and 0601 kept
the closure, which declared `tmp` at its top and returned before its end.

## Decision

**A let chain of one binding whose two branches are each one value, `if a
&& let Some(x) = e { t } else { f }`, reads its binding first, `const x =
a ? e : undefined;`, and is then one test of it and two values, `x !=
null ? t : f`, as an `if let`'s is** (`logicals::chain_values`, in tidy's
`jumps` stage, before the passes that make a plain `if let`'s value one):

```js
const k = flag("doubled", on) ? lookup("doubled", n) : undefined;
const v = k != null ? (k * 2) >>> 0 : 7;
```

The same order: `a`, then `e` only where `a` holds, once, then `t` or
`f`. Where `a` fails, `x` is `undefined`, and the test of it fails as the
chain did: it's a test of being there, `x != null`, or `x`'s truth where
its payload is never falsy (`implies` now says `x` implies `x != null`:
`null` and `undefined` are falsy). The one rule of when a chain's binding
may be read first is `logicals::read_first`, which 0672's getter read and
this ask alike. The same of a value the function returns, `return k !=
null ? (k + 100) >>> 0 : n;`, and of tests before it, `a && b ? e :
undefined`.

It stays the chain's two `if`s:

- **a test `undefined` can't be asked**, `parse.TAG === "Ok"` of `let
  Ok(n) = ..`, which would throw where `a` fails;
- **a branch that's not one value**: one that does something first, one
  whose value is branches of its own (`small`), and an `Option`'s `None`,
  the guard of its `Some` (`folds_into_return`);
- **a `return` the program wrote**, a statement;
- **more than one `let`**, a chain of chains.

A field's read MIR makes no `const` of, `if on && let Some(w) =
settings.width`, is one test as before, `on && settings.width != null ? ..`.

**And a module's `const` a closure called where it's made makes, whose
body is `const`s and then the value it returns, is those `const`s and the
value, the module's** (`prepare::initializers::declared_at_top`, amending
0601): the same code, run once, in its order, as the module loads.

```js
// eslint-disable-next-line react-internal/no-production-logging
const createTask$1 = __DEV__ ? console.createTask : undefined;

const createTask = createTask$1
  ? // eslint-disable-next-line react-internal/no-production-logging
    (name) => createTask$1(name)
  : noTask;
```

Each such `const` is named apart from what the module names elsewhere,
`createTask$1` of the thread-local's own `createTask`, as JS declares a
name once and a module's `const` would be what another item's read of
that name reads: a name given apart, as a shadow is
([0352](0352-reclaimed-names.md)).

## Why

- **One value, written as one**: as a person writes a value an `if` gives,
  0657's rule, now of every `if` of one value each, its test a chain's too.
- **Read once, where Rust reads it**: `console.createTask` once, as a
  getter may answer it; no IIFE, no `tmp`.
- **A module's top as JS writes one**: upstream's module is its `const`s,
  not a closure called to make one.

## Alternatives

- **`a && e`**: `false`, not `undefined`, where `a` fails, which `x !=
  null` holds of: a wrong answer.
- **Reading `e` twice, `a && e ? e : f`**, as upstream reads
  `console.createTask`: a second read Rust doesn't make, a call's run
  twice.
- **An assignment in the test, `(x = e) != null`**: ESLint's
  `no-cond-assign` refuses it.
- **Only a getter's read first (0672)**: a call's, `lookup(n)`, kept the
  chain's two `if`s and `tmp`, though its value is one.
- **Hoisting only the closure a let chain made**: one rule of a closure of
  `const`s and a value is the one there is to tell; `const at =
  Math.random(); const SEEN = at < 2 ? 1 : 2;` is the module's as
  React's is.

## Consequences

- No corpus case, snapshot or guide example changed but those this adds:
  a let chain's value of a call was only React's, and a thread-local's
  closure of `const`s and a value only `test/lowering-cells.test.ts`'s.
- A name given apart may be where the module names it elsewhere only as
  another function's local, `at$1`: the module's mentions, as reclaiming
  reads them (ADR 0352), say what's taken.

## What holds it

- Corpus case `let_chain_values` against native Rust: a `let`'s, a
  function's, two tests' and a truth test's values, a branch reading a
  variable of the binding's name, a thread-local made where it's used and
  three made as the module loads, two of one binding's name; and what
  stays, a branch that logs, one of branches, a written `return`, `None`,
  an `Ok`'s test where `on` fails.
- `test/lowering-bindings.test.ts`, "a thread-local of a let chain's value
  is the module's consts": React's `createTask`, its getter read once, and
  `noTask` where the host has none; and "…, named apart" of a binding's
  global.
- `test/lowering-cells.test.ts`, "a thread-local an if makes is a let each
  branch assigns": `SEEN`'s `const`s the module's, `KEPT`'s `let` a call.
- Mutations `chain-values-nested`, `chain-read-first-of-any-test`,
  `chain-values-of-branches`, `chain-values-of-none`,
  `chain-values-of-written-return` (`lower/mir/tidy/logicals.ts`),
  `truth-implies-nothing` (`lower/mir/tidy/branches.ts`),
  `declared-at-top-off` (`prepare.ts`) and `declared-at-top-not-apart`
  (`prepare/initializers.ts`).
