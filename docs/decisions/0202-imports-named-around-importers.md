# 0202. An import is named in each file that imports it

Status: Accepted. Amends [0028](0028-js-module-imports.md). Amended: a module's own item renamed twice imports nothing (below).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An import's name is the same in every file that has it, unique among the
crate's imports, and was named around every module's items. react.dev's
`ButtonLink` imports next/link's default, `Link`, and its `IconLink`
module has a function of its own, `fn Link`, which ButtonLink's file
never sees: every file's import was `import Link$1 from "next/link"`.

Named around only the modules that import it, it still was, once one of
them had its own: react.dev's `MDX/Link` imports next/link beside its
`fn Link`, and `Breadcrumbs`, `ButtonLink`, `DocsFooter` and `BlogCard`
each had `import Link$1 from "next/link"`. (Amended.)

## Decision

**An import is named in each file, around the globals, that file's items
and its other imports: another module's item of its name is no reason to
rename it, and one of the file's own renames it there only.**

```js
// components/ButtonLink.jsx, beside components/MDX/Link.jsx's `function Link`
import Link from "next/link";
```

```js
// inner/leaf.js, of its own `fn join` and a `path_join` that's node:path's `join`
import { join as join$1 } from "node:path";

function join(greeting) {
```

- **A default import is named as the file's `use` renames it**: `use
  next::link::Link as NextLink` is `import NextLink from "next/link"`, as
  react.dev's MDX `Link` names it beside its own; a `static`'s is camel
  case, as an asset's own name is. (Amended.)
- **Every module names every import**, so a module's locals avoid it, as
  they did: a library's export (ADR 0100), or one a trait's default
  copied into an impl uses (ADR 0049), is used by modules known only once
  they're lowered.

## Why

- **A name is the file's**: one in a file that doesn't import it is no
  name of that file's import, nor is one another file renames. A module
  of the crate's own, imported by the importing one, is the linker's to
  alias, as it was.
- **It's tested by the `imports` snapshot**: `inner/leaf.rs` has a `fn
  join` of its own and imports `node:path`'s `join` too, `join$1` there,
  and the root's `import { join } from "node:path"` keeps its name.
  A JSX test's module renaming next/link's default has `import NextLink
  from "next/link"`, another's keeps `Link`, and a `static` renamed
  `banner_img` is `bannerImg`.

## Amendment: a crate module's named import by its `use`'s name

A `use` that renames another module's function of the crate imports it
by that name, `use inner::client as sandpack` is `import { client as
sandpack } from "./inner.js"`, as react.dev's MDXComponents imports
`{SandpackClient as Sandpack}` and keeps `Sandpack` in its table, a
default import already was. A module test imports two so; a mutation
drops the name. The JSX snapshots that rename a component, `Card as
Panel`, `MEMO as Cached` and `THEME as Theme`, print those names.

## Amendment: every import by its `use`'s name

A `use .. as` names every import it renames, not only a crate module's
function and a binding's default: a library's item, its default too, and
a binding's named export. React's ReactAct writes `import queueMacrotask
from 'shared/enqueueTask'`, shared's default, and its port's `use
shared::enqueue_task::enqueue_task as queue_macrotask;` imported
`enqueueTask` and called it so; `use react::use_id as make_id` imported
`useId`. Each is now the name the module's `use` gives it,
`import queueMacrotask from "../shared/enqueueTask.js"` and `import {
useId as makeId } from "react"`, in that file alone. A binding of a
member of an export, `./math.js#tools.scale`, renames no import: the
export is `tools`, and `use bound::scale as scaled` reads
`tools.scale(x)`. A `use`'s `#[rust_js::name]` is the name, as ADR 0874
has it, a library's default's too: `#[cfg_attr(rust_js, rust_js::name =
"queueMacrotask")]` was ignored there, silently. Two `use`s that give
one item two names in one module are refused, as its file imports it
once: either name would leave the other unsaid. Case: N (ADR 0262): a file's name of its import is no
behavior; the call is the same function's.

