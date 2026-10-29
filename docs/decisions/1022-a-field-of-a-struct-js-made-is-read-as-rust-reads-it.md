# 1022. A field of a struct JS made is read as often as Rust reads it

Status: Accepted (the user's decision, rd204). Amends
[0194](0194-jsx-attributes-read-once.md),
[0672](0672-a-property-a-test-reads-is-read-once.md) and
[0866](0866-a-let-only-its-if-let-reads-is-read-there.md): a field of a
struct is plain data, read again where that reads the same, only where
the program built the struct; one JS made is a property a getter may
answer, as a binding's read is.

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results would
change. A getter on an object JS made runs once for each read: read twice
where Rust reads it once, it may give another value, count its reads, or
throw. Paid for in output: a `const` of the field where it was read in
place.

## Context

React DOM's `restoreViewTransitionName`, ported, reads the style prop as a
struct `js::cast` gives, a JS object the program didn't build:

```rust
let style_prop: Option<&StyleProp> = unsafe { js::cast(js::get(props, STYLE)) };
// ..
let margin = style_prop.margin;
if margin.is_some() {
    style.set_margin(margin);
}
```

It printed `if (styleProp.margin != null) { style.margin =
styleProp.margin; }`: tidy reads in place a `let` of a place, where the
place reads the same however often it's read (ADR 0866), and a struct's
field was such a place, `Expr::has_effects` false of a member of a
variable that no binding's getter reads (ADR 0672). ADR 0194 had said a
field is read again "only of plain Rust data, never of a JS object"; but
a struct JS made is a JS object, whose `margin` may be a getter, which ran
twice where Rust reads it once (AGENTS.md: preserve side effects and
evaluation order).

## Decision

**A read of a field of a struct whose value JS may have made is a
property a getter may answer**, `Known::Getter`, as a binding's read is:
every pass that asks `has_effects`, `reads_same` or `Known::Getter`
keeps it where Rust reads it, so a `let` of it keeps its `const`.

One owner says which structs those are, the crate's analysis of what may
come from code that isn't its own, `analysis/fn_pointers.rs`, which
already follows each value from outside for the fn pointers it may hold
(ADR 0907): `FnPointers::js_may_make(struct)`, asked by
`representation.rs`'s `project` where it reads a field. A struct JS may
make is one a value code that isn't the crate's made is or holds, through
its fields, a reference, an `Option`, a `Vec` and what a function it holds
gives:

- a binding's or a library's result, `js::cast`'s too, and what a JS
  function it gives gives when it's called;
- a library's static or constant, and what a library's `Deref` gives.

A struct the program builds and never takes from outside is its own data,
read in place as before; so is what std makes of what it isn't given, a
`transmute`'s, which is the program's bytes, not a JS object, and what
serde and serde_json give, which rust-js lowers itself (ADR 0459), of
JSON's data (`recognition::lowered_serde`).

**Not decided here: what JS calls an exported function with.** A struct
parameter of an exported function, a component's props, a class's `this`,
is read as the program's data, as before. Taking each for a JS object,
as a getter may answer, kept a `const` of nearly every field a component
or a method reads, `const step = this.step;`, and made a `Cell` field of
one a handle object, `{ get value() { .. } }.value`; React passes its
components props it made of what it was given, data properties. Where JS
calls one with an object of getters, a field read twice runs one twice.

**Nor what JS calls a function of the crate's with**, a closure's, a `fn`
pointer's or a `dyn Fn`'s parameters given to a binding: most are the
program's own objects given back, a bound function's, `function::bind(
call_destroy.react_stack_bottom_frame, ..)`'s fiber, or a state's, and
taking them for JS's kept React's `__DEV__ ? fiber._debugTask : null`
from being written so.

**Nor a generic `js::cast::<T>`.** What generic code casts is its caller's
type, and a cast of what the program stored, React's value stack's, is
its own object: such a struct is read as the program's data.

```js
const margin = styleProp.margin;
if (margin != null) {
  style.margin = margin;
}
```

## Why

The fact is the one `has_effects` already asks of a binding's read, so
no pass learns a new rule: tidy's moves, logicals and temporaries keep a
getter's read where it's written, as ADRs 0672 and 0866 say. Which
structs come from outside is the same question ADR 0907 answers of fn
pointers, over the same calls, so it's answered by that walk, not a
second one.

`test/lowering-moves.test.ts` counts a getter's reads of a field of a
struct `js::cast` gives, read once, and checks the program's own
struct is read in place. `test/corpus/field_of_a_struct_js_made_read_once.rs`
compares a binding's struct, `JSON.parse`'s, and the program's with
native Rust. Mutations take away each route and the fact.

## Alternatives

- A struct that's `js::Defined`: a class the program builds, React's
  fiber, is one too, and a struct JS made needn't be.
- Following each value from where JS made it to each read: a function's
  parameter reads what any caller gives, so it must ask the type anyway.
- Every struct: the program's own data, read in place today, would keep a
  `const` of each field read.
