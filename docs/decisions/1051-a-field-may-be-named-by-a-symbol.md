# 1051. A field may be named by a symbol

Status: Accepted (rd480, the user's decision of 2026-11-01: option A,
`#[rust_js::name]` extended). Extends [0573](0573-every-rust-name-is-camel-case-in-js.md),
whose `#[rust_js::name = ".."]` names a field by text, and
[0634](0634-a-registered-symbol-is-a-const.md), whose symbols are
values.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference. A field is a place Rust reads and writes by its name; what
JS keys it by is the program's choice, as a text name is, and every read
and write of the field is of that key.

## Context

React's Flight reply server keys a resolved model's response by a symbol
of its module's own, so a client's value can't name it:

```js
const RESPONSE_SYMBOL = Symbol();
new ReactPromise(RESPONSE_MODEL, value, { id, [RESPONSE_SYMBOL]: response });
const { [RESPONSE_SYMBOL]: response, id } = chunk.reason;
```

and its async iterable is an object of one method keyed by
`Symbol.asyncIterator`, `{[ASYNC_ITERATOR]() { .. }}`. A struct's field
could only be named by text, so the port keyed `response` by its name, a
key a caller sees in `Object.keys(chunk.reason)`, and set the iterator
method on an empty object.

## Decision

**`#[cfg_attr(rust_js, rust_js::name(SYMBOL))]` on a struct's field names
it by the symbol `SYMBOL` holds**: a `&'static js::Symbol` static or
thread-local of the crate's or a library's, JS's own,
`js::symbol::async_iterator`, or a `js::SymbolFor` constant. One way:
`#[rust_js::name]` already owns a field's key, by text, `name = ".."`; a
path in parentheses is its symbol form.

- **Its key is the symbol where the field is**: the literal's
  `[RESPONSE_SYMBOL]: response`, a read and a write `reason[RESPONSE_SYMBOL]`,
  an update's `{ ...pair, [SECOND]: 9 }`, a derived `Default`'s, and a
  closure there a method, `[Symbol.asyncIterator]() { .. }`, as a text
  field's closure is (ADR 0882). The symbol is its item read where it's
  used (`field_keys::field_symbol`): a binding's static as JS's global,
  the crate's or a library's as `item_read` reads it, so the load order
  makes it before a literal that reads it, `export const TAG = ..;` before
  `export const made = { [TAG]: 7 };` however the Rust orders them.
- **One owner of a field's key**: `bindings::field_key` gives a
  `FieldKey`, a name or a symbol's item, which only it makes;
  `Shape::Object` lists them; each reader asks it which. The JS IR gains
  `Prop::Keyed` and `Prop::KeyedMethod`, a computed key and its value.
- **rustc resolves and checks the path.** An attribute's tokens are no
  path rustc resolves, so the `jsx!` pass writes a use beside the struct,
  `#[rust_js::names_field] const _: fn(&Reason) = |value|
  ::js::symbol::names_field(&value.response, &RESPONSE_SYMBOL);`, its path
  the attribute's own tokens (`parser::field_symbol_uses`). rustc resolves
  it, checks it's a `js::symbol::FieldName`, "`u32` is no symbol a field
  may be named by", and sees the import or static used: no `#[allow]`, no
  unused warning in `rust-js clippy` (ADR 0888). rust-js reads which item
  it is, and of which field, of that use's HIR (`symbol_names::symbol_name`),
  never by a name it resolves itself. The use is a mark, `bindings::is_mark`,
  which writes no JS. A plain `cargo clippy`, which runs no pass, warns of
  a static only the attribute names, as it does of what only JSX uses
  (ADR 0440).
- **Its declaration is a computed key**, `[RESPONSE_SYMBOL]: Response`,
  and the symbol a `unique symbol`, what TypeScript takes a computed key
  of: an exported one's `export const TAG: unique symbol;`, a private one
  of the module's own declared `declare const RESPONSE_SYMBOL: unique
  symbol;` beside it, JS's own by its global, `[Symbol.asyncIterator]`.
  One of another module's, which the declarations would need to import, is
  refused where a declared struct's field names it.

### Refused

Each a compile error saying what to write (`symbol_names::validate`):

- on anything but a field: an item or a variant is named by text;
- of anything but one path;
- on an enum variant's field or a union's, whose keys are their own;
- on a field of a struct of type or const parameters, or one written in a
  function's body, beside which the pass writes no use;
- on a class's or an object of methods' field, a flattened one, a
  `#[rust_js::test]` member and a component's props field;
