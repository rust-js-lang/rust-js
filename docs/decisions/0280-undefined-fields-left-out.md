# 0280. A field that's `undefined` is no key

Status: Accepted. Amended: an optional `OnceCell` field is left out where it's made empty; a `None` field a comment is written after on
its line keeps its key, `undefined`, the comment's place; a comment before a
field changes no key, and where no key follows it, it's before the object's
`}` (rd321); a field that holds a cell keeps its key where it's made `None`
(the user's decision, 2026-10-27); an optional field's `None` is left out
whatever its comments, one after it on its line too (the user's decision,
2026-11-03).

Case: A ([0262](0262-when-rust-and-js-disagree.md)); B where upstream writes the key, `_owner: undefined`, the user's decision (2026-11-01, rd391): `in`, `Object.keys` and `hasOwnProperty` tell them apart, listed in 0262's Consequences.

## Context

A struct is an object of each of its fields (ADR 0020), so one made with a
`None` has a key of `undefined`:

```js
meta={{ title, titleForTitleTag: undefined, version: undefined, description: undefined }}
const init = { method: "POST", headers: undefined, body, referrer: undefined, /* .. */ };
```

Hand-written JS leaves those keys out, as react.dev's pages do, and as
Sandpack's files are `{ code, hidden, active }`.

## Decision

**An object's field whose value is `undefined`, a `None`, `()` or a unit
struct, is no key**: `{ title }`, `{ method: "POST", body }`. Not in an
object with a spread, `{ ...flags, on: undefined }`, where the key
overrides the spread's.

The runtime reads objects alike either way: `$eq` compares the keys either
has, and `$key`, a set's or a map's, leaves out a field that's `None`.

## Why

- **No program can tell**: what reads the field reads `undefined`, which
  is its `None`, as JS reads a key that isn't there; `==` and hashing treat
  the two alike.
- **JS then does what the original does**: `in`, `Object.keys` and a spread
  of it, `{ ...defaults, ...props }`, see no key, where `undefined` would
  override a default.
- **It's tested**: a test makes a struct of a literal `None`, a default, and
  one given `None`, compares and hashes them alike, and keeps a spread's
  `undefined`; mutations keep the keys, drop the spread's, and count keys
  in `$eq` and `$key`.

## Amendment: wherever the object is written

The readability pass that leaves the field out skipped a template's
values, so the same `Options { value: None }` was `f({})` returned and
`f({ value: undefined })` in a `format!`, and `"value" in options` told
them apart. It now visits every expression an expression holds, named
variant by variant, so a new one is a compile error until it's handled.

## Amendment: a field a comment is written on keeps its key

React's `createCacheNode` writes each field's meaning after it:

```js
return {
  s: UNTERMINATED, // status, represents whether the cached computation returned a value or threw an error
  v: undefined, // value, either the cached result or an error, depending on s
  o: null, // object cache, a WeakMap where non-primitive arguments are stored
  p: null, // primitive cache, a regular Map where primitive arguments are stored.
};
```

The port's `v: Cell::new(None), // value, ..` was left out, and its
comment with it, though every comment is in the JS where its code is
(ADR 0444). A comment on its own lines before the field was worse: it
was the next field's, `// value ..` above `o: null`.

**A `None` field a comment is written on keeps its key, `v: undefined, //
value ..`**: one after its value on its line, in it, or on lines of their
own right before it. Only there is the comment where its code is: no other
place says what it says. A field without one is left out, as before.

- **Which fields have one is the struct literal's** (`aggregates.rs`,
  `commented_fields`), read from its text as rustc's lexer reads it, and
  carried on the object as written, `js::ObjectLayout::commented`, its keys,
  which the pass that leaves `undefined` out keeps: a field's value is a
  temporary until tidy reads it in place, so the object, not the value,
  says so.
- Case: A still: what reads the key reads `undefined` either way, and
  `$eq` and `$key` read a key of `undefined` and none alike. `in` and
  `Object.keys` see it, as they see React's.

Held by `test/lowering-comment-fields.test.ts`, "a None field a comment is
written on keeps its key and its comment", run in Node, and the mutations
`commented-none-left-out` (`prepare.ts`), `commented-field-trailing-unread`
and `commented-field-leading-unread` (`lower/aggregates.ts`).

## Amendment: a comment before a field speaks of the `None`s after it

Superseded the same day by the amendment below: it wrote keys upstream
leaves out.

React's `CacheContext` heads two fields with one comment:

```js
  // We'll initialize these at the root.
  _currentValue: null as any,
  _currentValue2: null as any,
  _threadCount: 0,
```

