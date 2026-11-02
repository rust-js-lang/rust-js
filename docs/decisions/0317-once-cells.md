# 0317. A `OnceCell` is a `{ value }` of an `Option`, set once

Status: Accepted. Extends [0025](0025-vec-loops-refcell-mut.md) and
[0051](0051-generic-options.md); counted by
[0314](0314-std-data-structures.md). Amended by
[0847](0847-a-oncecell-field-is-absent-until-set.md): in a field it's the
property, absent until it's set; a `set` is an assignment where the cell
is known empty, else a test, and `$onceSet` is gone. Amended below
(rd573): callbacks' handle on what owns them, an `Rc<OnceCell<Weak<T>>>`
set once after the call they're given to, is that `Rc`'s `let`; and
(rd553) one kept in a public field stays counted, its refusal naming why.

Case: A ([0262](0262-when-rust-and-js-disagree.md)), but for `wait()`
and a `OnceLock`'s reentrant init, D: refused.

## Context

ADR 0314's count had `OnceCell` and `OnceLock` at none of their methods:
even `OnceCell::new()` and a `static` `OnceLock` were refused.

## Decision

**A `OnceCell` or `OnceLock` is a `{ value }` holding an `Option`, as a
`Cell<Option<T>>` is, and its methods are std's, by a helper each where
setting it must check it's unset.**

| Rust | JS |
|---|---|
| `OnceCell::new()`, `OnceLock::new()` | `{}` |
| `c.get()`, `c.into_inner()` | `c.value` |
| `c.set(x)` | `$onceSet(c, x)`, `Err(x)` if it's set |
| `c.get_or_init(f)` | `$getOrInit(c, f)`, which panics as std's does if `f` sets it; a `OnceLock`'s `$getOrInitLock(c, f)` |
| `c.take()` | `$cellReplace(c)` |
| `c.get_mut()` | `c.value` of an object; of a number or text, `c` itself, the `{ value }` a `&mut` to one is (ADR 0074) |
| `{:?}`, `clone()`, `==`, `default()` | `OnceCell(1)` or `OnceCell(<uninit>)`, and its `Option`'s |

- What it holds is `$some(x)`, boxed where it looks like `None` (ADR
  0051), so a `OnceCell<()>` that's set is set.
- A `static` `OnceLock`, which `new()` only ever makes empty, is its
  module's one `{}`, as a `static` is one value (ADR 0096).
- `OnceCell` and `OnceLock` have no diagnostic items: recognition knows
  them by their crate and name, `core`'s `OnceCell` and `std`'s `OnceLock`.
- A `OnceLock`'s `f` that sets it deadlocks in Rust, which JS can't do:
  `$getOrInitLock` throws `rust-js does not support ..` there, not
  `OnceCell`'s panic.
- `wait()` stays refused: on JS's one thread, nothing else can set it, so
  an unset one would wait forever.
- `get_mut()` of a value that looks like `None`, `()` or an `Option`, is
  refused: its box isn't the `{ value }` a `&mut` to one is.

## Why

- **It's the same program**: `get_or_init`'s `f` runs once, a second `set`
  is `Err`, and an init that sets the cell itself panics, as std's do.
- **It's tested**: a corpus case runs each against native Rust, of a
  number, text, an object, `()` and an `Option`, a `static` and `{:#?}`, and
  one the reentrant panic; mutations overwrite on a second `set`, unbox,
  run `f` every time, drop the panic, copy a `get_mut` number, keep what
  `take` took, and refuse a `OnceLock`, a `static` one and a `()` one.
- `docs/std-coverage.txt`: `OnceCell` 7 of 7, `OnceLock` 7 of 8.

## Amended by

