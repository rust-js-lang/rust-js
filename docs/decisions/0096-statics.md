# 0096. A `static` is its module's value, and a `static mut` its `{ value }`

Status: Accepted. Extends [0031](0031-consts.md) and [0037](0037-thread-locals.md).
Amended: a `static mut` whose value rustc can't say is its initializer's,
in its `{ value }`, and a static that refers to another's number is that
number; a `static mut` is its module's variable, `export let` where it's
`pub`, which only its module writes, and a raw pointer to one a handle on
its place (below).

Case: C, A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`static` items are an error (ADR 0031, 0037). They're how Rust keeps one
value for the whole program: a table, a name, a counter. 156 of rustc's
tests stop at them, and what they use says what matters:

| What the tests use | Tests |
|---|---|
| immutable statics, and nothing below | 101 |
| `static mut` | 39 |
| a static holding a reference, `static S: &T` | 16 |
| atomics, `AtomicUsize` and the like | 9 |
| `extern` statics, thread-local statics, raw addresses | 2 each |
| `Mutex`, `RwLock`, `OnceLock`, `LazyLock`, threads | none |

A `static` differs from a `const` in two ways that JS can keep: it's one
place, so `&X` is the same every time, and a `static mut` can be written.
Its value, as a `const`'s, is worked out by rustc at compile time.

## Decision

**An immutable `static` is a `const` of its module, holding the value
rustc computed** for it, written as a `const`'s is (ADR 0031): numbers,
strings, arrays, objects. It's declared once, so `&X` is that one value
every time, and an object in it is one object.

```rust
static ORIGIN: Point = Point { x: 0, y: 0 };
pub static NAMES: [&str; 2] = ["a", "b"];
pub fn moved(dx: i32) -> Point { let mut p = ORIGIN; p.x += dx; p }
```

```js
const ORIGIN = { x: 0, y: 0 };
export const NAMES = ["a", "b"];
export function moved(dx) {
  let p = { ...ORIGIN };
  ...
}
```

- **A read of it is a read of a place:** a `Copy` value that the crate
  changes in place somewhere is copied as it's read (ADR 0020), as a
  `const`'s is.
- **It goes at the top of its module with the `const` items,** is exported
  when it's `pub` or another module uses it, and one inside a function goes
  there too, as a `const` does.
- **A value rustc's can't say is its initializer, lowered as code,** a
  static's or a `const`'s: a function, a closure, a `dyn`, which a value
  tree has no pointer for, and one too large for a value tree:

  ```js
  const F = double;
  const TABLE = [["double", double], ["triple", triple]];
  const GREETER = { value: undefined, impl: enGreet() };
  const DATA = new Array(200000).fill(42);
  ```

  Rust makes a `const`'s value afresh where it's used, and JS has one: it's
  every use's where its type is shareable, functions, closures, shared
  references, numbers and strings, and arrays, tuples, `Option`s and the
  crate's own types of those, which are copied where they're read if
  anything changes one in place. An initializer that reads a static is
  still an error: that static's value may not be made yet where the
  module loads it. (Amended: each was an error.)

**A `static mut` is its module's `{ value }`** (amended below: its module's
variable, `let COUNT = 0`), as a thread-local's `Cell`
is (ADR 0037), holding the value rustc computed for it:

```rust
static mut COUNT: u32 = 0;
pub fn bump() -> u32 { unsafe { COUNT += 1; COUNT } }
```

```js
const COUNT = { value: 0 };
export function bump() {
  COUNT.value = COUNT.value + 1 >>> 0;
  return COUNT.value;
}
```