The port's `current_value: Cell::new(None), current_value2:
Cell::new(None)` under it was `currentValue: undefined` and no
`currentValue2`: the comment kept its own field's key and not its
sibling's, so the two `None`s it speaks of were written two ways.

**A comment on lines of their own before a field speaks of the `None`s
written right after that field too**, each on the line after the one
before it with no comment between: each keeps its key, `currentValue:
undefined, currentValue2: undefined`. Another field, a blank line or a
comment ends them; a comment after a value on its line is that field's
alone, as before.

- **Which are `None`s is MIR's**, a `None` or `Cell::new(None)` the literal
  writes (`Given::WrittenNone`, ADR 0810), handed to `commented_fields`
  with the literal's text, which says where its comments and blank lines
  are.
- An optional field's `None` is left out as before where no comment is on
  it or heads it (ADR 0810).
- Case: A still, as above.

Held by the same test, which adds a run ended by another field and one
ended by a blank line, and the mutations `commented-run-unread`,
`commented-run-past-a-value` and `commented-run-past-a-blank-line`.

## Amendment: a comment changes which keys are written only where it has no other place (2026-10-26)

The amendment above wrote keys React's committed files don't have.
createContext's object, `// These are circular` before `Provider` and
`Consumer`, gained `_currentRenderer: undefined, _currentRenderer2:
undefined`, which upstream sets only in DEV; ReactElement's, `// Record
the component responsible for creating this element.` before `_owner`,
gained `_store: undefined`, which upstream defines only in DEV, by
`Object.defineProperty`. Each is a key `Object.keys` sees, in production
too: a shape at the JS boundary that changed. And before it, a comment on
lines of their own before a `None` kept that one key and not its
sibling's, `_currentValue` and not `_currentValue2`, so a comment decided
what the object is.

**A comment changes which `None` keys are written only where it has no
other place.** A comment after a field's value on its line, or in it, is
that field's alone: it keeps its key, `v: undefined, // value ..`, as
before, as React's createCacheNode has it. A comment on lines of their own
before a field heads what follows: the keys are what they are without it,
and it's before the next key the object has, `// We'll initialize these at
the root.` before `_threadCount`, `_currentValue` and `_currentValue2` left
out alike. Only where no key follows does the `None` it's before keep its
key, the comment's place then, as it would otherwise be outside the
object.

- **Which comments are where is the literal's** (`aggregates.rs`,
  `commented_fields`): the keys a comment is on, `ObjectLayout::commented`,
  and the keys one is before, `ObjectLayout::led`; **which keys are left
  out is preparation's** (`src/prepare/undefined_keys.rs`), which alone
  sees each value as it ends, and keeps a led `None` only where no key
  after it is kept.
- **What this undoes**: the amendment above, and the first amendment's
  `None` a comment is before keeping its key, which it said put the
  comment "before the next field"; it does, and now that's the rule, as the
  comment heads what follows.
- Case A still: what reads a key reads `undefined` either way; `in` and
  `Object.keys` see what upstream's do.

Held by `test/lowering-comment-fields.test.ts`, "a comment before a field
changes no key but where it has no other place", of createContext's,
ReactElement's and the cache context's shapes and a comment before a last
`None`, run in Bun, and "a None field a comment is written on keeps its
key and its comment"; and the mutations `commented-field-leading-unread`,
`commented-field-leading-as-on` (`lower/aggregates.ts`),
`led-none-always-kept` and `led-none-last-left-out` (`prepare/undefined_keys.ts`).

## Amendment: a field that holds a cell keeps its key (2026-10-27)

The user's decision. React's `createEffectInstance` writes `{ destroy:
undefined }`, a key the effect sets later, so every instance has its
shape from the start:

```rust
pub struct EffectInstance {
    pub destroy: Cell<Option<&'static Unknown>>,
}

