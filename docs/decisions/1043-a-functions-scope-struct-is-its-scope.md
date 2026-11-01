# 1043. A function's scope struct is its scope: its methods are functions declared in it

Status: Accepted (the user's rd345 decision, B1, 2026-11-01). Amended
(B2, the same decision): its fields are the function's variables. Extends
[0047](0047-methods.md), whose object of a type's methods stays every
other struct's, and [0308](0308-local-functions.md), whose functions
declared in a body are written there.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference. Each call of the function makes the struct's one value and
its own functions closing over it, as each call of a JS function makes its
own variables and nested functions; nothing else ever holds the value, so
what each method is given as its `self` is always the value its own
function's call made.

## Context

React's ReactFlightReplyClient (react-client) writes `processReply` as one
function of about 840 lines, whose state is its variables, `nextPartId`,
`pendingParts`, `formData` and the rest, read and written by a dozen
functions declared in it, which call each other, `resolveToJSON` calling
`serializeModel`, whose `JSON.stringify(model, resolveToJSON)` calls it
back:

```js
export function processReply(root, formFieldPrefix, temporaryReferences, resolve, reject) {
  let nextPartId = 1;
  let pendingParts = 0;
  let formData = null;
  ..
  function resolveToJSON(key, value) { .. serializeModel(value, lazyId) .. }
  ..
  const json = serializeModel(root, 0);
  ..
}
```

Rust's closures can't call each other, nor themselves, but through a
cell made for the purpose; what an engineer writes is a struct of that
state, whose methods the functions are, with `self: &Rc<Self>` where a
closure a method makes holds the state (gap rd345). rust-js printed it as
ADR 0047 has every struct's methods, an object of functions at the
module's top, each given the state first, `Reply.serializeModel(reply,
root, 0)`, where upstream has nested functions reading the function's
variables.

## Decision

**A private struct whose one value a function makes, which only its own
methods and that function's closures hold, is the function's scope: its
methods are functions declared in the function, after the value's `let`,
each closing over the value, its `self`, and called by its name without
it.**

```rust
struct Reply { next_part_id: Cell<u32>, parts: RefCell<Vec<String>>, .. }

impl Reply {
    fn resolve_to_json(self: &Rc<Self>, key: &str, depth: u32) -> String {
        ..
        let inner = self.serialize_model(key, depth - 1);
        ..
    }

    fn serialize_model(self: &Rc<Self>, key: &str, depth: u32) -> String {
        let reply = Rc::clone(self);
        let later = move |k: &str| reply.resolve_to_json(k, 0);
        ..
    }
}

