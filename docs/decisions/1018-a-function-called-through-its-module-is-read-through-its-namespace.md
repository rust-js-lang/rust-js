# 1018. A function called through its module is read through its namespace

Status: Accepted (the user's decision, rd114). Amended (2026-10-29): a
function given as a value through its namespace is the namespace's
member, `Dep.noop`, not an arrow of its call. Extends
[0202](0202-imports-named-around-importers.md) (an import is named in
each file that imports it) and [0100](0100-separate-crates.md) (a
library's items imported by their module's file); beside
[0633](0633-a-binding-named-through-a-module-is-read-through-it.md),
where a binding is read through the module a path names.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. An ES module's
namespace object holds a live binding of each export, so `Plugin.f` is
the binding `import { f }` reads, read at the same moment, and an export
not made yet throws as its named import would. The one difference, the
`this` a call through a member gives, is unseen: a crate's or a
library's free function never reads `this`, only a class's method and a
closure given `with_this` do, and neither is called by a module path.

## Context

React DOM's DOMPluginEventSystem calls each plugin through its module:

```js
import * as EnterLeaveEventPlugin from './plugins/EnterLeaveEventPlugin';

EnterLeaveEventPlugin.extractEvents(dispatchQueue, domEventName, ..);
```

The port writes the Rust for it, `use crate::enter_leave_event_plugin;`
and `enter_leave_event_plugin::extract_events(..)`. rust-js imported
every function by its name, so it printed `import { extractEvents as
extractEvents$1 } from "./plugins/EnterLeaveEventPlugin.js"`, renamed
apart from the module's own `extractEvents`, and `extractEvents$1(..)`:
a name the program never wrote, and the module the program named lost.

## Decision

**A function named by a path through the module that has it, with no
`use` of the function, `plugin::extract_events(..)`, is read through the
module's namespace, `import * as Plugin from "./plugin.js"` and
`Plugin.extractEvents(..)`; one the program `use`s, `use
plugin::extract_events; extract_events(..)`, is its named import, as
before.** The Rust differs, so the JS does: not two ways for one thing.

```rust
mod plugin;
use plugin::name;

fn run() -> u32 {
    plugin::extract_events(2) + name().len() as u32
}
```

```js
import { name } from "./plugin.js";
import * as Plugin from "./plugin.js";

function run() {
  return (Plugin.extractEvents(2) + name().length) >>> 0;
}
```

- **Which paths.** One whose last but one segment names a module by its
  name, `plugin::f`, `crate::events::plugin::f` and `super::plugin::f`
  alike, of a function that module has itself, called or given as a
  value, `.map(plugin::doubled)`. Not `crate::f`, `self::f` or
  `super::f`, which name no module but the path's own place; not a
  function another module has that the named one re-exports; not a
  type's method, `plugin::Plugin::new()`, which its type's object has;
  not a function its module exports as its default, which is what the
  module is. A thread-local, a `static` and a `const` read through its
  module are too (amended below, rd114a).
  `analysis/namespaces.rs`'s `module_paths` finds them in HIR, by the
  path's span and its call's, which a call's MIR is at; lowering asks it
  (`FnCx::named_fn_ref`) and makes the read `Plugin.f`, a member of the
  module's `js::Symbol` whose export is `js::NAMESPACE`, `*`, as a
  `Reexport`'s namespace is.
- **One import of the namespace per module and file**, its own line
  after the module's named import where it has one, as a package's
  namespace is (`js::Import::namespace`, `js::Package::namespace`).
- **Its name** is the module's Rust name camelCase, its first letter a
  capital (`lower::namespace_name`): `enter_leave_event_plugin` is
  `EnterLeaveEventPlugin`, as JS names a namespace after its module; a
  library's root by its crate's name, `Scheduler` of `scheduler::f()`.
  Given apart where the file has the name already, `Other$1` beside a
  struct `Other`'s object, as the linker gives every import apart.
- **A library's**, whose manifest lists the function: through the
  namespace of the JS module the importer would import the function
  from (`Foreign::import_key`), its module's file, or its package by the
  name where the path is through the library's root and the root
  re-exports it (ADR 1008), `Dep.helper()`; by the name it's exported
  as there.
- **Read as a name is**: `Plugin.f` is an item (`Known::Item`), the same
  wherever it's read, and a callee named by it is one a forwarding arrow
  passes on to (`Expr::is_named`), `.map(Plugin.doubled)`.

## Why

- **It's upstream's JS**, of the Rust the port naturally writes; the
  user decided it (rd114).
- **It keeps the name the program wrote**: the module, where a named
  import of a common name, `extractEvents`, was renamed apart.
- **A `use` stays a named import**, so a program says which it means,
  as a JS author does by which `import` they write.

## Alternatives

- **Only where the named import would be renamed.** Less churn, but
  whether a call is through its namespace would hang on what else the
  file names: not one rule. Rejected.
- **The module's file's name**, `DOMPluginEventSystem` of
  `#[path = "DOMPluginEventSystem.rs"] mod dom_plugin_event_system`.
  Upstream's names are its files'; most are the Rust name's PascalCase
  too, but one with an acronym differs, `DomPluginEventSystem`. The
  name is chosen as the linker names imports, before artifact planning
  places files. Not taken; a port that meets one asks.
- **A thread-local, `static` or `const` read through its module by its
  name**, `import { current }` beside `import * as Fiber`. Taken at
  first, then rejected by the user (rd114a, below).

## Consequences

- Every corpus case and guide example that calls `module::f()` reads
  through the module's namespace; the readability ratchet's renamed
  imports fall with them.
- **A value of a function a `let` names through its module**, `let
  Selected = ui::Empty;`, is `import * as Ui` and `const Selected =
  Ui.Empty;`, what a JS author with a namespace import writes, where it
  was folded into `import { Empty as Selected }`. Accepted by the user
  (2026-10-29), held by the JSX snapshot `spreads-and-paths`.
- Bindings keep their own forms: a binding's `link_name` says its JS,
  `#*` its namespace (ADR 0256), which this doesn't touch.

## Amendment: a function as a value through its namespace is the member (2026-10-29)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, as the decision
says: the member is the binding the named import was.

A library's function given as a value through its module's path,
`&dep::noop` as a `&'static dyn Fn() -> u32`, was `return noop;` while
it was imported by its name, and became `return () => Dep.noop();`: the
arrow rust-js makes of a function as a value (`passed_on::Taker::FnValue`)
was its call's function only where that was a variable. **One read through
its module's namespace, `Dep.noop`, an item's member (`Expr::is_named`),
is taken as the variable is:** `return Dep.noop;`.

Main's tests of a library's function called by its path, in
`test/crates.test.ts`, `test/rest-parameters.test.ts` and
`test/react.test.ts`, read it through its namespace now, `Dep.createContext(1)`
and `import * as People from "./people.js"`, as the decision makes it.

- Held by `test/crates.test.ts`, "a library's function returned past a
  guard's drop is returned after its finally", run: mutation
  `namespace-fn-value-arrowed` (`lower/passed_on.ts`).

## Amendment: any value read through its module (rd114a)

The user's decision (2026-10-29). A file read `fiber::f()` through
`import * as Fiber` and `fiber::CURRENT.get()` through `import {
current }` of the same module: two import forms of one module, for one
way of writing the Rust.

**Any value a path names through the module that has it is read through
the module's namespace: a function, a `const` rust-js prints by its
name, a `static`, a `static mut` and a thread-local**, which
`thread_local!` makes a `const`. `fiber::CURRENT.get()` is
`Fiber.current`, `counter::LINES` is `Counter.LINES`, `dep::NAME` of a
library's root `Dep.name`. One the program `use`s is its named import,
as a function is. A `const` written as its value, `5`, names nothing.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. A namespace
object's property is the export's live binding, as its named import is:
a thread-local or a `static mut` its module sets, `export let`, read
through `Fiber.current` after `Fiber.setCurrent(3)`, is 3, as ES
specifies and as `exported_cells` and `module_path_namespace` check
against native Rust. Only its module writes it, as before
(`outside_writes`).

- **Not a value that may hold a function a call of it runs**, a `dyn
  Fn`, a `fn` pointer or a closure, `REPORT: Box<dyn Fn() -> bool>`:
  read by its name, `report()`, as before. Through the namespace its
  call would be `Report.report()`, which gives the function the
  namespace object as its `this`, where Rust gives a `with_this`
  function `None` (ADR 0972): a wrong answer (case: results change, so
  Rust's behavior, by the named import). Its module's other values are
  read through the namespace still (`namespaces::holds_a_callable`, by
  the type's generic arguments, a thread-local's by its key's `T`). A
  field's function called as a member, `(s.f)()` as `s.f()`, gives `s`
  as its `this` the same way, filed as #402.
- **Where it's read**: lowering asks for each item by the span of the
  path that names it (`FnCx::item_read`, and `named_fn_ref` of a static's
  reference), as it asks for a call's.
- **Its module's variable read through the namespace is the variable**,
  `Fiber.current`, not its cell's `.value`: `cell::imported_variables`
  sees `Fiber.current` as it sees `current`, of the crate's
  (`js::Symbol` of `*`, by what the module exports it as) and of a
  library's (the namespace's import, `Dep.name`).

Held by `test/corpus/module_path_namespace.rs` (a thread-local read
before and after its module sets it, a `static`, a `const` array), the
re-blessed `exported_cells` and `atomics`, and
`test/lowering-module-variables.test.ts`'s library and `static mut`
tests and its thread-local of a `with_this` function, which run the JS,
and `test/crates.test.ts`'s pairs, which run against native Rust; the
mutations `namespace-of-functions-alone`, `namespace-of-a-callable-value`,
`namespaced-variable-held`, `namespaced-library-variable-held`,
`static-read-by-name-through-its-module` and
`item-read-by-name-through-its-module`.

## Amendment: every namespace is named PascalCase, after its JS file (rd114b/c)

The user's decisions (2026-10-29). A crate's module's namespace was its
Rust name PascalCase, `DomPluginEventSystem` of `#[path =
"src/events/DOMPluginEventSystem.rs"] mod dom_plugin_event_system`,
where upstream's file, and its JS, is `DOMPluginEventSystem`; and a
binding's namespace (ADR 0256) was camelCase, `import * as scheduler`,
where React writes `import * as Scheduler`: two rules for one name.

**Every namespace import is named by its JS module's file, PascalCase**
(`lower::namespace_name`, the one rule): a crate's module's by the
file its JS is written to, a `#[path]` module's file's stem,
`DOMPluginEventSystem`, any other's its Rust name, `EnterLeaveEventPlugin`
of `enter_leave_event_plugin` (`lower::module_file_stem`, as ADR 0273
places its JS); a library's module's by the file it's imported from,
`Events` of `../dep/events.js`, its root's by its crate's name, `Dep`;
a binding's by what it's imported from, `Scheduler` of `scheduler`,
`ReactContextMenu` of `@radix-ui/react-context-menu`, or the module of
its bindings alone, `SideMenu` of `mod side_menu` (ADR 0256); and the
test runner's file's import of a module of `#[test]`s, `Tests` of
`tests.js` (`program::TestFn::namespace`, ADR 0026). Given apart on a
clash, as every import is. This settles c06.

**A `link_name`'s `* as Name` names it otherwise only**, `react-dom#* as
ReactDOM` of a name its file's words don't spell: one that names it as
it's named anyway, `scheduler#* as Scheduler`, is an error naming
`scheduler#*.path`, as a `#[rust_js::name]` that repeats its default is.

Case ([0262](0262-when-rust-and-js-disagree.md)): none, a local name.

Held by `test/modules.test.ts` (a `#[path]` module's
`DOMPluginEventSystem`, `scheduler#*`'s `Scheduler`, `* as Sched`, the
repeat refused, `SideMenu`), `test/lowering-bindings.test.ts`
(`React.__CLIENT_INTERNALS..`), `test/compiler.test.ts` (`Greet`), the
snapshot `imports` and the guide's bindings page, compiled; and the
mutations `namespace-named-by-rust-name`, `library-root-namespace-by-file`,
`namespace-named-camel-case`, `namespace-name-repeat-accepted` and
`namespace-of-any-holding-module`.

## Amendment: a function a module re-exports, through that module (rd137)

React port gap rd137 (2026-10-29). DOMPluginEventSystem's port calls
`simple_event_plugin::register_events()`, which SimpleEventPlugin
re-exports, `pub use crate::events::dom_event_properties::register_simple_events
as register_events;`. "Which paths" above left out a function another
module has that the named one re-exports, so it was that function's
named import, `import { registerSimpleEvents } from "./DOMEventProperties.js";`,
where upstream writes `SimpleEventPlugin.registerEvents();`.

**A function of the crate's that a module's file re-exports, named
through that module's path, is read through that module's namespace, by
the name it re-exports it as** (`reexports::exported_in`, the name
`export { registerSimpleEvents as registerEvents } from ..` gives it):
`SimpleEventPlugin.registerEvents()`. Which `pub use`s its file
re-exports is `analysis::is_reexport`'s, asked of the module by
`reexports::reexporting_use`; one it doesn't, a `pub(crate) use`, stays
the function's named import, as `plugins.js` exports nothing of it. A
path in the re-exporting module itself, and a copy (ADR 0051), stay
their named import. The reading module imports the re-exporting one
(`reexporters`, its dependencies): its namespace is offered for import, and
the load-order graph has that import, as JS does. Case
([0262](0262-when-rust-and-js-disagree.md)): none, the same function.

Held by `test/modules.test.ts`, "a function called through its module is
read through its namespace" (`Plugins.doubled`, `Plugins.twice`, and a
`pub(crate) use`'s named import), and `test/lowering-module-variables.test.ts`,
"a function read through a module that re-exports it imports that module";
mutations `reexported-path-*`, `reexporter-*`, `namespace-through-reexport`.