pub fn create_effect_instance() -> EffectInstance {
    EffectInstance { destroy: Cell::new(None) }
}
```

It was `{}`: a `None` is no key, wherever it is.

**A field that holds a cell, the property it holds (`property_cell`: a
`Cell`'s, a struct's `RefCell`'s by its declaration, a `OnceCell`'s),
keeps its key where it's made `None`, `destroy: undefined`.** A cell is
there to be set later; a plain `Option` field isn't, and its `None` is
still left out. A nullable and optional cell field, `#[rust_js::nullable,
rust_js::optional]`, is absent until it's set, as ADR 0810 says, so
upstream's fields set only under a flag, `FiberNode`'s `actualDuration`
and `_debugOwner`, whose Flow types are optional too, are written so.
Case A, as before: what reads the field reads `undefined` either way.

- **One list of kept keys**: the struct literal's lowering (`aggregates.rs`)
  gives its object's `ObjectLayout::kept`, the keys a comment is written
  on and those of cell fields, which `undefined_keys::left_out` keeps.
- About 15 corpus outputs re-blessed, `cache: undefined` of a
  `OnceCell`, `next: undefined` of a `Cell`; the guide's `FiberNode`
  marks its flag-only fields optional; the error on `optional` of a field
  that isn't nullable says a cell's keeps its key.
- Held by `test/lowering-field-cells.test.ts`'s "a cell field made None
  keeps its key"; mutations `kept-none-left-out` and
  `cell-fields-unknown`.

## Amendment: an optional `OnceCell` field is left out where it's made empty (lz1, 2026-10-27)

React's lazy payload is `{ _status: Uninitialized, _result: ctor }`, its
`_ioInfo` set only in DEV. The port's `io_info: OnceCell<Rc<IoInfo>>`, a
cell that keeps its key (above), printed `_ioInfo: undefined`, and
`#[rust_js::optional]`, which leaves a cell's key out till it's set, was
refused on what isn't nullable. **A `OnceCell` field may be
`#[rust_js::optional]`**: made empty, it's left out, `{ _status: -1 }`, set
later as any cell field is; its declaration was optional already, absent
until it's set (ADR 0847). A `OnceCell` has no `None` to write but the
empty one, so it needs no `nullable`.

- Case ([0262](0262-when-rust-and-js-disagree.md)): N for Rust, whose
  `get()` reads `undefined` either way; the key's absence is what JS
  sees, as the attribute asks.
- Held by `test/lowering-optional-fields.test.ts`, "an optional OnceCell
  field is left out where it's made empty", and the mutation
  `optional-once-cell-refused`.

## Amendment: an optional `Cell<Option<T>>` field is left out where it's made `None` (rd331g, 2026-11-01)

react-art's Text constructs itself, `super(props)`, and has no `_text`
until its ref sets it, `ref={t => (this._text = t)}`. The port's `_text:
Cell<Option<&'static Unknown>>`, made `Cell::new(None)`, printed
`this._text = undefined;`, an own key upstream's instance lacks, which
`Object.keys` sees; `#[rust_js::optional]` was refused on what isn't
nullable, and nullable and optional printed its ref's `this._text = t ??
null;`, a `?? null` upstream doesn't write. **`#[rust_js::optional]` is
of a cell of an `Option` too, a `Cell<Option<T>>` or a `RefCell` of one**,
as of a `OnceCell` (lz1): made `None`, it's left out; set, it holds what
it's given, as any cell field does. Case N
([0262](0262-when-rust-and-js-disagree.md)): its `get()` is `undefined`
either way; the key's absence is what JS sees, as the attribute asks. The
refusal names the cells it's of. Held by
`test/lowering-optional-fields.test.ts`'s "an optional Cell of an Option is
left out where it's made None", run in Node, and the mutation
`optional-cell-refused` (`lower/bindings.rs`).

## Amendment: a comment before a last `None` is before the object's `}` (rd321, 2026-11-03)

The amendment of 2026-10-26 kept the key of a `None` a comment on lines of
their own was before where no key followed, "the comment's place then, as
it would otherwise be outside the object". React's ReactFiberThenable
makes its thenable's `ReactIOInfo` so:

```rust
let io_info = Rc::new(ReactIOInfo {
    name: .., start: .., end: .., byte_size: None, value: .., env: None,
    // We don't know the requesting owner nor stack.
    owner: None,
    stack: None, debug_stack: None, debug_task: None,
});
```

Its fields are `#[rust_js::nullable, rust_js::optional]`, absent until set
(ADR 0810), and upstream's object has no `owner`; the port's printed
`owner: undefined`, so `"owner" in ioInfo`, `Object.keys` and DevTools,
which reads these objects, saw a key a comment had made. So did the
program: `js::has(&info, "owner")` was `true` with the comment and `false`
without it. A comment changed what a program computes.