- [0847](0847-a-oncecell-field-is-absent-until-set.md): A `OnceCell` in a field is the property, absent until it's set
- [Below](#amendment-callbacks-handle-on-what-owns-them-is-its-let): Callbacks' handle on what owns them is its `let`
- [Below](#amendment-a-callback-a-public-field-holds-isnt-the-returns-alone-rd553): A callback a public field holds isn't the return's alone (rd553)
- [Below](#amendment-callbacks-made-in-an-executor-read-its-calls-value-rd621): Callbacks made in an executor read its call's value (rd621)

## Amendment: callbacks' handle on what owns them is its `let`

The user's decision (rd573, "rust-js form + Weak", 2026-11-02). Case
([0262](0262-when-rust-and-js-disagree.md)): A, no observable difference,
under the proof below; where it doesn't hold, the cell and its `Rc`s are
counted, as written.

### Context

React's Fizz server's entries make callbacks before the request they
need, given to the call that makes it, which they then flow:

```js
let request;
const onAllReady = () => { .. startFlowing(request, writable); };
request = createRequest(children, resumableState, .., onAllReady, ..);
```

The Rust a person writes for that is a cell the callbacks share, set
once the request is made; of its `Weak`, as the Rust book teaches, so the
request, which owns the callbacks, and the callbacks, which hold the
request, make no cycle of `Rc`s that never frees:

```rust
let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
let on_all_ready: Rc<dyn Fn()> = {
    let cell = Rc::clone(&request_cell);
    Rc::new(move || {
        let request = cell.get().and_then(Weak::upgrade).unwrap();
        start_flowing(&request, writable);
    })
};
let request = create_request(.., Some(on_all_ready), ..);
let _ = request_cell.set(Rc::downgrade(&request));
```

Its `upgrade` asks whether the request is alive, so ADR 0843 counted
every `Rc<Request>`: the cell a `{ value }`, each `Rc` `{ value, strong,
weak }`, a `try`/`finally` of each, and in React's port, react-dom, a
refusal: its callbacks' counted cell made its `dyn Fn`s drop what they
hold, where react-dom-bindings', which hold them, drop nothing (ADR
0626).

### Decision

**A local `Rc<OnceCell<Weak<T>>>`, cloned only for closures that read it
by `cell.get().and_then(Weak::upgrade)`, and set once, right after the
call those closures are given to returns the `Rc<T>`, of that `Rc`, is
that `Rc`'s `let`, which the closures close over, where what the call
returns alone owns them and nothing counts the `Rc`:**

```js
let request;
const onAllReady = () => {
  const request$1 = $unwrap(request);
  startFlowing(request$1, writable);
};
request = createRequest(.., onAllReady, ..);
```

- **Its making is the `let`**, `let request;`, of the `Rc`'s variable;
  its clones, its setting and its drops are no JS, and the call's value
  is assigned to it, `request = createRequest(..)`.
- **Each read is the variable**, `undefined` until it's set, which is
  `None`: `cell.get().and_then(Weak::upgrade)` is `request`, so a
  callback called before it's set reads `None`, and its `unwrap` panics
  with std's message, `$unwrap(request)`, as Rust's does.
- **The upgrade asks no count** (`counted_rcs`, `asks_no_count`), as a
  closure's handle on itself's doesn't (ADR 0843, amended).

What proves it, one owner, `analysis/self_handles/owners.rs`, read of
each body's MIR as borrowck reads it:

- **The cell**: a variable `Rc::new(OnceCell::new())` of an `Rc` of a
  `OnceCell` of a `Weak`, used only to clone it, each clone captured by
  one closure, to set it once, `let _ = cell.set(Rc::downgrade(&request))`,
  its `Result` dropped, and to drop it. Set on a path that comes round to
  it again, or more than once, or its `Result` read, it's a cell.
- **The closures**: each made an `Rc` and given, as it is, coerced or in
  a `Some`, moved, to one call, whose destination is the variable the
  cell is set of, never changed after through a `&mut` or a pointer; each
  reads its clone only by the chain, to a local of its own; a closure
  reads one handle at most.
- **Nothing between**: from the call's return to the setting, only the
  steps of setting it, so no closure reads the cell set where JS's
  variable is assigned already.
- **What the call returns alone owns the closures**
  (`analysis/owned_by_return.rs`, `owns`): each parameter they're given
  to is moved, as it is, through `Some`, `unwrap`, `unwrap_or_else` and
  the like, or to another function whose return owns it, into a field of
  a struct an `Rc::new` is given, and that `Rc`, only moved, borrowed
  shared and dropped, is what each path returns; the field is an `Rc`,
  private to its crate, of a struct that isn't `Clone`, which no body
  reads but by a shared borrow only dereferenced, the call through it,
  or drops. So a closure lives only while the struct's value does: while
  its `Rc`'s strong count is one at least, or while it's dropped, which
  runs code only where its drop runs a destructor, which counts it
  (ADR 0320). A library's function's contract says the same of it,
  `owned_by_return`, its parameters by index (`crate::library::Calling`):
  React's `createRequest`'s callbacks are its request's.
- **Nothing counts the `Rc`**: once counting settles, a handle whose `Rc`
  is counted anyway, its counts read or its drop running a destructor, is
  dropped from the facts, and lowered as written (`uncounted`).

So while a closure runs, the `Rc` the call returned is alive, and once
the cell is set, the upgrade gives it; before, `None`. Both are what JS's
variable holds.

### Why

- **Upstream's JS**: `let request; .. request = createRequest(..)`, the
  callbacks closing over it, and React's request a plain object.
- **The idiomatic Rust**, with no cycle: the `Weak` the Rust book teaches,
  where an `Rc<OnceCell<Rc<Request>>>` would never free, and a `Box::leak`
  or a `&'static` is what the port rules forbid.
- **It's the same program**: a read before the setting is `None`, a
  second `set` keeps the first, and an upgrade after the request is gone
  gives `None`; each is kept, by the cases the proof refuses.

### Alternatives

- **Upgrade asks no count wherever a `Weak` is uncounted**: a callback
  kept apart, by a clone, a list or a field cloned out, runs after its
  request is dropped and reads it, where Rust's gives `None`. Rejected
  (case C).
- **Prove the `unwrap` can't fail**, so it's `request` alone: that the
  call never calls a closure before it returns asks what every function
  it calls may call, through `dyn Fn`s too. Deferred; the `$unwrap` keeps
  std's panic.
- **`const request = createRequest(..)` after the closures**, as
  upstream's renderToReadableStream writes: a callback the call runs
  before it returns would throw JS's `ReferenceError` where Rust reads
  `None`, so only where that's proven, deferred with the `unwrap`.

### Consequences

- Where any of a crate's `dyn Fn`s may hold what has a destructor, a
  counted `Rc` say, each `dyn Fn`'s drop may run code (ADR 0626), so a
  request holding its callbacks runs code as it's dropped, and is counted:
  the handle is lowered as written.
- The library contract's items carry `owned_by_return`; a manifest
  without it, an older library's, says no return owns anything, and its
  consumer counts, or is refused, as before.

### What holds it

- `test/corpus/owner_handle.rs`: React's entries' shape, a request made
  through a function that gives its `Option`s' payloads, `unwrap_or_else`,
  to the struct, callbacks called after the setting, one called before,
  `None`, and one whose `unwrap` panics, with native Rust;
  `owner_handle_guarded.rs`: guards dropped in the maker and the
  callbacks, a panic in the call, and handles made in a loop.
- Counted, as written, each with native Rust: `owner_handle_kept_apart`
  (a clone kept apart, called after the drop), `owner_handle_stashed`
  (kept in a list by a function the call gives it on to),
  `owner_handle_field_cloned` (a method clones the field),
  `owner_handle_cloned_request` (the request is `Clone`),
  `owner_handle_returns_another` (the call returns another request),
  `owner_handle_dropped_calls` (the request's destructor calls it, its
  upgrade `None`), `owner_handle_set_twice`, `owner_handle_set_result_read`,
  `owner_handle_set_in_loop` (set each turn, the first kept),
  `owner_handle_called_between` (called before the setting),
  `owner_handle_closure_sets` (a callback sets the cell itself) and
  `owner_handle_swapped` (the request's variable swapped through a `&mut`).
- `test/crates.test.ts`, "a library's request that owns its callbacks
  alone is its consumer's callbacks' let": the contract's
  `owned_by_return`, none of a public field, and the consumer's `let`.
- React's port (react-in-rust d036531): react-server's contract says
  `createRequest`'s and `createPrerenderRequest`'s returns own their
  `onAllReady` and `onShellReady`, `resumeRequest`'s and
  `resumeAndPrerenderRequest`'s theirs; a react-dom entry written as
  above is `let request; .. request = ReactFizzServer.createRequest(..)`,
  where it was refused. No JS of the port's changed.
- Mutations in `scripts/mutations/lower/analysis/self_handles.ts`,
  `self_handles/owners.ts`, `owned_by_return.ts`,
  `lower/mir/structure.ts` (`owner-handle-undeclared`),
  `lower/library.ts` (`owned-by-return-of-a-library-unread`,
  `owned-by-return-unexported`) and `lower/recognition/wrappers.ts`
  (`unwrap-or-else-unrecognized`, `once-get-unrecognized`).

## Amendment: a callback a public field holds isn't the return's alone (rd553)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results change,
for the shape this keeps counted; none for the rest: the refusal's words
only.

### Context

React's Flight server's prerender entries, ported as Fizz's are, make
`onAllReady` before the request it flows, of the cell above:

```rust
let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
let on_all_ready = {
    let cell = Rc::clone(&request_cell);
    move || {
        let request = cell.get().and_then(Weak::upgrade).unwrap();
        // .. a stream that flows it
    }
};
let request = create_prerender_request(model, .., Rc::new(on_all_ready), ..);
let _ = request_cell.set(Rc::downgrade(&request));
```

react-dom builds of Fizz's; each of the five react-server-dom-* packages
was refused, "this crate's `dyn Fn`s may hold a value with a destructor,
.. but `react_server`, a rust-js library whose `dyn Fn`s drop nothing,
holds one of them" (ADR 0626). The two are one shape but for one word:
Flight's `Request` kept `pub on_all_ready: Rc<OnAllReady>`, Fizz's a
private `on_all_ready`. A public field, any crate may read and clone,
so `react_server`'s contract says its `create_prerender_request`'s return
doesn't alone own `on_all_ready` (`kept_field`, as the decision above has
it). The cell was counted, the closure holding it dropped what it held,
and `react_server`, whose `dyn Fn`s drop nothing, would drop nothing of
it: refused, rightly, but with no word of which `dyn Fn`, what it held,
or why that was counted.

### Decision

**The proof is unchanged: a callback kept in a public field isn't what
the call's return alone owns.** Another crate, any of the program's, may
clone it out of the request, `Rc::clone(&request.on_all_ready)`, keep it
past the request and call it, when Rust's upgrade gives `None` and the
request's `let` would still hold it. A library can't see the crates that
will use it, so its contract can't say none does. The crate is refused,
as before.

**The refusal names the `dyn Fn` the crate makes that drops, what it
holds that does, and, where that is a cell made as an owner handle's is,
the call whose return doesn't own the closure, and what the field must
be:**

```
error: rust-js: this crate's `dyn Fn`s may hold a value with a destructor, which their drop runs, but `dep`, a rust-js library whose `dyn Fn`s drop nothing, holds one of them, and would drop nothing of it (ADR 0626)
  --> app.rs:22:50
   |
22 |     let request = create_prerender_request(name, Rc::new(on_all_ready));
   |                                                  ^^^^^^^^^^^^^^^^^^^^^ this `dyn Fn` holds a `std::rc::Rc<std::cell::OnceCell<std::rc::Weak<dep::Request>>>`, whose drop runs code
   |
   = note: its cell is the `let` of the `Rc` the call it's given to returns only where that return alone owns the closure (ADR 0317, amended), and `dep::create_prerender_request`'s doesn't own `on_all_ready`: the field it's moved to must be private to its crate, of a struct that isn't `Clone`, and read only to call through it; a library's contract says so of its function (`owned_by_return`)
```

- `closures::dropping_fn_dyn`, which `fn_dyns_drop` was, gives the first
  coercion of a function that drops to a `dyn Fn` (its span), and what
  it holds that drops (a closure's capture, or the function itself if
  it's a type parameter's), not only whether there is one.
- `owners::owner_made` records each cell made as an owner handle's is
  but given to a call whose return doesn't own its closure
  (`SelfHandles::unowned`): the closure, the cell's type, the call and
  which parameter.
- Where the crate's `dyn Fn`s drop because a library's own may, nothing
  the crate made is to blame, and the refusal is as before.

The port's form is Fizz's: the request keeps its callbacks in fields
private to its crate, `on_all_ready: Rc<OnAllReady>`, which none of the
entries reads. With that, react-server's contract says
`create_prerender_request`'s return owns it, and each entry's cell is
`let request; .. request = createPrerenderRequest(..)`.

### Why

- **Correctness first**: a public field's callback can outlive its
  request in Rust, as the pair below runs, so the `let` would change the
  result.
- **Refuse clearly**: the refusal named neither the closure nor the
  cause, a public field in another crate. Now it names the line, the
  cell and the call, and says what the field must be.

### Alternatives

- **A public field's callback owned, a consumer that clones it refused**:
  the library's contract would bind every crate that reads the field, and
  refuse valid Rust, a clone of a public `Rc`, in crates that make no
  cell. Rejected: a private field is the Rust the proof needs, and the
  Rust Fizz's port already has.
- **The refusal unchanged**: it named nothing that led a port to the
  word `pub`.

### What holds it

- `test/crates.test.ts`, "a library's request whose callback is a public
  field refuses its consumer's owner handle, saying why", of
  `test/crates/refused/owner_handle_public_field` (`refused/` holds pairs
  whose app is refused): the app clones the callback out of the request,
  drops the request and calls it. Natively it prints `no request`, its
  upgrade `None`, where the `let` would give the request. It's refused,
  naming the line, the cell's type and `dep::create_prerender_request`'s
  `on_all_ready`, and `dep`'s contract has no `owned_by_return`. The
  private field's pair, `owner_handle`, still compiles and runs as native
  Rust does.
- Mutations `dropping-dyn-fn-unnamed` (`analysis.rs`),
  `dropping-fn-dyn-holds-itself` (`analysis/closures.rs`) and
  `owner-handle-unowned-unrecorded` (`self_handles/owners.rs`), and
  `fn-dyn-drops-unseen`, `fn-dyn-generic-unseen`,
  `handed-fn-dyn-still-drops`, `library-fn-dyns-always-drop`,
  `library-fn-dyns-never-drop` and `owner-handle-not-owned`, rewritten
  for the new code.

## Amendment: callbacks made in an executor read its call's value (rd621)

### Context

React's Flight client's DEV `then` wraps a chunk in a native promise whose
`resolve` and `reject` callbacks, made in its executor, write
`_debugInfo` on that promise, `wrapperPromise._debugInfo = ..`. The port
holds the promise in a cell, `Rc<Cell<Option<Promise<..>>>>`, set right
after `js::promise::new` returns, which the callbacks read,
`wrapper.get().unwrap()`. The cell was a `let` assigned after the call,
read by `$unwrap(wrapper)`, where upstream closes over the `const`.

### Decision

A value cell is the variable of the value it's set to, and each read of
it, `cell.get().unwrap()`, is that variable, where (`self_handles/
value_cells.rs`):

1. it's an `Rc<Cell<Option<T>>>` of a `T` nothing changes in place, a
   shared reference or a JS object's `Copy` handle, a `Promise`, made by
   `Rc::default()` or `Rc::new(Cell::new(None))`, lent to one closure, the
   executor, which the call is given;
2. it's set once, `cell.set(Some(value))` of the call's value, a variable
   never changed after, next after the call, never reached again;
3. each reader is a closure the executor makes of a clone of it, moved,
   in an `Rc` or a `Some`, into a place a `&mut` of its maker's local
   lends, that capture only stored into, its old value dropped: the
   executor neither calls a reader nor gives it to a call, and the maker
   reads those locals only after the set;
4. each reader reads it only by `get().unwrap()`, whose value is never
   lent mutably, nor tested.

The cell's making, borrow, clones, setting and drops are no JS, and each
read the variable; `const wrapperPromise = new Promise((res, rej) => {
resolve = (value) => { wrapperPromise._debugInfo = ..; res(value); };
.. })`. Otherwise the cell is a cell, as written.

### Why

ADR 0262's first case, no difference: no reader can run before the set
(3, 2), so each read gives the value the variable holds, and neither
Rust's panic of a read before it nor JS's of a `const` not yet made is
reached. The executor may call what it's given, `res`, at once: it's no
reader. A `&mut` the maker was given, rather than its own local, is
refused: its caller could call a reader once the call has panicked, before
any set, where Rust panics and the variable would throw another error.

### What holds it

- `test/corpus/value_cell_set_after_its_call.rs`, against native Rust:
  two readers through a tuple of clones, an executor that calls what it's
  given, and the value's only read after the set, given back.
- `test/corpus/value_cell_read_in_its_executor.rs`, each a cell, as
  written, its `unwrap` panicking as Rust's: a reader the executor calls,
  one it gives to a call that calls it, one stored through its maker's
  caller's `&mut`, called once the call panicked, one the maker calls
  before the set, a `Copy` struct a reader changes its copy of, and a
  variable changed after the set.
- `test/js-builtins.test.ts`, "callbacks made in a promise's executor read
  the promise's const": Flight's shape, run, `_debugInfo` set, of a
  promise the executor settles at once too.
- Mutations `value-cell-called-in-executor`, `value-cell-given-on`,
  `value-cell-callers-place`, `value-cell-read-before-set`,
  `value-cell-changed-in-place` and `value-cell-variable-changed`
  (`self_handles/value_cells.rs`).

