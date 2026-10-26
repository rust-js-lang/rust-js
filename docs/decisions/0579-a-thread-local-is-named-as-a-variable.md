# 0579. A thread-local is named as its module's variable

Status: Accepted. Amended by [0619](0619-a-primitive-thread-local-is-a-constant.md): one of a primitive keeps its name, as a constant. Amended by [0922](0922-an-export-is-named-first.md): an exported one keeps its Rust name only beside another export. Amends [0573](0573-every-rust-name-is-camel-case-in-js.md),
which left a name with no lowercase letter as it was; extends
[0270](0270-module-variables.md), whose thread-locals are their module's
variables.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A name changes
nothing that runs.

## Context

React's scheduler keeps its state in module variables:

```js
var taskQueue = [];
var taskIdCounter = 1;
var currentPriorityLevel = NormalPriority;
```

Its Rust is a `thread_local!` each, named in `SCREAMING_SNAKE` as rustc's
`non_upper_case_globals` has every static: `TASK_QUEUE`, `TASK_ID_COUNTER`.
ADR 0573 made every Rust name camelCase but kept one with no lowercase
letter as written, so the JS was `let TASK_ID_COUNTER = 1;`: a variable
named as JS names a constant.

## Decision

**A `thread_local!` is named as JS names a module's variable**:
camelCase of its `SCREAMING_SNAKE` words, `TASK_QUEUE` as `taskQueue`
(`variable_case`, beside `camel_case`); capitalized where it holds a
component, `THEME_CONTEXT` as `ThemeContext`, as JSX takes a component's
name. React says which types are components, `#[rust_js::component]`: a
context, and `memo`'s, `lazy`'s, `forward_ref`'s and a component value.

- **One named after another item of its module keeps its Rust name**: a
  thread-local read by a function of its name, `CACHE` of `cache()`, is
  `CACHE`, as JS keeps two names apart, not `cache$1`. (Amended by
  [0922](0922-an-export-is-named-first.md): an exported one only beside
  another export; a private item of its camelCase name is named apart,
  as its importers read the export by that name.)
- **A `const` is a constant, named as written**, `MAX_SIZE`: JS's own
  convention, and what keeps `START` apart from the `start()` that
  returns it. One that must be another says so, `#[rust_js::name =
  "maxSigned31BitInt"]`.

## Why

- **A name is the JS construct's**: a thread-local is a `let` (ADR 0270),
  and JS names a variable in camelCase; a `const` is a constant.
- **The convention people know**: Rust's `SCREAMING_SNAKE` for a static
  is the lint's, not a statement that it's constant; JS's camelCase
  `taskQueue` is what React and every JS module write.

## Alternatives

- **Every `SCREAMING_SNAKE` name camelCase, constants too**: React's
  `maxSigned31BitInt` without an attribute, but `const GREETING` beside
  `fn greeting()`, an ordinary Rust pair, became `greeting$1`, and a
  component `fn H2` `h2`, an HTML tag. Rejected.
- **`#[rust_js::name]` on each thread-local**: what every program writes
  for a name rust-js should know. Rejected.
- **Capitalize a thread-local by its use as a JSX tag**: a name is made
  where its item is, not where it's used. Rejected for React's marker.

## Consequences

- A closure a thread-local's `with` is given, `|seen| ..`, whose
  parameter is the variable's name, is renamed, `seen$1`, until that
  closure is the variable itself.
- 12 corpus cases and snapshots, and the tests that held a thread-local's
  name, read its camelCase.

## What holds it

- `test/lowering.test.ts`, "a thread-local's SCREAMING_SNAKE name is
  camelCase, a constant's as written".
- `test/jsx.test.ts`'s contexts and components, `<Theme.Provider>` and
  `<ForwardedInput>`; `test/modules.test.ts`'s `CACHE`-style names kept.
- Mutations: a thread-local left as written, a component left lowercase,
  a name another item has taken.