- **Reads and writes are `COUNT.value`,** from any module (amended below:
  another module reads it by its name, and can't write it): an imported JS
  binding can't be assigned, and an object's field can.
- **A shared reference to one, `&COUNT`, is the value it points to**, as
  any shared reference is (ADR 0023): a number's is its value then, and an
  object's is the object. Rust makes writing the static while such a
  reference is still used undefined behavior, so no program Rust defines
  can tell. `println!("{}", COUNT)` takes one, as does a method of
  `&self`. (Amended: the first version rejected every reference; 22 of
  rustc's tests stopped there, most of them counting drops; 8 more pass,
  and of the 15 left, 6 stop at what destructors don't do yet, ADR 0098.)
- **`&mut COUNT` and `&raw mut COUNT` are rejected** for now (amended below:
  `&raw mut COUNT` is a handle on its place): a `&mut` of a
  number has no JS value yet.

**An atomic is a `{ value }` too, as a `Cell` is**, as `static N:
AtomicUsize = AtomicUsize::new(0)` is `const N = { value: 0 }`. JS runs a
module on one thread, so every ordering holds, and each operation is the
plain one:

| Rust | JS |
|---|---|
| `N.load(o)`, `N.store(v, o)` | `N.value`, `N.value = v` |
| `N.swap(v, o)` | the old value, and `N.value = v` |
| `N.fetch_add(v, o)`, `fetch_sub`, `fetch_and`, `fetch_or`, `fetch_xor`, `fetch_max`, `fetch_min` | the old value, and `N.value` changed as the integer type wraps (ADR 0011) |
| `N.compare_exchange(a, b, o1, o2)` and `_weak` | `Ok(old)` and `N.value = b` if it was `a`, else `Err(old)` |

A 64-bit atomic's value is a BigInt (ADR 0086). The orderings, and what
their arguments do, are evaluated, as any argument is.

**Rejected, with an error that says so:** an `extern` static (a foreign
symbol), a `#[thread_local]` static (unstable), a raw address of any
static (`&raw const X`, `addr_of!`; amended below: of a `static mut`, a
handle on its place), and a static whose value has no value
tree and isn't an atomic's, as one holding a reference to another static,
a function pointer, or a `dyn`.

## Why

- **It's what Rust means:** one place, whose value is fixed at compile time
  and, for a `static mut`, written where rustc's `unsafe` allows it.
- **Nothing depends on load order:** the values are rustc's, so statics,
  as constants, don't read each other as modules load (ADR 0031).
- **The JS is what a JS programmer would write for state:** a module's
  `const`, and a `{ value }` where it changes, which a JS caller can read
  and write too.

## Alternatives

- **Lower the initializer, as thread-locals are:** it would show how the
  value was written, but statics would have to be ordered, and couldn't be
  across modules that import each other (ADR 0031's reason).
- **A `static mut` as a module's `let`:** it reads better, `COUNT += 1`,
  but only its own module can assign it, and a `let` another module writes
  would need a setter.
- **Leave `static mut` out:** Rust discourages it, but 39 of the tests use
  it, and so does code written before thread-locals and atomics.

## Consequences

- A `static` is made when its module loads, from a literal, so there's
  nothing for it to run.
- `Sync` means nothing on one thread: a static rustc accepts is one JS
  value, whatever its type's `Sync` says.
- A `pub static mut` is an exported `{ value }` that JS can change (amended
  below: its `export let`, which only its module writes), as
  Rust code can, in `unsafe`.
- **A `static mut` whose value rustc can't say**, a function's, or a
  struct of one, is its initializer's, as a `static`'s is, in its `{ value
  }` (amended, 2026-10-15): one JS cell is every use's, whatever its type,
  as its uses read and write its `value`; one its module alone uses is a
  `let`, as any is. It was refused; the corpus's `static_mut_fns.rs`, and
  rustc's `statics/static-function-pointer.rs`, `consts/const-fn-val.rs`
  and `consts/const-vec-of-fns.rs`, pass.
- Left for later: a `&mut` to a `static mut`, a static holding a
  reference to another's object, and `Mutex` or `OnceLock` in a static,
  which none of the tests use.
- **A static that refers to another's number**, `&A.count` or
  `&LIMITS[0]` (amended, 2026-10-15), is that number, read out of the other
  static's memory: a reference is its value (ADR 0023), and an immutable
  static with no cell in it can't change, so no program tells the copy
  from it. One to another's object, which `ptr::eq` would tell apart, is
  still an error. The corpus's `static_refs_to_statics.rs`, and rustc's
  `consts/issue-17718-borrow-interior.rs`, pass.
- **rustc makes value trees only of constants,** so a static's is read out
  of the memory rustc computed for it, a field at a time with rustc's own
  destructuring of a constant, following each `&` in it. That gives a
  struct a variant index too, which only an enum's value tree has: the
  first version read `Point { x: 3, y: 4 }` as `{ x: 0, y: 3 }`, and
  `statics.rs` found it.
- An atomic is `{ value }` wherever it is: `new`, `default()`,
  `into_inner` and `{:?}` are a `Cell`'s. `fetch_nand`, `fetch_update`,
  `get_mut` and `AtomicPtr` are rejected for now.
- **Of rustc's tests, 76 more pass** (1,411 of 2,691), each of which had
  stopped at a static; 29 still stop at what's rejected here, most often a
  function pointer in a static (9) or a reference to a `static mut` (7).
- A `thread_local!`'s own storage is a static std writes inside it, which
  JS needs none of (ADR 0037): it isn't one of the crate's statics.
- `#[thread_local]` is unstable, so the corpus, whose cases are modules,
  can't enable it: its rejection is a diagnostics test.

## Amendment: a `static mut` is its module's variable, which only its module writes

Case ([0262](0262-when-rust-and-js-disagree.md)): C for what a JS importer
reads, a box where Rust has the value, which Rust's behavior keeps; A for
the rest, each module reading and writing what it did. The user's choice
(2026-10-26): the rule ADR 0270, amended, made for an exported thread-local
of a value, one rule for a module's state, not a second mechanism.

A `pub static mut LINES: usize` was `export const LINES = { value: 0 }`,
which any module wrote, `LINES.value = 10`: a JS module importing it read
the box, its `.d.ts` said `number`, and a library's consumer couldn't read
it at all, "this constant, from its MIR". A thread-local of a `Cell` is its
module's `export let` since ADR 0270's amendment, which only its module
sets; a `static mut` had two forms of one job beside it.

**A `static mut` is its module's variable, as a thread-local of a value
is:** a `let`, `export let` where it's `pub` or another module reads it,
`const` where nothing writes it whole; named as written.

```rust
pub mod counter {
    pub static mut LINES: usize = 0;

    pub fn line() -> usize {
        unsafe {
            LINES += 1;
            LINES
        }
    }

    pub fn set_lines(lines: usize) {
        unsafe { LINES = lines }
    }
}
```

```js
export let LINES = 0;

export function line() {
  LINES = (LINES + 1) >>> 0;
  return LINES;
}

export function setLines(lines) {
  LINES = lines;
}
```

- **Its module writes it**, whole, `STATS = { calls: 0 }`, or a field or
  an item in place, `STATS.calls = ..`, `SLOTS[i] = ..`; its `.d.ts` is
  its type, `export let LINES: number;`.
- **Another module reads it by its name**, `import { LINES }`, ES's live
  binding, `Known::Live` as an exported cell's read is
  (`cell::imported_variables`); a library's manifest says it's a variable,
  `Calling.variable`, so its consumers read it so too.
- **Another module's write is refused**, in the crate or a consumer's:
  whole, `counter::LINES = 3`, `+= 1`, a field's or an item's, or a raw
  pointer taken to it, `&raw mut counter::LINES` or `addr_of!`
  (`analysis/plain_locals.rs::outside_writes`). The error says to write a
  `pub fn` in its module that sets it, as JS's module exports one.
- **A raw pointer to it is a handle on its place** (ADR 0407), as one to a
  local is: `&raw mut COUNT` and `addr_of_mut!(COUNT)` were refused as an
  address. One read or written through where it's taken, `*addr_of_mut!
  (COUNT) += 2`, or reborrowed there, `&mut *addr_of_mut!(COUNT)`, is the
  variable read or written. **One given on**, `add(&raw mut TOTAL, 3)`, is
  `$ptr(TOTAL, "value")`, a handle on its box: its module keeps it its
  `{ value }` (`plain_statics` reads it whole), and **one it exports is
  refused at its declaration**, as an exported cell its module needs the
  cell of is, saying to read and write it by its name, or through a
  pointer only where it's taken, or to keep it private and export a `pub
  fn` that reads it. A pointer to an immutable `static` is still refused,
  and so is `&mut COUNT`, which Rust 2024 denies too (`static_mut_refs`).

What carries it is ADR 0270's: `analysis::plain_locals::holds_a_value`
says a `static mut` of the crate's or a library's is its module's state of
a value, `module_statics` lists a module's, `plain_statics` makes each its
variable as it makes a thread-local's, `outside_writes` refuses another
module's write, `refuse_boxed_export` an exported box, and the library's
`variables` say which are variables.

Alternatives:

- **Keep the `{ value }`**: one form for a JS importer, which reads the
  box, a wrong answer, and another for a thread-local's state.
- **A setter rust-js writes**, `export function setLines(v)`: an export
  the Rust has none of, ADR 0270's reason; the program's own `pub fn` is
  what JS writes.
- **Keep the box only where another module writes it**: two forms of one
  construct, chosen by what some other module does.

`test/corpus/static_mut.rs` set `log::LINES = 10` from its crate's root;
it calls `log::set_lines(10)` now, and reads `log::LINES` before and after
its module writes it, against native Rust. `static_mut_fns.rs`'s `HANDLER`
is `export let HANDLER = { run: double, name: "double" }`, set whole by its
module. `static_mut_pointers.rs`, against native Rust, gives pointers on,
compares them, writes through them and reborrows them.

What holds it: `test/lowering-module-variables.test.ts`, "a pub static
mut its module alone writes is its export let", read by a JS module and
another of the crate's; "a private static mut is its module's let"; each
write from another module refused; a library's read by its consumer and
its write refused; and "a static mut a pointer to is given on is its box,
refused where it's exported". The mutations `static-mut-*`,
`static-pointer-unrefused` and `library-static-mut-unread`. Of rustc's 46
run-pass tests in scope that use a `static mut` or a raw pointer to a
place, none changed: 35 pass, and 11 still stop where they did.
