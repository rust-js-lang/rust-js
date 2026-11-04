# 0317. A `OnceCell` is a `{ value }` of an `Option`, set once

Status: Accepted. Extends [0025](0025-vec-loops-refcell-mut.md) and
[0051](0051-generic-options.md); counted by
[0314](0314-std-data-structures.md). Amended by
[0847](0847-a-oncecell-field-is-absent-until-set.md): in a field it's the
property, absent until it's set; a `set` is an assignment where the cell
is known empty, else a test, and `$onceSet` is gone. Amended below
(rd573): callbacks' handle on what owns them, an `Rc<OnceCell<Weak<T>>>`
set once after the call they're given to, is that `Rc`'s `let`; and
(rd553) one kept in a public field stays counted, its refusal naming why;
(rd621) value cells an executor's callbacks read are the call's `const`,
and where nothing the call runs before it returns may call the
callbacks, each `unwrap` of the upgrade is the `let` itself.
Amended (#577): a cell of the call's `Rc` itself, not its `Weak`, read by
`cell.get()`, is that `Rc`'s `let` too, whoever has its readers.

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
- [Below](#amendment-a-cell-of-the-calls-rc-itself-is-its-let-577): A cell of the call's `Rc` itself is its `let` (#577)
- [Below](#amendment-a-tuple-of-a-value-cells-clones-641): A tuple of a value cell's clones (#641)

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


## Amendment: a callback nothing calls before its call returns reads the value (rd621 A)

### Context

React's Fizz server's entries, ReactDOMFizzServerNode's and the others,
make `onAllReady` and its fellows before the request they flow, as
above, and each reads it by `cell.get().and_then(Weak::upgrade)
.unwrap()`. The cell is the request's `let`, but each callback's
`unwrap` stayed, `const request$1 = $unwrap(request);`, six of them,
where upstream reads `request`. A callback called before the cell is set
reads `None` there, and its `unwrap` panics as Rust's does; read as the
variable, it would throw another error, so the `unwrap` could go only
where no callback can run before the set.

### Decision

A parameter what a function returns alone owns is quiet where nothing
the function may run before it returns may call it
(`OwnedByReturn::quiet`): its value is kept in fields of the crate's
struct, which `owns` now reports, through the functions it's given on to,
and no body naming one of those fields, all that may call what it holds
(the field is private, read only to call through it), is among what the
function may run, nor a function of the crate's whose body rust-js can't
read. What a body may run is one rule, `analysis/may_run.rs`, which a
thread-local's initializer asked already (ADR 1006, amended): the bodies
it calls, std's calls by the bounds they're called under
(`cell_borrows::std_runs`), what a library's or a binding's call is given,
each impl of a trait a call through it may be of, a fn pointer's targets
(ADR 0907), each thread-local's initializer it reads, and the
destructors its drops run. Any it can't follow gives no answer, so
quiet is never assumed: a `dyn Fn`'s call of what isn't its parameter's
given, a type parameter's call of what it's given, a fn pointer that may
be anything, a body whose MIR it can't read, or a drop of a `dyn` whose
drop may run a destructor (`drops::drop_runs`, the one rule of what a
drop runs). One a library's function keeps, whose fields the crate can't
name, would be none, but none is owned so: a library's function can't
return an `Rc` of the crate's struct. A library's contract lists its quiet parameters,
`Calling::quiet_by_return`, beside `owned_by_return`, of the same owner
(library ABI 14); a stale manifest is refused by its compiler identity
and ABI, so it can't claim one.

An owner handle whose call is quiet in every parameter its closures are
given as reads its `Rc` as the variable at each `unwrap` of the upgrade
too: the `and_then` and the `unwrap` are no JS, and the local the
`unwrap` gives is the variable, `startFlowing(request, "all")`. A read
that tests the upgrade, `match` or `if let`, reads the variable as
before.

The drop rule of `may_run` was a type's walk of its arguments for the
crate's destructors: a destructor of a field of the crate's type, or one
a `dyn` hides, wasn't found. Now a drop that runs none runs nothing; one
that does, or holds a `dyn Fn`, runs each destructor of the crate's
through the crate's types' fields and what std's and a library's hold,
what a library's destructor holds of a closure, and what each closure the
crate makes a `dyn Fn` of only to hand it to JS holds
(`closures::handed_fn_dyns`): `drop_runs` leaves those out of what a
`dyn Fn`'s drop runs, and any other function made one whose drop runs
code makes every `dyn Fn`'s drop run, which rust-js can't follow. Walking
every function made a `dyn Fn` instead refused React's
`RECONCILE_CHILD_FIBERS`, a `pub` thread-local, whose generic closures'
`dyn`s it couldn't read. A
thread-local's initializer asks the same (ADR 1006, amended).

### Why

ADR 0262's first case, no difference: a closure kept in such a field is
called only by a body naming it, as owned_by_return proves, and none of
those runs before the call returns, nor does the maker run anything
between the call and the set (`set_next`), so each callback runs only
once the cell is set, and its `unwrap` gives the value the variable
holds. Neither Rust's panic nor JS's `undefined` is reached.

A handle is direct where every closure's parameter is quiet, not each
closure alone: React's makers give all of theirs to one quiet call, and
a call that runs one of them before it returns, `complete_shell` above,
calls a `dyn Fn`, which may run anything, the others too. A guard with a
destructor around a direct read needs no `try`: nothing between can
throw (`owner_handle_guarded`).

### Alternatives

- The callee's `Rc` untouched until returned (run 43's design): Fizz's
  `create_request` lends `&request` to its helpers, so nothing changed.
- A second reachability for this question, std's MIR walked: hashbrown's
  rehash calls a `&dyn Fn` and `LocalKey::with` a fn pointer, so Fizz
  would give up; and two answers to what a body runs is two authorities.

### What holds it

- `test/corpus/owner_handle_quiet.rs`, against native Rust: a request
  that lends itself to helpers, gives std a closure, hashes a task of its
  own and reads a thread-local, each `unwrap` the request; one that calls
  a `dyn Fn` it's given, one that drops a struct whose field's destructor
  calls a `dyn Fn`, and
  one that calls a `dyn` of the crate's trait, an impl of which may run a
  derived `PartialEq`, whose body rust-js doesn't read, keep theirs.
  `owner_handle.rs`'s, whose request may complete before it returns, keep
  theirs; `owner_handle_guarded.rs`'s direct read needs no `try`.
- `test/corpus/owner_handle_named.rs`: a request whose helper names the
  field one callback is kept in, without calling it, is quiet in the
  other only, so the handle keeps both `unwrap`s.
- `test/crates.test.ts`, "a library's request that owns its callbacks
  alone ..": `create_request`, whose `complete_shell` calls a `dyn Fn`, is
  quiet in none, and its consumer's `unwrap` stays; `create_quiet_request`
  is quiet in both, and its consumer's reads are the request.
- Mutations `quiet-field-named`, `quiet-unfollowed` and
  `quiet-unread-body` (`owned_by_return.rs`), `quiet-handle-any` and
  `quiet-library-assumed` (`self_handles.rs`), `quiet-unwrap-lowered`
  (`self_handles/owners.rs`), and `may-run-field-destructor`
  (`may_run.rs`). A drop of a `dyn Fn` runs what any function the crate
  makes one of holds, but no owner handle can show it: a crate whose `dyn
  Fn`s hold what drops counts each `Rc<dyn Fn>`, so it makes none.

## Amendment: a cell of the call's `Rc` itself is its `let` (#577)

React's port, rd718 (#577, 2026-11-04). Case
([0262](0262-when-rust-and-js-disagree.md)): A, no observable difference,
under the proof below; where it doesn't hold, the cell is a cell, as
written.

### Context

ReactDOMRootFB's legacy roots wrap the user's callback in a function that
reads the root the next statement makes, which upstream closes over:

```js
if (typeof callback === 'function') {
  const originalCallback = callback;
  callback = function () {
    const instance = getPublicRootInstance(root);
    originalCallback.call(instance);
  };
}
const root = createHydrationContainer(initialChildren, callback, ..);
```

The port's Rust holds the root in a cell the wrapper shares, set once the
root is made, of the root's `Rc` itself: the root owns nothing of the
wrapper's that a `Weak` would free.

```rust
let created: Rc<OnceCell<Rc<FiberRoot>>> = Rc::default();
if let Kind::Function(original_callback) = js::classify(callback) {
    let created = Rc::clone(&created);
    callback = Some(js::unknown_of(move || {
        let instance = ReactFiberReconciler.get_public_root_instance(created.get().unwrap());
        function::call(original_callback, Some(&instance), ());
    }));
}
let root = ReactFiberReconciler.create_hydration_container(initial_children, callback, ..);
let _ = created.set(Rc::clone(&root));
```

ADR 0287 made the cell a `let`, but a `let` of its own, set by a test,
`let created; .. $unwrap(created) .. if (created === undefined) { created
= root; }`, where upstream has one variable.

### Decision

An owner handle's cell may hold the `Rc` the call returns, a clone of it,
as it may its `Weak` (`self_handles/owners.rs`, `Held::Strong`): an
`Rc<OnceCell<Rc<D>>>` of a `D` a `Some` of which isn't boxed, never
looking like `None` (ADR 0051's `may_look_like_none`), made empty by `Rc::new(OnceCell::new())` or
`Rc::default()`, cloned only into closures, each of which reads it only by
`cell.get()`, set once, `let _ = cell.set(Rc::clone(&root))`, of the
variable the call's value is given to, a variable never changed after,
right after the call, with nothing between, never reached again, and
nothing counting `D`'s `Rc`s. Then the cell is the `Rc`'s `let`, as the
`Weak`'s is: `let root;` where the cell is made, `root = call(..)`, each
`get()` the variable, and the cell's making, clones, setting and drops no
JS. Each `unwrap` of what `get()` gives stays, `$unwrap(root)`, as Rust's
panics before the set:

```js
let root;
if (typeof callback === "function") {
  const originalCallback = callback;
  callback = () => {
    const instance = getPublicRootInstance($unwrap(root));
    originalCallback.call(instance);
  };
}
root = createHydrationContainer(initialChildren, callback, ..);
```

`rust-js` asks nothing of where the closures go or who calls them: unlike
a `Weak`, which gives the `Rc` only while what owns the closures keeps it,
a clone, once it's set, is there as long as the cell is, so no call need
own them (`owned_by_return`). Not asked, the call isn't asked what it may
call before it returns either (`quiet`), so the `unwrap`s stay. Otherwise,
the cell is a cell, as written, ADR 0287's `let` where it's one.

### Why

ADR 0262's first case, no difference. The `let` is `undefined` until the
call returns, as the cell is `None` until it's set; the set is next after
the call, with nothing between that could run a closure, so from then on
both give the root, the variable's value the cell's clone, the same
object where nothing counts its `Rc`. A closure run before, by the call or
the maker, reads `None` and `undefined`, and its `unwrap` panics as Rust's
does, with Rust's message: no TDZ is reached, the `let` being made with
the cell. A second set, which Rust refuses, `Err`, can't happen: there's
one, never reached again, and the closures only read.

- Of `D` that may be `undefined`, an `Option`, `()` or a struct of no
  fields, the cell's `Some` boxes it (`$some`), so the variable, which holds what the call gives,
  isn't what `get()` gives: kept as a cell.
- A set only on one path, or of another `Rc`, or a read between, would
  give `None` where the variable holds the root: kept as cells.
- A counted `Rc`'s clone is a count, which `Rc::strong_count` shows: kept
  as a cell.

### Alternatives

- The closure reads upstream's `const root` itself, its `unwrap` gone, as
  the porter asked: a call before the set would throw a `ReferenceError`
  (TDZ) where Rust panics with `unwrap`'s message, case B of ADR 0262,
  never decided alone. Without the `unwrap` only where nothing may call
  the closures before the set (rd621 A's `quiet`): the reconciler keeps
  the callback in an update queue whose commit `may_run` finds behind the
  call's scheduling, so the proof isn't there.
- The wrapper a `function () {..}`, as upstream's: an arrow is rust-js's
  one form of a closure, and nothing reads its `this`; the reconciler's
  `callback.call(instance)` can't tell them apart (the coordinator's
  decision for the user, #577).
- A JS-level rewrite of `let x; .. const r = call(); if (x === undefined)
  x = r;`: it would reconstruct a cell's facts from the JS shape, where
  the owner handle already says them.

### What holds it

- `test/corpus/owner_handle_strong.rs`, against native Rust: ReactDOMRootFB's
  shape, a wrapper the root's flush calls, no callback, one the call
  calls before it returns, whose `unwrap` panics, and one its maker calls
  before the root is made, `None`, each the root's `let`.
- `test/corpus/owner_handle_strong_cells.rs`, each a cell, as written: a
  read between the call and the set, a set on one path, a set each turn of
  a loop, a set of another root, and an `Rc` of an `Option` and of a
  struct of no fields;
  `test/corpus/owner_handle_strong_counted.rs`, a counted root.
- Mutations `owner-handle-strong-boxed` (`self_handles.rs`),
  `owner-handle-strong-traced` and `owner-handle-strong-owned`
  (`self_handles/owners.rs`).

## Amendment: a tuple of a value cell's clones (#641)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. A value cell's
clones are no JS, but a tuple that held them, `let (resolved, rejected) =
(Rc::clone(&wrapper), Rc::clone(&wrapper))`, was written, `const _10 =
[_11, _14$1];`, reading temporaries nothing declares. Tidy's later
stages, and preparation's `unread constants`, took it away, so no build's
JS was wrong; but stopped after tidy's first stage, the closure threw a
`ReferenceError` (found by ADR 1086's least variant). Flight's own,
`(Rc::clone(self), Rc::clone(&wrapper))`, holds a clone of what isn't the
cell beside, which a later field's read needs, `[chunk, _14$1]`.

Now `value_cells::made_in` says each tuple's: one of the cell's clones
alone is no JS with them; one of a clone beside what isn't writes
`undefined` in the clone's place, `[count, undefined]`, as each clone it
holds is a local of no value (`SelfHandles::unwritten`, read by
`LocalFacts::unwritten` where `mir_aggregate` writes a tuple). Its field
is moved out only into what's no JS, so nothing reads the `undefined`.
The finished JS is unchanged.

Held by corpus `value_cell_set_after_its_call` (`wrap`, and `mixed`,
Flight's shape), in `test/tidy-stages.test.ts`'s few cases, whose
`leastDiffers` list is empty now; mutations
`value-cell-tuple-of-clones-kept`, `value-cell-tuple-field-read`,
`value-cell-tuple-beside-dropped` (`self_handles/value_cells.ts`) and
`tuple-field-of-no-value-read` (`mir/rvalues.ts`).
