# 0206. TypeScript, read and written by TypeScript's own parser and printer

Status: Accepted.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

rust-js meets TypeScript both ways. It writes a crate's `.d.ts` (ADR
0196), so far as strings it put together, which TypeScript only checks
afterwards. And it has TypeScript to read: react's attributes are typed by
`@types/react` alone, `AnchorHTMLAttributes` and its chain, and a
TypeScript codebase moving to Rust is declarations to read too. Reading a
`.d.ts` by patterns breaks on what a pattern doesn't foresee, a type over
several lines, a comment in one, an alias named elsewhere. As the JS is
printed by oxc from a tree (ADR 0018), TypeScript should be read and
printed by a parser and a printer, TypeScript's own.

## Decision

**`@rust-js/typescript`, the workspace's `typescript/`, is TypeScript's own
parser and printer, through TypeScript 7's API, and a model of
declarations between them:**

```js
import { open } from "@rust-js/typescript";

const ts = await open(["node_modules/@types/react/index.d.ts"]);
const { declarations } = await ts.read("node_modules/@types/react/index.d.ts");
const text = await ts.print({ header: "// ..", declarations });
await ts.close();
```

- **The model is JSON**: imports, interfaces with what they extend and
  their members, type aliases, functions, constants, `export default` and
  namespaces; and types: keywords, literals, references with their
  arguments, unions, intersections, arrays, tuples, functions and object
  types. What it doesn't take apart is `other`, as it was written. A type
  parameter is its name and what it extends, where it's said,
  `{ name: "C", constraint: ReactNode }`. (Amended: it was its name
  alone.)
- **`read` is TypeScript's parser**, `typescript/async`'s
  program of the files a session opens: each file's syntax tree, as the
  model.
- **`print` is TypeScript's factory and printer**: the model as
  TypeScript's syntax tree, each declaration printed by TypeScript, a
  blank line between them, as TypeScript writes them.
- **What it prints is erasable syntax only**, as TypeScript's
  `erasableSyntaxOnly` says: there's no `enum`, parameter property or
  `import =` in the model, and a namespace holding values is refused but a
  `declare`d one's.
- **TypeScript is 7.1.0-beta**, pinned, the first whose API isn't under
  `unstable/` (amended: it was 7.0.2's, `unstable`): a session is its native process, the async API's, which
  works under Node and Bun alike, where the sync API needs Node's.

## Why

- **It's TypeScript's**: what it reads, TypeScript parsed, and what it
  writes, TypeScript printed, so neither is rust-js's guess at TypeScript.
- **One model, both ways**: what's read can be printed, and printed, read
  as it was, which the module's test checks of a file of every kind of
  declaration; and what rust-js writes and what it reads are the same
  shape, for its generators and for moving TypeScript to Rust.
- **It's tested**: reading a file of each kind of declaration, printing a
  model, the two as one round trip, and `@types/react`'s
  `AnchorHTMLAttributes` taken apart.

## Costs

- **A session is a process of TypeScript's**, which starts in a second
  or so the first time on a machine.
- **The API is `unstable`**: a TypeScript release may change it, and the
  pin is moved deliberately.
- **The model is what rust-js needs**: classes, enums, overloads,
  conditional and mapped types are `other`, written as they were.
- **The compiler prints through the package a project has installed**, so
  a change to the model needs both released together: an older
  `@rust-js/typescript` can't print a newer compiler's type parameters.

## Amendment (2026-11-15): the reader preserves generic defaults (#1916)

The canonical typeParameters reader forwards the engine’s defaultType through
readType into the existing model’s default field, independently of constraint.
The writer already supports that field. No generator reconstructs defaults or
substitutes a constraint for one. The shared reader covers interfaces, aliases,
functions, methods, construct signatures and generic function types.

Before this fix, Box<T = string> printed as Box<T>, rejecting a strict caller
that omitted its type argument. Exact model and read/print/read tests now cover
primitive, reference, dependent and literal defaults, including a bounded
parameter whose default differs from its constraint. Original and printed
strict callers accept omitted arguments and reject invalid bounds/defaults.
Mutants drop defaults, replace them with constraints, or drop constraints.

This is ADR0262 case N at the declaration-tool boundary. Native Rust cannot
observe this reader; the independent oracle is the actual TypeScript checker.
There is no new representation, API alias or Rust semantic exception.

## Amendment (2026-11-15): computed symbol keys remain expressions (#1912)

The existing MemberName computed variant is used by the reader as well as
the writer. Property and method signatures with symbol keys read the AST's
ComputedPropertyName and preserve its qualified path. A literal string whose
text looks bracketed remains a string; it is never guessed to be a symbol.
The writer validates the existing symbol-path contract and refuses unsupported
computed expressions explicitly instead of printing another key.
Inventories use the model owner’s memberKey operation: computed paths retain
their brackets, raw string labels stay compatible, and reserved bracket/quote
prefixes are JSON-escaped so a literal name cannot collide with a symbol. The
WebAPI coverage consumer uses that same operation; its existing baseline stays
unchanged.

The published parser previously read [Symbol.iterator] as a string, causing
the printer to emit a quoted property with a different identity. Exact model
and read/print/read proof covers methods, readonly properties and optional
disposal methods. Independent strict TypeScript consumers accept both the
original and printed Iterable and symbol-property contracts, while keeping
the literal bracketed string method distinct. Source mutants break reading,
printing, qualification and the explicit refusal independently.

This is case N at the TypeScript tool boundary; native Rust cannot observe
this reader. The fix does not require the separate engine-library review or
change native runtime binding semantics. It is independently tested against
the published parser's original beta engine and the stable-engine review.

## Amendment (2026-11-15): construct signatures are structured members

A Member of kind construct holds typeParameters, params and returns, read
with the existing readers and printed by TypeScript
createConstructSignatureDeclaration. It is distinct from a method named
new (quoted by the printer) and from a class constructor. Interfaces and
object types share this one model. Optional and rest parameters, generic
constraints and the return type survive a read/print/read round trip.

Builtins and WebAPI coverage consume construct members to count
constructors; the old raw-text new regexes are removed. Property/method
readers deliberately skip construct members. checkedMembers, owned by this
module, validates the Member discriminants before those readers select
the kinds they consume; an unknown kind throws. It preserves the input
type and does not validate or invent field representations. The negative
protocol test supplies an unknown member to both the guard and printer.
Rust transports JSON models to the printer; existing Rust declaration
producers continue producing their existing property/method/index members.

This follows ADR 0262 case N: these are TypeScript syntax/model boundaries,
with no new Rust/JavaScript semantic exception. Tests cover both interface
and nested object positions, a method literally named new, optional/rest
arguments, generic constraints and exact round trips. Intl coverage uses
the same constructor model to exercise every changed native call site.