Tests: `cross-crate-items` imports a library's default and its function
by their `use`s' names; `modules` a binding's default, its named export,
and a member's export by its own name, and refuses two names of one.
Mutations: each of the three left out, a member's `use` naming its
export, and two names kept.

## Amendment: a `use`'s name is cased as its item's own

A `use .. as` name was camelCase as any Rust name is, `camel_case`, which
leaves a word with no `_` as it's written: `use shared::object_is::OBJECT_IS
as IS;`, shared's default thread-local, was `import IS from
"../shared/objectIs.js"` and `IS(newValue, oldValue)`, where the
thread-local's own name is `objectIs`, `SCREAMING_SNAKE` words lowercase
then camelCase (ADR 0573), and shared's own `shallowEqual` reads it `is`.

**A `use`'s name is written as its item's own name would be**
(`bindings::cased_as`, which an item's default name asks too): a
thread-local's as its module's variable, `is`; one of a primitive as it's
written, a constant; anything else camelCase, as before. Case: N (ADR
0262), a file's name of its import.

Test: `cross-crate-items`, "a use's name of a library's thread-local is
cased as the thread-local's own": `import is, { CALL_LIMIT as MAX_CALLS }
from "../dep/object_is.js"`, run. Mutation: `renamed-thread-local-as-written`.

## Amendment: a module's own item, which its file declares

Case ([0262](0262-when-rust-and-js-disagree.md)): none; a program rustc
accepts, `privacy/pub-restricted-path-usage-55376.rs`, was refused.

`pub(super) use self::Foo as Bar;` and `pub(in super::my_mod) use
self::Foo as Baz;` of a module's own `Foo` were refused as one import of
two names, but a module's file declares its own items and imports none of
them: its JS reads `count` by its own name, `export function count()`,
wherever a `use` renames it.

**The refusal is of a `use` of another module's item** (`naming.rs`'s
`refuse_two_names`), one the file imports. Held by corpus
`use_renames_own_item`, against native Rust, and the mutation
`own-item-renames-refused`; `test/modules.test.ts`' "two uses naming one
item apart are refused" still holds the refusal.

## Amendment (2026-10-29): a `use`'s name of an item its `#[rust_js::name]` names is cased as that name

React DOM's ReactDOMEventListener writes upstream's `import {
IdlePriority as IdleSchedulerPriority, getCurrentPriorityLevel as
getCurrentSchedulerPriorityLevel } from 'scheduler'` as `use
scheduler::{IDLE_PRIORITY as IDLE_SCHEDULER_PRIORITY,
GET_CURRENT_PRIORITY_LEVEL as GET_CURRENT_SCHEDULER_PRIORITY_LEVEL}`, of
thread-locals the scheduler names `IdlePriority` and
`getCurrentPriorityLevel`. The alias was cased as the item's default name
is, a primitive thread-local's as written, so it imported `IdlePriority as
IDLE_SCHEDULER_PRIORITY` (React port gap rd79).

**The name a `use .. as` renames an item to is cased as the item's own JS
name is, where its `#[rust_js::name]` gives it one** (`bindings::alias_cased`):
PascalCase of a PascalCase one, `IdleSchedulerPriority`; camelCase of a
camelCase one, `getCurrentSchedulerPriorityLevel`; as written of a
SCREAMING one, `MAX_LANE`'s `TOP`. A name of none of these cases,
`$$typeof`, says nothing, and the item's kind decides, as before. The
item's own default name is unchanged: a `#[rust_js::name]` repeating it
is still refused.

Held by `test/cross-crate-items.test.ts`'s "a use's name of an item its
rust_js::name names is cased as that name" (in the library and its
consumer, run) and the mutations `alias-cased-as-own`,
`alias-pascal-camel` and `alias-screaming-cased`.

## Amendment (2026-10-31): a module namespace's import by its struct's `use`'s name

A `use` that renames a `#[rust_js::namespace]` struct names its module's
namespace import, `use ..::ReactFiberConfigNoop as DefaultConfig` `import
* as DefaultConfig from "./ReactFiberConfigNoop.js"` (rd292b,
[ADR 1037](1037-a-module-namespace-is-a-unit-struct-of-its-functions.md),
amended).

