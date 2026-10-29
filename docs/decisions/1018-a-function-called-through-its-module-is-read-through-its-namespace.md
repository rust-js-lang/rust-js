# 1018. A function called through its module is read through its namespace

Status: Accepted (the user's decision, rd114). Extends
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
  module is; and not a thread-local, a `static` or a `const`, which are
  read by their names as before (the user's decision covers calls).
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
- **A thread-local, `static` or `const` read through its module too.**
  `fiber::CURRENT.get()` stays `import { current }` beside `import * as
  Fiber`. Left for the user: the namespace read is live, so it would be
  correct, but it changes what the decision covers.

## Consequences

- Every corpus case and guide example that calls `module::f()` reads
  through the module's namespace; the readability ratchet's renamed
  imports fall with them.
- Bindings keep their own forms: a binding's `link_name` says its JS,
  `#*` its namespace (ADR 0256), which this doesn't touch.
