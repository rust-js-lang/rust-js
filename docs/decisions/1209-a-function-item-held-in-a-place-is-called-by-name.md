# 1209. A function item held in a place is called by name

Date: 2026-11-27.

Status: accepted (#2629).

Case: results change ([0262](0262-when-rust-and-js-disagree.md)):
Rust's behavior is preserved.

## Context

A function item held in a local and called through it,

```rust
let t = touch;
t(&made);
```

is built by rustc as a call whose callee is a copy of the place, `_4 =
copy _3; _5 = move _4(..)`, not the function's constant. Its type is still
the function's, `FnDef(touch)`, which is zero-sized: the value says nothing
the type doesn't. rustc's own codegen calls it directly, and its optimized
MIR names the function, since GVN makes every zero-sized copy its
constant. A `#[track_caller]` function called so is given the call's
location.

rust-js asks what a call calls by `func.const_fn_def()`, at about 160
sites in rust-js-mir and rust-js-lower: the call graph, the cell-form and
cell-borrow analyses, a caller's location, effects, drops and the
`Option` and interval proofs. Each read `None` of the copied place, so a
call through a local was any function's call, and gave wrong answers. The
metamorphic net's `closure-fn` rewrite, which turns a closure into a
nested `fn` item and keeps its `let`, found it in the nightly of
2026-11-26 and 2026-11-27:

- A `RefCell` borrowed by the function while a borrow of it was held went
  unchecked. Native panics "RefCell already borrowed"; the JS ran on
  (`refcell_flow_through_std`, `refcell_flow_thread_local`,
  `refcell_flow_thread_local_init`).
- A `#[track_caller]` function called so named its own line, not the
  call's: `let h = here; h()` printed `3` where native prints `7`.

## Decision

The MIR rust-js lowers is the one place this is answered. `Mir::new`
(ADR 0364, "the MIR a body is lowered from, as it's taken"), through which
every body passes, the crate's, a library's optimized one and each
promoted constant, makes each call whose callee is a place of a function
item's type a call of that function by name, `Operand::function_handle`
(`named_callees` in analysis/prepared.rs). That is the form rustc's
optimized MIR gives the same call. Every reader of a call's callee then
asks the callee itself, the 160 sites and any added later, with nothing to
migrate.

It does so only where the local's JS is the function's name
(`held_by_name`): a local of a function's type (`DefKind::Fn` or
`AssocFn`), not a parameter, never lent `&mut`, whose every write is the
function's constant or a copy of another such local. Three kinds keep
their call through the local, as the JS value differs from the name
there:

- a binding's function, whose value is JS's, read where it's held;
- a constructor, which runs no code of its own and may have a JS form of
  its own: `let symbol_for = SymbolFor; symbol_for(key)` stays
  `symbolFor(key)` after `const symbolFor = Symbol.for` (ADR 0964);
- a function a call or an `.await` gives, `js::import!(lint::long_lines)
  .await`, written of its `Poll`'s field, and `js::import_module!(root::
  Root).await`'s `module.default()`, whose value is the dynamically
  imported one, so naming it would import its module statically (ADR
  0304). A call's result is any value of the type, so
  `identity(touch)`'s stays a call through its local too.

A `fn` pointer's call, which may be any function's, and a closure's, a
call of its `Fn` impl, keep their forms too.

The alternative, an owner function `callee(func, body)` that each of the
160 sites asks in place of `const_fn_def()`, with an architecture test
banning `const_fn_def` elsewhere, answers the same question in one place
but leaves every site's reading of `Operand::Constant` as it is, and each
new reader must know to ask it. Normalizing the MIR where it's taken
leaves one form of a direct call for every reader. The architecture test
holds that every body is taken through `Mir::new` (`mir_promoted` read
only in analysis/collected.rs).

## Consequences

- The JS calls the function by name: `let t = touch; t(&made);` is
  `touch(made);`, and the local `t`, which nothing else reads, isn't
  written. A local also used as a value, `.map(t)`, keeps its
  declaration.
- `test/corpus/fn_item_called_through_a_local.rs` holds a
  `#[track_caller]` function's line through a local and a pointer, and a
  cell's borrow checked through a local, against native Rust.
- The mutation `callee-in-a-place-unnamed` removes the normalization;
  `callee-awaited-named` names a local an `.await` writes, which
  `docs/guide/project/modules.md`'s dynamic import catches, and
  `callee-given-by-a-call-named` one a call writes, which
  test/bindings.test.ts' `js::import_module!` catches.
- Two corpus snapshots move to the call by name:
  `closure_giving_back_a_write_error`'s `held()` is `named()`, its local
  gone, and `scope_struct_kept_apart`'s `get(named)` is `Named.get(named)`,
  `const get = Named.get;` kept, as a property's read is. Keeping the
  program's local as the callee while the analyses see the function is
  #2633 (P3).
- A function a dynamic import gives and calls through its local is still
  any function's call to the analyses: a cell it borrows while a borrow
  is held goes unchecked there, as before this decision.
- About a dozen sites read `tcx.optimized_mir` directly, not through
  `Mir::new`: a std body's, which rustc's GVN already normalized, or a
  question answered conservatively of a callee it can't name (whether a
  call may unwind, may box). A reader of the crate's own optimized MIR
  that asks what a call calls should take it through `Mir::new`.
