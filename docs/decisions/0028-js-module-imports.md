# 0028. Imports from JS modules: `#[link_name = "module#path"]`

Status: Accepted. Extends [0021](0021-js-interop.md) and the `#[link_name]`
forms of [0024](0024-web-crate.md). [0039](0039-generic-bindings.md) adds
imports for their side effects, and names a default import after the `static`
that holds it. Amended below: a binding's `#[rust_js::name]` names its import.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

To write rust-js's own playground in rust-js, a program has to use JS
packages, not only globals: `import { EditorView } from "codemirror"`,
`new WASI(..)` from `@bjorn3/browser_wasi_shim`, `EditorState.create(..)`
from `@codemirror/state`. It needs named exports that are functions,
classes, static methods and plain values. It sometimes needs a default
export (`import page from "./index.html"`), or the whole module. ADR 0021
left this open.

How the others do it (checked in local clones):

- **ReScript** puts the module on each binding:
  `@module("vscode") @scope("commands") external executeCommands: .. = "executeCommands"`.
  `= "default"` is the default export. `@module external mk: .. = "xx/foo_class"`
  is the module itself. The output is one namespace import per module,
  named after it (`import * as Vscode from "vscode"`), and every use is a
  path from it (`Vscode.commands.executeCommands(..)`).
- **Scala.js** has `@JSImport("foo", "Bar")`, `JSImport.Default` and
  `JSImport.Namespace`. The compiler reduces each one to *a module and a
  path in it*. The emitter prints `import * as $i_foo from "foo"` and
  selects the path (`Emitter.genModuleImports`, `SJSGen.pathSelection`).

Both keep the same two facts about a binding: **which module, and what path
inside it**. rust-js already has half of that. A dotted `#[link_name]`
(`console.log`) is a path from the global object.

Rust has `#[link(name = "..")]` and `#[link(wasm_import_module = "..")]` for
"these come from elsewhere". Here is what rustc (the pinned nightly) says
about each spelling:

| Spelling | rustc |
|---|---|
| `#[link(..)]` on an `extern "Rust"` block | warns that it should be applied to a block with a non-Rust ABI, and that this will become an error |
| the same on `extern "C"` | warns that `&str` and `String` aren't FFI-safe |
| `#[link_name]` on the block | warns that it can't be used on foreign modules |
| a tool attribute, `#[js::module(..)]` | fine, but every program needs `#![feature(register_tool)]` and `#![register_tool(js)]` (rejected in ADR 0021) |
| `#[link_name = "node:fs#readFileSync"]` | fine |

## Decision

**A `#[link_name]` with a `#` is an import: the module, then the path in
it.**

```rust
unsafe extern "Rust" {
    #[link_name = "node:path#join"]
    safe fn path_join(a: &str, b: &str) -> String;
    #[link_name = "@codemirror/state#EditorState.create"]
    safe fn create_state(config: &Config) -> &'static EditorState;
    #[link_name = "new @bjorn3/browser_wasi_shim#WASI"]
    safe fn new_wasi(args: &[&str], env: &[&str], fds: &Fds) -> &'static Wasi;
    #[link_name = "codemirror#basicSetup"]
    safe static basic_setup: &'static Extension;
    #[link_name = "./index.html#default"]
    safe static page: &'static Html;
    #[link_name = "./greet.js#*.polite"]
    safe fn polite(name: &str) -> String;
}
```

```js
import * as greet from "./greet.js";
import page from "./index.html";
import { EditorState } from "@codemirror/state";
import { join } from "node:path";

EditorState.create(config);
```

- **Before the `#`** is the module, written into the `import` as it is, for
  the bundler or the browser to resolve. The *last* `#` splits, since a
  module's name can start with one (Node's `#internal` imports).
- **After the `#`** is a path. Its first name is what's imported and the
  rest are property reads, as with `console.log`. `default` is the default
  export, and `*` is the module itself.
- It works wherever a global does: functions, `new`, and statics. Methods and
  properties (a `this` parameter, `"get x"`) are on a value, so a `#` there
  is an error.
- **Each file imports only what it uses**, in one statement per module
  (`import greet, { punctuation } from "./greet.js"`), plus one for the
  namespace if it's used.
- **Names are unique in the crate, and reserved like globals.** A named
  import keeps its export's name. A default or namespace import is named
  after its module, as in ReScript: `./greet.js` is `greet`, and
  `@codemirror/lang-rust` is `langRust`. A name that's taken gets `$1`:
  `import { URL as URL$1 } from "node:url"` beside the global `URL`. A local
  variable can't hide an import.
