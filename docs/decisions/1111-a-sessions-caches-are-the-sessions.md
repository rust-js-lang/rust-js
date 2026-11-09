# 1111. A session's caches of rustc's ids are the session's

Status: Accepted (m6-react's choice of form B, 2026-11-09). Amends the
memos of #884: `std_path`'s, `compiled_std`'s, `has_compiled_bodies`' and
`compiled_provided_method`'s, and the run's flags of ADR 0932 and ADR 1105.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. What the JS is doesn't
change; what could answer wrongly can't any more.

## Context

The lowering kept what it found of rustc's ids in `thread_local!`s:
`STD_PATHS` (a `DefId`'s std path), `COMPILED` and `COMPILED_TYPES` (std's
compiled bodies and their types, `DefId`s), `PROVIDED` (a provided method's
`DefId`), and two flags of the run, `MEANING` and `EVERY_OUTSIDE_STRUCT`, none
ever emptied. A `DefId` is a crate's number and an index, and a session
numbers its crates as it loads them, so a second session in the process
would be answered with the first's items. None compiles twice today: each
build, Cargo's, the editor's and the dev server's, runs the compiler as its
own process; the playground instantiates its WebAssembly compiler anew for
each compile (`wasm/web/rust/compiler.rs`); the panic runtime's and std's
sysroot's sessions run before the lowering's and lower nothing. So no wrong
answer was found. It held by luck of the hosts, not by design.

## Decision

**What a lowering session finds of rustc's ids, and the flags its run is
given, are its `Caches`, made as the session starts and dropped as it ends**
(`lower::session`). `RustJs::after_expansion` starts it with the run's flags
(`session::start`), and the guard it gives empties the one scoped slot as it
drops: the only way it's emptied. Every read is `session::caches(tcx)`,
checked against the session, by its rustc `Session`'s address: a read
outside a session, or with another session's `tcx`, is a bug, which panics
saying so; a session started inside another too. It's how rustc carries its
own implicit context: one slot, scoped to exactly the session, checked
against the `tcx` it's asked with. Threading a context through every helper
of recognition, the other form, would change several hundred signatures for
the same guarantee.

No `thread_local!` or `static` of the compiler's holds rustc's ids
(`DefId`, `LocalDefId`, `DefIndex`, `CrateNum`, `HirId`, `Symbol`, `Ty`) but
the session's slot.

`jsx_syntax`'s `CRATE_ID` stays a process's: it's a `StableCrateId`, the
hash of the crate's name and its metadata, the same in every session of the
crate, not a session's number.

## Consequences

A cache keyed by a rustc id goes in `Caches`, read through `caches(tcx)`.
`compiled_for_build` and `zips_by_index_compiled` take the `tcx` they read the
session's flag with. The JS is the same, byte for byte: the corpus, the
snapshots, the conformance table, std's bodies and React's port.

Held by `test/architecture.test.ts`'s "no thread-local or static holds
rustc's ids but the lowering session's caches", which names
`lower::session` as their place, and by `lower::session`'s unit tests: a
second session starts with caches of its own, its flags its own; a cache
asked after its session ends, or with another session's `tcx`, panics; a
session inside another panics.