**A comment changes no key.** A comment on lines of their own before a
field is before the next key the object writes, as before, or, where none
follows, before the object's `}`, on lines of their own, as one after its
last field is (ADR 0708, amended): `{ name, start, // We don't know ..\n
}`. Which keys the object writes is preparation's alone
(`js::undefined_keys::left_out`); where the comment goes is the printer's,
which asks the object as it's printed where the keys it writes end, the
last's value's end (`to_oxc::closing`'s `ObjectEnd::written_to`): where
the literal writes each field's value, from its start to its end
(`ObjectLayout::written`, which the field's start, `ObjectLayout::start`,
is of, so a JS value of no place of its own, `row ?? null`, is known),
else the value's span; where one's is known neither way, the comment
isn't placed there, and is before the statement. A comment of the
innermost literal it's in, past them, is before the `}`, as one with only `,` and
comments after it was. One inside a written field's value, a method's
body's, React's Flight client's `// We shouldn't get any more enqueues
..` in its stream controller's `error`, is that code's, as before. The literal's
"led" keys, `ObjectLayout::led`, which only this exception read, are gone.
An object that writes no key, all its fields `None`, is `{\n // why\n }`.

A comment after a field's value on its line still keeps its `None`'s key,
`v: undefined, // why`, React's createCacheNode's, where upstream writes
the key too: that comment's place is its field's line.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results change: a
program's `in`, `Object.keys` and `hasOwnProperty` of its own struct, and
JS's, now see the keys its fields say, whatever the comments. Native Rust
has no `in`, so the corpus can't show it; the test runs the program's own
`js::has`.

Held by `test/lowering-comment-fields.test.ts`'s "a comment before a last
None is before the object's closing brace, the None left out" (React's
`ReactIOInfo`, a comment before the last-written `None`, one in a nested
literal, one of an object of no keys, and the program's `js::has`, run),
"a comment inside a field's value stays there",
and "a comment before a field changes no key but where it has no other
place", now `Object.keys` of `["props", "_owner", "ref"]`; mutations
`object-end-comment-before-left-out-unplaced`,
`object-end-comment-of-outer-literal`, `object-end-last-written-first`,
`object-end-value-end-unread`, `object-end-written-value-unread`
(`to_oxc/closing.ts`). The mutations of the exception,
`led-none-always-kept`, `led-none-last-left-out`,
`commented-field-leading-unread` and `commented-field-leading-as-on`, go
with its code.

## Amendment: a comment after an optional field's `None` changes no key (the user's decision, 2026-11-03)

The amendment above left one comment that decides a key: one after a
`None`'s value on its line, createCacheNode's `v: undefined, // why`. It
applied to `#[rust_js::optional]` fields too, which ADR 0810 says are
absent until set, so `owner: None, // why` printed `owner: undefined`
(and lost its comment, as the `None`'s JS has no place of its own), and
the program's own `js::has(&info, "owner")` was `true` with the comment
and `false` without it, as rd321's was.

**The user's decision: an optional field's `None` is left out whatever
its comments.** A comment after it on its line, or in its value, is the
line of no code, and is placed as one on lines of its own before it is
(rd321): after the key written before it, before the next, or before the
object's `}`:

```rust
Info {
    name,
    env: None, // set where it's known
    start: 1.0,
    stack: Cell::new(None), // DEV only
}
```

```js
return {
  name,
  // set where it's known
  start: 1,
  // DEV only
};
```

A field that isn't optional, whose `None` is no key only as JS leaves
`undefined` out, keeps createCacheNode's `v: undefined, // why`, as
upstream writes it.

- **Which keys are kept is the struct literal's lowering**
  (`aggregates.rs`): `ObjectLayout::kept` no longer has a commented field
  declared optional (`bindings::is_optional`), so
  `js::undefined_keys::left_out` leaves it out as any.
- **Where the comment goes is the printer's**: `to_oxc::closing::left_out`
  gives where each value is written of a field its object, as printed,
  leaves out (`ObjectLayout::written` of a key the object has no field
  of), and a comment whose line is that field alone, `env: None,` and
  comments, `closing::of_left_out`, is no code's, so it's placed as one on
  lines of its own, by the starts and `closing::end_of` that place
  rd321's. One after a literal written on its line, `Short { env: None, n:
  1 } // why`, is its statement's, as before.
- Case ([0262](0262-when-rust-and-js-disagree.md)): C, as rd321's: a
  program's `in`, `Object.keys` and `hasOwnProperty` of its own struct saw
  a key a comment made. Native Rust has no `in`, so the test runs the
  program's own `js::has`.

Held by `test/lowering-comment-fields.test.ts`'s "a comment after an
optional field's None changes no key" (a line comment before a written
key, a block comment, one before the `}`, one before the first key, a
non-optional field's kept, a literal on one line, `Object.keys`, `in`,
and the program's `js::has`, run), and the mutations
`commented-optional-none-kept` (`lower/aggregates.ts`),
`left-out-fields-unknown`, `left-out-fields-every-written` and
`left-out-line-of-other-code` (`to_oxc/closing.ts`).
