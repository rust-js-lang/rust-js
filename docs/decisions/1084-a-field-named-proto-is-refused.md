# 1084. A field whose JS name is `__proto__` is refused

Status: Accepted (rd754). Amends [0020](0020-structs-and-tuples.md): a
struct's, a variant's or a union's field is a property of its JS name, but
never `__proto__`. Amends [0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md):
`class(es, fields)` no longer asks it of the fields it declares.

Case ([0262](0262-when-rust-and-js-disagree.md)): D. Results change where
the field isn't its object's own, and keeping Rust's would take each place
a field is written to define it instead, a constructor's
`Object.defineProperty(this, "__proto__", { .. })`, which no one writes: a
compile error, saying what to write.

## Context

`__proto__` is a Rust name as any other: `struct Linked { __proto__:
Option<u32>, size: u32 }` compiles natively. Its JS name is its own,
`__proto__` (ADR 0573 keeps leading and trailing underscores), and an
object's `__proto__` is `Object.prototype`'s accessor, whose set makes the
value the object's prototype and whose read gives the prototype. Only an
own property of that name, which a literal makes of a quoted or computed
key, `{ ["__proto__"]: v }`, as rust-js has printed one since structs were
objects (ADR 0020), hides it. Every other place a field is made or written
went through the accessor:

- a struct's `None` is left out (ADR 0280), so a read of the field gave
  `Object.prototype`: `Linked::default().__proto__.is_none()` was `false`,
  and its `Debug` printed `Some([object Object])`;
- a class's constructor sets its fields, `this.__proto__ = proto;` (ADR
  0761), an ES class's too: an instance's `proto()` gave its class's
  prototype, `Some([object Object])` where Rust prints `None`, `Some(3)`
  or `4`;
- `*shape = Shape::Linked { __proto__: Some(8), .. }` of another variant
  is `$assign`'s `Object.assign` (ADR 0147), a union's write is
  `$unionSet`'s `union[key] = value` (ADR 0406), a copy apart is
  `$apart`'s `Object.assign` (ADR 0074), and a flattened keyed enum's
  write deletes its keys and sets its own (#504): each set the prototype.

A wrong answer (P0), found by review, no program of the corpus or the
ports having such a field.

## Decision

**A field of the crate's structs, enums' variants and unions whose JS name
is `__proto__`, its Rust name's or a `#[rust_js::name = "__proto__"]`'s,
is refused at the field**, saying which and what to write:

```text
error: rust-js: a field whose JS name is `__proto__`, `__proto__` of `Linked`, sets or reads its object's prototype where it isn't the object's own, through `Object.prototype`'s accessor: name it otherwise, or give it another JS name, `#[rust_js::name = ".."]` (rd754, ADR 1084)
```

`#[cfg_attr(rust_js, rust_js::name = "proto")] __proto__: Option<u32>`
compiles, its JS `proto`, and runs as natively, however the struct is made,
set, copied or replaced. The rule is the field key's owner's,
`bindings.rs`'s `prototype_key_refused` beside `field_key`, asked of every
field `bindings::validate` checks, so a struct, a variant, a union and a
class of either form are refused alike. `class(es, fields)`'s own check
of a field it declares no longer asks it, as it no longer asks
`constructor` (rd750).

What's JS's stays JS's: a binding's member named `__proto__`, a JS
object's, is read and set as JS does; a `HashMap<String, _>`'s
`"__proto__"` key is a `Map`'s (ADR 0059); a dictionary's, `Dict`'s, is its
object's own, `Object.fromEntries`'s, never written out as a literal's key
(ADRs 0666, 0967); and the printer still quotes a literal's `__proto__` key,
`["__proto__"]`, of any object it prints.

## Why

- **The field's own is what Rust reads.** Through the accessor it's the
  prototype, not the value, so a program that compiles natively printed
  and compared other values.
- **No one place could keep it.** Defining rather than setting the field
  would take the `None` left out, a constructor's assignment, `$assign`,
  `$apart`, `$unionSet` and the flattened enum's write each to ask of the
  name, five places knowing one rule, and a constructor's
  `Object.defineProperty(this, "__proto__", { value: proto, writable:
  true, enumerable: true, configurable: true })`, which no one writes.
  A field written later would need the same.
- **The fix is one attribute.** The field keeps its Rust name, and its
  JS name is what it says; no port or program has met one.

## Alternatives

- **Keep it, defining the field everywhere it's written.** Correct where
  every writer is found, and silently wrong where one isn't; the output a
  constructor's `Object.defineProperty`. Rejected.
- **Rename it, `__proto__$`, as std's iterator struct's `count` is
  `count$` (ADR 0912).** A name no one asked for in a `.d.ts` and an
  object JS reads, where an attribute says which name is wanted, and a
  `#[rust_js::name = "__proto__"]` still to refuse. Rejected.
- **Refuse a class's alone, as `constructor` is (rd750).** A plain
  struct's `None` and a variant's write are wrong too. Rejected.

## Consequences

A Rust program with a field named `__proto__` names it for JS. Held by
the corpus's `field_named_proto.rs` (`compile-fail`, its `Default` and
`None`), `field_named_proto_renamed.rs` (the remedy, run natively and in
Node: a struct's `None` left out, cloned, updated, replaced by `*r = ..`
and `mem::replace`; a variant written of another; a union's write; a
class's constructor and setter; a `HashMap`'s `"__proto__"` key), and
`test/diagnostics.test.ts`'s refusals of a struct's field, a field given
the name, a variant's, a union's, a class's and an ES class's; mutations
`proto-field-unrefused` and `proto-field-refused-by-its-rust-name`
(`lower/bindings.rs`).