- **A relative module (`./`, `../`) is relative to the crate root's JS
  file.** A file two directories down imports `./greet.js` as
  `../greet.js`, so every module that imports a file names it the same way.
  The JS names it as a person does, from the importing file's directory:
  `./inner/wave.js` is `./wave.js` in `inner/leaf.js`, not
  `../inner/wave.js`.

## Why

- **One string holds both facts**, which is how ReScript and Scala.js model
  a binding: which module, and which path. That's the dotted path we
  already had, with a module in front.
- **rustc accepts it silently.** The alternatives either warn, will soon be
  errors, or need boilerplate in every program.
- **Named imports read like hand-written JS.** The generated file looks like
  the playground's own `main.ts`. ReScript and Scala.js print namespace
  imports and paths instead, which is simpler for them, but the JS reads
  less naturally. Bundlers tree-shake both.
- **Crate-wide names** mean the same import has the same name in every file,
  which is easier to read and to search for.

## Alternatives

- **`#[link(wasm_import_module = "..")]`** means "imports from this module"
  in Rust already, and would be the most fitting. But rustc warns about it on
  `extern "Rust"`, the warning is on its way to an error, and `extern "C"`
  warns about every `&str` instead.
- **A separate attribute for the module**, like ReScript's `@module`: needs a
  tool attribute, and every program would have to register the tool.
- **Namespace imports only**, as ReScript and Scala.js do: fewer cases to
  handle, but `path.join(..)` where the source said `join` isn't how people
  write JS.
- **Relative to the importing file**: the same `./greet.js` would mean
  different files in different modules, and moving a Rust module would
  silently change which file it imports.

## Consequences

- The module name is repeated on each item, as in ReScript. A crate of
  bindings (like `web`, ADR 0024) keeps that in one place.
- The generated JS needs a bundler (`bun build`, Vite) or a browser import
  map for bare package names, like any JS that imports packages. Node and
  Bun run it as it is.
- The playground can't load packages. It shows the JS, and says why it
  doesn't run it.
- Not yet: import attributes (`with { type: "json" }`), exports whose names
  aren't identifiers, and dynamic `import()`. (Amended: `js::import!` of an item
  is dynamic `import()`, ADR 0304.)

## Amended by

- [0202](0202-imports-named-around-importers.md): An import is named in each file that imports it
- [0305](0305-require.md): A binding marked `require` is read through `require(module)`
- [0373](0373-one-binding-form.md): A binding of a JS function is an ordinary function, and a JS type a struct

## Amendment: a binding's `#[rust_js::name]` names its import (2026-10-28)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a name.

React's ReactFiberHooks imports `{NotPendingTransition as
NoPendingHostTransition} from './ReactFiberConfig'` (nm1 of the port's
gaps). Its port binds it where it's read, by the name it reads it by:

```rust
unsafe extern "Rust" {
    #[cfg_attr(rust_js, rust_js::name = "NoPendingHostTransition")]
    #[link_name = "./ReactFiberConfig.js#NotPendingTransition"]
    safe static NO_PENDING_HOST_TRANSITION: Option<&'static Unknown>;
}
```

The `#[rust_js::name]` was read by nothing: the import was the export's
name, `import { NotPendingTransition }`, and a person reading the Rust
would think it named the JS. An attribute that silently does nothing is
a second way to write nothing.

**A binding's `#[rust_js::name]` is the name its import is given in the
module that declares it**, another module's own binding of the export
imported by its own, as ReactFiberHostContext imports `NotPendingTransition`
as it is, `import { NotPendingTransition as
NoPendingHostTransition }`, of a function's binding too, `ping as
pingHost`, and of a default's, which it names before the `static` holding
it does (`analysis/naming.rs`, `name_imports`), as `#[rust_js::name]` on
a `use` of one name names its import (ADR 0874, amended): what the item
is called in JS. **One of a global's binding, of a member of an export,
`chart.js#Chart.register`, or of a namespace's, `#*.path`, whose import
no name of its own holds, is rust-js's error**, naming the namespace's own
form, `#* as Name.path` (ADR 0256, amended); so is a second name of one
import (`given_imports`). A binding a module holds as its `const` is
named by its `#[rust_js::name]` there, `const ReactSharedInternals =
react.__CLIENT_INTERNALS..`, as before (ADR 0633), and names no import.

The alternative, `#NotPendingTransition as NoPendingHostTransition` in
the `link_name` as `* as Name` names a namespace, would be a second way to
give a binding the JS name `#[rust_js::name]` gives every other item.

Held by `test/modules.test.ts`, "a binding's rust_js::name names its
import"; mutations `binding-given-name-unread`,
`global-given-name-ignored`, `member-given-name-ignored`,
`two-given-import-names`, `held-binding-name-refused` and
`given-import-name-of-another-module` (`analysis/naming.ts`).
