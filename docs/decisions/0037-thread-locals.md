# 0037. `thread_local!` is a variable of its module

Status: Accepted.

Case: A, B, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A web program keeps state between events: how many times something ran,
the element currently on screen, a log. The playground keeps three such
values for its Result frame. In JS they're variables at a module's top level.

Rust keeps such state in a `static`. A `static` must be `Sync`, and JS values
aren't: an `&'static Element` can't go in a `Mutex`. So Rust programs on
the web keep JS values in `thread_local!`, as wasm-bindgen's documentation
recommends:

```rust
thread_local! {
    static COUNT: Cell<u32> = Cell::new(0);
}
```

`static` items were an error in rust-js (ADR 0016). ReScript has top-level
`let`s holding `ref`s, and Scala.js has `object`s with `var`s.

## Decision

**A thread-local is a `const` of its module, holding its value**, made from
the macro's initializer:

```rust
thread_local! {
    static COUNT: Cell<u32> = Cell::new(0);
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}
pub fn bump() -> u32 { COUNT.set(COUNT.get() + 1); COUNT.get() }
```

```js
const COUNT = { value: 0 };
const LOG = { value: [] };
export function bump() {
  COUNT.value = COUNT.value + 1 >>> 0;
  return COUNT.value;
}
```

- **JS runs a module on one thread**, so "one per thread" is "one", the
  module's own variable. It goes at the top of the file with the `const`
  items (ADR 0031), and is exported if another module uses it.
- **`thread_local!` expands to `const NAME: LocalKey<T>`**, whose block holds
  `fn __rust_std_internal_init_fn() -> T { init }` and std's storage for it,
  a `static` of each target's kind. rust-js lowers the `init` function, uses
  its body as the value, and leaves the storage out.
- **The methods**, on the `Cell` or `RefCell` it holds (ADR 0023):

  | Rust | JS |
  |---|---|
  | `KEY.get()`, `KEY.set(v)` | `KEY.value`, `KEY.value = v` |
  | `KEY.with(f)` | `f(KEY)` |
  | `KEY.with_borrow(f)`, `KEY.with_borrow_mut(f)` | `f(KEY.value)` |

  A closure that only returns is put in place, on the key or its value:
  `START.with(|s| s.get())` is `START.value`, not `((s) => s.value)(START)`.

## Why

- **It's how Rust programs for the web already keep state**, and rustc
  checks every use of it.
- **The JS is the variable a JS programmer would write**, with nothing of
  std's storage left.

## Alternatives

- **`static mut`**: plain, but `unsafe` at every use, and Rust 2024 warns
  about references to one.
- **`static` of a `Sync` type** (atomics, `Mutex`, `OnceLock`): right for
  numbers, but JS values aren't `Send` or `Sync`, so it can't hold an
  element.
- **Pass the state around**: works, but a web page's event handlers all need
  the same state, and that's what a module's variables are for.

## Consequences

- **A thread-local is made when its module loads**, not at first use as in
  Rust. An initializer with side effects, or one reading another module's
  thread-locals, runs at load. Initializers like `Cell::new(0)` and
  `RefCell::new(Vec::new())` can't tell.
- `with` and `with_borrow_mut` call their closure right there:
  `((log) => log.push(line))(LOG.value)`.
- Plain `static`s were still an error, until ADR 0096, and so are `take`
  and `replace` on a thread-local.

## Amendment: a `with` of one statement

`KEY.with(|it| { it.f(); })`, of a closure of one statement, is that
statement, the thread-local in its parameter's place: `KEY.f();`, as
react.dev's runESLint writes `linter.defineRules({..})`. It was the
closure called in place, `((it) => { it.f(); })(KEY)`.

## Amendment: a constant holds a thread-local as its variable

A constant's `&'static LocalKey<T>` is the thread-local's variable,
which every use of the constant shares as every use of the key does:
react.dev's MDXComponents table holds `pre: CodeBlock`, a memo
component, `export const MDXComponents = { .., pre: CodeBlock, .. }`.
It was refused as a constant of a type rust-js can't share. A JSX test
renders a table's memo; a mutation refuses it again.

## Amended by

- [0270](0270-module-variables.md): A thread-local only read and set is its module's variable
- [0306](0306-item-order.md): A module's items are where its Rust has them

## Amendment: one whose initializer runs the crate's code is made where it's first used

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what a thread-local's
initializer prints, a panic in it, and what it reads of another module
were where its module loaded; they're where Rust has them now. B for what
a binding's call does in one, which the user decided (2026-10-25).

A thread-local was made as its module loaded, its initializer the
module's code. Rust makes each at its first use. `thread_local!{ static
X: Cell<u32> = Cell::new(noisy()); }` printed `init` before `main`'s first
line, where Rust prints it at the first `X.with`; and in an import cycle,
one whose initializer called a function of a module not yet loaded threw
a `ReferenceError` reading that module's `const` (#207). React's contexts
are thread-locals too, `pub static TocContext: Context<Toc> =
create_context(Vec::new())`, which React writes as `export const
TocContext = createContext([])`, made as the module loads.

- **A thread-local whose initializer runs the crate's code, or may
  panic, is made where it's first used** (`analysis/lazy_locals.rs`): its
  initializer's MIR calls a body of the crate's, a function, a method of
  its impl or a closure, or of what rust-js can't see the body of; drops a
  value of the crate's with a destructor; checks what may fail; or calls
  what never returns. It's `$local(() => init)`, whose call makes it the
  first time and gives it after, read by its call, `source()`, the
  crate's or a library's (`Calling.lazy`, ABI 5); one whose initializer
  panics stays unmade, and its next use runs it again, as Rust's does.
  Its `with(|x| ..)` holds it first, `const x$1 = x();`, and of a closure
  that only reads a path of it, `|e| e.clone()`, is that path of its
  call, `source()`.
- **One that only makes a value, or calls std's or a binding's
  functions, is made as its module loads**, as JS writes it, React's
  `createContext([])`: the user's choice of the JS idiom where only JS's
  or std's code runs. What a binding's call does then, or a std call's
  panic, is at the module's load, the guide's difference 25.
- **A lazy one runs nothing as its module loads**, so a dictionary in an
  import cycle with its module isn't refused for it (ADR 0592): `counts`'
  thread-local of `shapes::total` compiles, as Rust's does.
- Held by `test/corpus/thread_local_made_where_used.rs`, against native
  Rust: an initializer's print after `main`'s, one's drop, one that
  panics made again, an import cycle's (#207), and another module's
  `const` read by its name; `test/lowering-structs-enums.test.ts`'s
  dictionary cycle, which compiles, and one of a binding's, refused; and
  the mutations `lazy-local-*`, `local-made-*`.