- on a field of a struct that derives or implements `Clone`, `Copy`,
  `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Debug`, `Hash` or serde's
  traits, whose JS reads each field by its name, until those read symbol
  keys.

A pattern of such a struct, `let Pair { first, .. } = pair;` or a
parameter's, reads each field, `const first = pair[FIRST];`, until a JS
pattern of computed keys is written for it, the next step.

## Why

- **It's upstream's JS**: `{id, [RESPONSE_SYMBOL]: response}`, the
  symbol a key no caller's text can name.
- **The Rust is a struct**, read and written as any, its key said where
  Rust meets JS, as a text name is.
- **rustc owns what the path is** (AGENTS.md): resolved and type-checked
  by rustc, not by a second resolver in rust-js, and used as rustc sees it.

## Alternatives

- **A new `#[rust_js::key(SYMBOL)]`**: a second attribute for a field's
  key beside `name`.
- **A typed key, `js::Keyed<ResponseKey, Rc<Response>>`**, a marker type
  per symbol: rustc-checked without a pass, but a dialect, the field's
  type changed and read through it.
- **rust-js resolving the attribute's path itself**: rustc's checks
  bypassed, and an import only the attribute names unused.
- **An untyped object, `js::set(reason, RESPONSE_SYMBOL, response)`**: a
  cast at every read.

## Consequences

- The js crate's symbols have no native form, so the corpus, run against
  native Rust, can't hold it: `test/field-symbol-names.test.ts` runs it
  in Node.
- `symbol_name` scans its module's marks for each symbol-named field's
  key; a field without the attribute pays one attribute lookup.

## What holds it

- `test/field-symbol-names.test.ts`, run in Node: the reply server's
  reason, made, read, written and declared; an iterable's method keyed by
  `Symbol.asyncIterator`; a `SymbolFor` constant's, made before the
  literal its module's load makes, and a constant's; two fields of two
  symbols, an update, a derived `Default`, a pattern and a parameter's;
  each refusal's message, and rustc's of a path that's no symbol or none.
- `test/clippy.test.ts`, "rust-js clippy of a symbol only a field's name
  uses says nothing": an imported thread-local and a private one, `-D
  warnings` clean.
- The guide's example, `docs/guide/interop/types.md`, compiled by `bun
  run docs`.
- Mutations: `field-symbol-any-use`, `field-symbol-use-unread`,
  `field-symbol-item-allowed`, `field-symbol-list-allowed`,
  `field-symbol-variant-allowed`, `field-symbol-unwritten-use-allowed`,
  `field-symbol-class-allowed`, `field-symbol-flatten-allowed`,
  `field-symbol-test-member-allowed`, `field-symbol-props-allowed`,
  `field-symbol-derives-allowed`, `field-symbol-not-unique`
  (`bindings/symbol_names.rs`); `field-symbol-name-unread`,
  `field-symbol-unvalidated`, `field-symbol-use-lowered` (`bindings.rs`);
  `field-symbol-use-unwritten` (`jsx_syntax.rs`),
  `field-symbol-any-tokens-used` (`jsx_syntax/parser.rs`);
  `field-symbol-read-by-name`, `field-symbol-global-as-item`
  (`field_keys.rs`), `field-symbol-constant-folded` (`representation.rs`);
  `field-symbol-literal-by-name` (`lower.rs`),
  `field-symbol-update-by-name` (`mir/rvalues.rs`),
  `field-symbol-default-by-name` (`std_impls.rs`),
  `field-symbol-pattern-by-name` (`patterns.rs`),
  `field-symbol-default-base-named` (`aggregates.rs`),
  `keyed-closure-no-method` (`prepare.rs`), `keyed-method-uncomputed`
  (`to_oxc/expressions.rs`), `field-symbol-private-undeclared`,
  `field-symbol-declared-by-text` (`declarations.rs`).