fn process_reply(root: &str, prefix: &str, depth: u32) -> String {
    let reply = Rc::new(Reply { .. });
    let json = reply.serialize_model(root, depth);
    ..
}
```

```js
function processReply(root, prefix, depth) {
  const reply = { prefix, nextPartId: 1, pending: 0, parts: [], settled: false };

  function resolveToJson(key, depth) {
    ..
    const inner = serializeModel(key, depth - 1);
    ..
  }

  function serializeModel(key, depth) {
    const later = (k) => resolveToJson(k, 0);
    ..
  }

  const json = serializeModel(root, depth);
  ..
}
```

One owner, `analysis/scopes.rs`, reads each body's MIR, the one it's
lowered from, and finds a struct a scope where:

- **It's its own**: not `pub`, of no type or const parameters, and no
  trait is implemented for it, so no `Clone` copies it, no `Drop` runs of
  it, and no generic code is given it. Its fields hold none of it.
- **Its impls are methods**: each item a method of `&self`, `&mut self`,
  or `self: &Rc<Self>` (an `Arc`'s too), of no type parameters, not
  `async`, given and giving nothing else of the struct, and no class's,
  object's or namespace's (ADRs 0761, 0943, 1037).
- **One value each call**: the crate writes one literal of it, in a
  function's own body, not generic, nor `async`, held by a `let` of that
  body's outermost block, which each call runs once, itself or in an
  `Rc::new`.
- **Nothing else holds it**: in that function, its methods and the
  closures made in them, what holds the value, a reference to it or its
  `Rc` is given to no call but its methods', as their `self`, and the
  `Rc`'s `new`, `clone` and `deref`; it's not given back, nor written but
  into another such variable or a closure made there. So no other body is
  ever given it. An `Rc` of it the crate counts (ADR 0320), `{ value,
  strong, weak }`, is no scope. Its methods are only called, never named
  as a value.

Lowering lowers each method as a body inside its function's, as it does
a closure's: its names apart from every one of the function's, which it
may read, its `self` the function's variable of the value, which no
parameter takes. Each method's function's name is reserved before the
function's variables are named, so none hides it (`fresh_in`). A call of
a method, from the function, another method or a closure in them, is a
call of its function, its `self` left out: a variable, which reading
does nothing. The functions are set apart by blank lines
(`Function::set_apart`), as a person sets apart a function declared among
statements.

## Why

- **It's upstream's JS**: processReply's functions, declared in it, read
  its state and call each other by name.
- **It's the Rust an engineer writes**: a struct of the state and its
  methods, `self: &Rc<Self>` where a closure holds it, as std's docs and
  the Rust book teach shared state; no cell of late-bound closures.
- **It's the same program**: whatever holds the value, wherever it's
  read, is the one its function's call made, and JS's lexical scope reads
  that call's: a closure made in one call and called from another reads
  its own call's value and functions, as Rust's reads the `Rc` it holds.
- **It's tested**: the corpus's `scope_struct_functions` runs a function
  twice against native Rust, a scope of `Rc<Self>` methods calling each
  other and a closure one makes, and one of `&mut self` methods; mutations
  leave out the conditions, the reserved names and the call's `self`.

## Alternatives

- **ADR 0047's object of methods**, as before: correct, but not what
  upstream writes, and each call names the state.
- **Fields as the function's `let`s** (B2, next): `nextPartId = id + 1`
  of `reply.nextPartId = ..`, as upstream has its variables; it builds on
  this, where nothing reads the value whole.
- **A form of nested `fn`s over a function's locals**: Rust has none; a
  closure of a cell of the others, `Rc<RefCell<Option<Rc<dyn Fn>>>>`, is
  Rust bent to fit.

## Consequences

- A struct that meets the conditions changes form wherever it is, not
  only in a port: its methods are no longer its module's object.
- A temporary of a method's result was named after the method, `const
  pop = Stack.pop(stack)`; the function has that name now, `pop()`, so
  the temporary is `value`, given apart where there are more, `value$2`
  (readability's `renamed x$1` grew 617.8 to 618.2 in 10,000 lines, and
  `$iter`'s share 84.5 to 84.6, of fewer lines, the objects gone). A
  variable or a parameter named as a method is given apart too, `resolve$1`,
  as the function's name is reserved first.
- A method given anything else of the struct, a second literal, a value
  passed on or returned, a trait impl, `pub`: the object of methods, as
  before.

## Amended (B2, the user's decision): its fields are the function's variables

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: nothing but its own functions and its function ever reads the
value, and each of its reads and writes is a field's, so a variable of each
field reads and writes as its property did.

Upstream's processReply keeps its state in its variables, `let nextPartId
= 1;`, `nextPartId++`, not in an object its functions read. **Where
nothing reads the scope's value but its fields, its literal is the
function's variables, one of each field, in the literal's order, and each
read or write of a field is its variable's**:

```js
function processReply(root, prefix, depth) {
  let nextPartId = 1;
  let pending = 0;
  const parts$1 = [];
  let settled$1 = false;
  const limit = depth;

  function resolveToJson(key, depth) {
    if (depth > 0 && depth <= limit) {
      const id = nextPartId;
      nextPartId = (id + 1) >>> 0;
      ..
```

- **`let` or `const`**: a field nothing writes, its property never
  assigned nor given a handle on, is a `const`; a `None` one written,
  `let head;`.
- **A parameter's own**: a field nothing writes whose value is a parameter
  the function never assigns, nor gives a handle on, nor declares again, is
  that parameter: `step` of `Counter { count: 0, step }` reads as `step`.
- **Named apart** from every name the function's body and parameters
  have, `parts$1` where the function has a `parts` of its own: a field
  read into a variable of its name, `let parts = reply.parts.borrow()`, is
  common, so readability's `renamed x$1` grew again, 617 to 619.4 in
  10,000 lines.
- **What holds it**: its `const`, and a `const` of that, which tidy left
  where it reads it, gone with it.
- **Where it's read whole**, by anything but a field's read, its JS keeps
  the object, as B1 has it: a counted `RefCell`'s field cell,
  `$fieldCell(tally, "n")`, reads the object, and a tuple struct's literal
  is an array, `[k, "x"]`.

`mir/scopes.rs`'s `scope_fields` makes them, of the function's body once
its own tidy has read it, so what it reads is the value's variable, the one
its lowering named, and the `const`s tidy kept of it. Each function's own:
a function with two scopes makes each's, not one of its methods' bodies.

Held by `test/corpus/scope_struct_functions.rs`, run against native Rust:
a field a parameter gives, read in a function whose own parameter has its
name, a parameter assigned after the literal, written fields `let`s; and
`scope_struct_read_whole.rs`, whose objects stay. Mutations in
`scripts/mutations/lower/mir/scopes.ts`.

