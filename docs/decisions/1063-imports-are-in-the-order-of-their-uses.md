# 1063. Imports are in the order of their `use`s

Status: Accepted (the user's decision, rd570). Amended (rd608): a binding
the module declares itself is named where it's declared. Replaces
[0295](0295-import-order.md)'s order and
[0770](0770-a-librarys-const-is-imported-by-its-name.md)'s ordering of a
subpath by its file.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results change. JS
loads a module's imports in their order, and what each runs as it loads
can read what another's load sets, so the order is the program's: the
Rust writes it, in its `use`s.

## Context

ADR 0295 ordered a module's imports by their specifiers: packages, then
relative modules, each by path. ADR 0770's amendments kept a library's
module, imported by its package's subpath, where its file's path put it
(0c233f18a), so the bare specifier didn't move it, and a crate's own
module in another package too (1d159bcd0). That held React DOM's
ReactDOMFizzServerNode's order, upstream's:

```js
import { createRequest, .. } from 'react-server/src/ReactFizzServer';
import { createRenderState, .. } from 'react-dom-bindings/src/server/ReactFizzConfigDOM';
```

only because ReactFizzServer is a binding's JS module, a bare specifier,
and ReactFizzConfigDOM a file's, `../../../react-dom-bindings/..`, which
sorts after every bare one. Under jest the two are in a cycle: where
ReactFizzConfigDOM loads first, `supportsClientAPIs` is still undefined
as ReactFizzHooks' dispatcher is made, every `useState` throws, and
react-dom's tests go from 4264 passing to 4153. Ported to Rust,
ReactFizzServer's module is a file's too, `../../../react-server/..`,
which sorts after `react-dom-bindings`: the order is a sort's accident,
and nothing in the Rust can say it.

## Decision

**A module's imports are in the order its `use`s are written:**

1. Each import is where the first `use` that makes it is, a binding's JS
   module, a library's module and the crate's own module alike, the one
   order key of every kind (`to_oxc/header.rs`, of each import's `span`,
   the `use` its comments are before, rd92). Of two one `use` makes,
   `use crate::{a::x, b::y};`, by specifier.
2. An import no `use` makes, of what the module names by its path or a
   binding of its own, comes after, packages then relative modules, each by
   its specifier: the one fallback.
3. Then what it imports for its effect, `js::import!`, in the order the
   Rust writes them, its CSS after what it styles; then rust-js's runtime
   helpers. As ADR 0295 said.

rustfmt sorts the `use`s of one group and keeps the groups, the runs
between blank lines, as written. A module whose imports load in an order
writes them in that order's groups, as upstream's imports are:

```rust
use react_server::react_fizz_server::{abort, create_request};

use react_dom_bindings::react_fizz_config_dom::{create_render_state, create_resumable_state};
```

`js::Import::ordered_as`, `js::Package::ordered_as` and
`Dependencies::ordered_as` are gone: a subpath is ordered by its `use`,
as every import is, and by its text where no `use` makes it.

## Why

- **It's the program's order.** JS runs a module's imports in turn, so
  where loads read what other loads set, as React's jest cycles do, the
  order changes results. The Rust is where the author says it, and rustfmt
  keeps what they wrote.
- **It's one key for every import.** The sort by specifier needed a second
  key, a file's path, to keep a subpath where it was, and still gave no
  way to put one module before another.
- **It's the JS a person writes**: upstream's files import in the order
  their author wrote, the order a port's `use`s follow.

Rejected:

- **Where the file's JS was imported before the port.** Not in the
  compiler's inputs (ADR 0091), without a second form saying it.
- **First use in the code.** Moving a function would change the load
  order, and upstream's order is its imports', not its code's.

## Consequences

- Every module with more than one import made by its `use`s prints them in
  their `use`s' order. rustfmt's sort of one group is by path, so within
  one the order is mostly the old one; across groups it is the program's.
- A port orders its `use` groups as upstream's imports.
- A module of no `use`s, its imports all named by path, keeps ADR 0295's
  order.

Held by `test/libraries.test.ts`'s "imports are in the order of the uses
that make them", whose JS modules note their loads, a binding's and a
library module's from two groups either way round, and by the mutations
`imports-by-path-not-use` and `imports-unused-first`.

## Amendment: a binding the module declares is named where it's declared (rd608)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, as above.

The reconciler reaches its host config through bindings each module
declares itself, `#[cfg_attr(rust_js, rust_js::link_name =
"./src/ReactFiberConfig.js#setCurrentUpdatePriority")] fn ..`, as
react-client does its `./ReactFlightClientConfig.js` and shared its
ReactSharedInternals. No `use` makes their imports, so the fallback put
them after every `use`'s, where upstream's ReactFiberHooks imports
`./ReactFiberConfig` first: no source could say its order.

**An import is where the source first names it**: the first `use` that
makes it, or the first binding the module declares of it, a function or
a static of its own or of an `extern` block, whichever is written first
(`UsesWritten::package_named_at`, carried as `js::Package::named_at`). The
module writes the declaration where the import goes, among its `use`s.
Only an import the source names neither way, a module read through its
path's namespace, is the fallback's. Comments stay with the `use` the
import is of (rd92), not a declaration's.

The other way, a `use`-able module of the bindings, `use
crate::react_fiber_config::{..};` of a module declaring them once, was not
taken here: a module of bindings alone writes no JS of its own to import
from, and each binding's relative specifier is its declaring file's,
which another module's import would have to rebase. The declaration's place
is the key a `use`'s is, so the rule stays one.

Held by `test/libraries.test.ts`'s "imports are in the order of the uses
that make them", a binding declared between two `use` groups loading
between them, and `test/modules.test.ts`'s bindings declared in their
order; mutation `imports-declared-unordered`.

## Amendment: a binding named through the module that holds it (rd609)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, as above.

A `use` that names a binding through a module that holds it, shared's
`use shared::internals::REACT_SHARED_INTERNALS;` or `use
shared::assign::assign;` (ADR 0633), makes that module's import,
`import ReactSharedInternals from 'shared/ReactSharedInternals'`, which
no rule placed: `UsesWritten` asked what the `use` names, the binding,
whose JS module, `react` or none, isn't what's imported, so the import
fell to the fallback after every `use`'s, where ReactCurrentFiber,
ReactFiberAct, ReactFiberClassComponent and others import it first.

**Such a `use` makes its holder's import**: of each binding it names,
the module its path names it through, where that module holds it, a
library's as its manifest says or the crate's own (`Held::use_holder`,
`held_by`'s rule of a path, ADR 0633), is what the `use` places
(`UsesWritten::package_at`, `module_at`), and not the binding's own JS
module, which the module doesn't import there. A `use` of the binding
itself, `use js::object::assign;`, is the binding's as before.

Held by `test/cargo-link.test.ts`'s "a library's module that holds a
binding is what a consumer reads it through", a library's holder and the
crate's own each before what sorts first, and `react`'s import where its
binding is declared; mutations `imports-held-library-unplaced`,
`imports-held-module-unplaced` and `imports-held-binding-own`.

## Amendment: a `use` of a library's namespace, or of a module of bindings (rd611)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, as above.

Two more `use`s placed no import, which fell after every `use`'s: React
DOM's `use react_reconciler::react_fiber_reconciler::ReactFiberReconciler;`,
the reconciler's namespace unit struct (ADR 1037), whose methods are its
module's functions, `import {..} from
'react-reconciler/src/ReactFiberReconciler'`; and ART's `use art::{..,
mode, transform};`, modules of bindings, `transform::new()`'s `import
Transform from 'art/core/transform'`. `UsesWritten` asked what the `use`
names, a struct or a module, of the JS module it's imported from, which
neither is.

**A `use` of another crate's namespace makes its methods' import, and one
of another crate's module its bindings'** (`UsesWritten::imports_from`,
`bindings_in`): of the module's own bindings, a function or a static, not
of modules inside it, which a `use` of each names. The crate's own
namespace and modules were placed already, as the crate's modules are
(`module_at`).

Held by `test/cargo-link.test.ts`'s "a use of a library's namespace or of
a module of bindings places its import", both before what sorts first;
mutations `imports-namespace-use-unplaced` and
`imports-module-use-unplaced`.

## Amendment: re-exports are among the imports, where the source names them (rd620)

React's reconciler port writes `use crate::react_portal;` where upstream writes `export {createPortal} from './ReactPortal';`, before `import {registerDefaultIndicator} from './ReactFiberAsyncAction';`. Its namespace's forwarding method makes the `export { createPortal } from "./ReactPortal.js"` (rd292b). The same holds for ReactTestSelectors, ReactFiberHooks and ReactFiberErrorLogger. But every `export {..} from` printed after every import: the printer put re-exports first in the module's code, below the header that holds the imports (ADR 0240). JS loads a module's imports and re-exports in the order they're written, so that order was a printer's accident, not the program's.

**A re-export is printed among the imports, at the first place the source names what it's from** (`program::Reexport::at`), as an import is:

- a `pub use` at itself;
- any re-export at an earlier `use` that names its module or an item in it (`UsesWritten::module_at`), or the package it's from (`package_at`). That covers a namespace's forwarding methods (rd292b) and a function that only forwards (rd575, ADR 1064), whose own spans are in the module's code;
- a `pub use` counts as a `use` of what it names for the module's own code too, `pub use shapes::area;`, so that import is placed at it, before its re-export;
- a `use` of a library's module, `use dep::action;`, places the import of that module's own functions, as a binding module's did (rd611): `items_in`, which was `bindings_in`.

Each is one line in the header block, `export { a, b as c } from "x";` or `export * as ns from "x";`, with no blank line between, as upstream writes them. Its comments are above it, and a doc comment is a JSDoc, as an item's is. An import and a re-export placed at one `use` print the import first. One the source names neither way is after every placed one, as before (ADR 1063).

Case ([0262](0262-when-rust-and-js-disagree.md)): C, as the imports' order is. What a module's load reads may depend on which module loads first, and the source now says it.

Held by `test/namespaces.test.ts`, `test/declarations-libraries.test.ts`, `test/modules.test.ts`, `test/cargo-link.test.ts`'s "a namespace's forwarding method's re-export is where its module's use is" (`use crate::portal;` before `use crate::aaa::a;`: the export prints first), the guide's modules.md and building.md, and snapshot `modules`. Mutations in `to_oxc/header.ts`, `lower/pipeline.ts` and `lower/analysis/naming.ts`.

## Amendment: a binding's class a class extends is imported where its `use` is (rd629)

ReactART writes `use react::{CSSProperties, Component, HostType, JSX, jsx};` first, where upstream writes `import * as React from 'react';`, but `import { Component } from "react"` printed after `./ReactARTInternals.js`. `Component` is a trait, react's class (`#[rust_js::types = "react#Component<P, S>"]`), which `impl Component<P> for Surface` makes the class's `extends` (rd331). Its import is the superclass's, which no `use` was taken to make: `imports_from` asked a trait for none. **A `use` of a binding's class (`classes::base_class`) makes the import from that class's module**, so it's placed there. Held by `test/class-components.test.ts`'s "a class's superclass is imported where its use is", mutation `superclass-use-unplaced` (`analysis/naming.ts`).

## Amendment: a header comment keeps the blank line after it (m6-wife's P3)

A module made only of re-exports, a package's entry, printed its `//!` doc right above its `export {..} from`, with no blank line between, where the Rust and the file it ports have one. A module with code kept the blank line, which the code's printer keeps. **A comment block in the header, above an import or a re-export, keeps one blank line after it where only whitespace with a blank line is between it and what it's above.** That doesn't apply to one moved from a `use` that makes no import to the next import, as other `use`s are between. A re-export's span was a statement start of its own in `to_oxc/starts.rs`, from when it printed in the code. The header's starts, the imports' and re-exports', hold it now, so that start is gone, and with it `reexport-comments-unplaced`, which survived on main since rd620. Held by `test/lowering-comments-items.test.ts`'s "a module of re-exports alone keeps its inner doc and the blank line after it" and "a module's inner doc comment is a plain JS comment". Mutations `header-comment-blank-dropped` and `header-comment-blank-of-anything` (`to_oxc.ts`).


## Amendment: each import is a statement where its `use` is (rd652, ADR 1069)

The imports and re-exports were one block of text above the code, so an
item written between two `use`s printed after every import, React's
ReactFizzConfigMarkup's `isPrimaryRenderer`. They're statements of the
module's program now, in this order, each after the code written before
its place ([ADR 1069](1069-imports-are-statements-of-the-module.md)); one
the source names neither way is right after the one before it, so the
order JS loads them in is this ADR's, unchanged. The order's owner is
`to_oxc/header.rs`'s `entries`.
