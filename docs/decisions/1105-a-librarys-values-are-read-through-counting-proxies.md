# 1105. A library's values are read through counting proxies, its program's reads compared on and off

Status: Accepted (#891; the net the user's queue asked for, 2026-11-09).
Extends [1087](1087-each-readability-rewrite-is-checked-on-and-off.md),
whose read oracle ran four hand-written read cases, to the corpus, and
[1104](1104-a-case-rewritten-in-meaning-runs-as-it-did.md), whose
`library` rewrite it runs.

Case ([0262](0262-when-rust-and-js-disagree.md)): none decided here. This
is a net: each wrong answer it finds is its own issue, decided there.

## Context

The bug-cluster audit ([bug-clusters.md](../bug-clusters.md), its fifth
recommendation) counts 88 wrong answers in `classes-js-objects`, the JS
object model: a getter read twice, `this`, prototype keys, classes. Each
was found by hand. Native Rust can't see one: a Rust value has no getter,
so a read made twice, or out of order, prints the same. ADR 1087's read
cases can, as each gives what it calls counting `Proxy`s, but they are four
libraries of 33 calls, written by hand.

ADR 1022 says which values JS may have made, whose properties a getter
may answer: a binding's result, and a library's. A program that uses a
library reads each value the library gives it as Rust reads it, as often
and in the same order, however its readability passes rewrite it.

## Decision

**Each corpus case's items are moved into a library, metamorph's
`library` rewrite (ADR 1104), and its program is run with every value the
library gives it read through a counting `Proxy`**
(`test/object-model.test.ts`, `test/object-model-run.ts`):

- **The membrane.** Each module of the library the program imports is
  loaded first, and the program's import of it is made an import of a
  module of the same names, each value counted: a function's result, an
  exported constant, and what's read of them, an object or a function a
  proxy of it, the same one each time. Each get, set, test of a key,
  listing of keys, call, with what it's given and, of a method, its
  `this`, and `new` is logged in the order it's made. What the library
  does is not: it's given back its own objects, and a read made while it
  runs gives its own value, so `Rc::ptr_eq` of them is as it was made. An
  object the program gave the library is the program's however it comes
  back. What isn't data is left out: an export's own functions, a callee's
  read; a method's read, whose call is what's counted; a symbol's, JS's
  protocol, `Symbol.iterator`'s, run as the library's; an array's indices,
  whose items are counted as they're read; a `Map`'s or a typed array's
  slots, which a proxy hasn't; what a call is given past the parameters its
  function declares, `map`'s index and array, and a last `undefined`; and
  the program's own objects, shown by their kind, whose form a pass may
  choose. A `let` the library assigns again, a `static mut`'s, is read
  live as itself, uncounted. A module that exports a `then` is loaded
  through a module of its own, as `import()` would call a thenable's.
- **The oracle.** The program compiled as built and at its least, tidy
  stopped after its first stage and none of preparation's rewrites, each
  runs as native Rust does, its output, stderr and how it ended; and as
  built reads what the least reads, each line as often. Order alone is
  tallied, not failed: the least is no oracle of order, as it reads a
  value `format_args!` shows before the arguments after it are made,
  where Rust and the JS as built make them all first. Where a run
  differs from native's, it's run again without the proxies: wrong so too,
  it's the metamorphic net's to hold, not this one's.
- **What it says.** Where the logs differ: the first line they differ at,
  the first of tidy's stages after which it reads otherwise, and each of
  preparation's rewrites without which it reads as the least.
- **The known list**, `test/object-model-known.txt`: each case that reads
  otherwise under its issue, a line `#<issue> <case>`; a listed case that
  reads as the least fails until it's taken off, so the list only shrinks.
- **Where it runs.** Every case a night (`OBJECT_MODEL=all`, nightly.yml's
  `object-model`, 4 parts; `bun run nightly object-model`). A push's
  sample, one case in 64 by a hash of its name and each listed one, in
  `check:fast` where its change is the object model's owners
  (`classes/`, `untagged/`, `field_keys.rs`, `bindings.rs`,
  `representation.rs`, `analysis/fn_pointers.rs`, `src/js/stmt_walk.rs`,
  `src/js/expr_walk.rs`), the readability passes, or the net itself.

## Its first run

Of 1,918 cases (2026-11-09): 1,486 ran with the library's values proxied,
240 have no item the rewrite moves, 142 programs and 19 libraries rust-js
refuses, 7 have no native run of the rewrite, and 2 run wrong unproxied,
the metamorphic net's. 16 read the same in another order. 9 read
otherwise, listed under their issues: tidy's `names/never taken` reading a
library's struct's field twice where Rust reads it once
(`refcell_quiet_borrows`'s `inner.types`) and an enum payload of a
library's result (#1065, P0); `values/shared tails` repeating one (#1066,
P0); and, to decide, `written/never taken` leaving out a flattened enum's
`undefined` writes, `loops/unlabeled` leaving out an unread `$someNone`
test, and two representation choices of the least (#1067). A library's
`pub fn then` makes its module a thenable, which any `import()` of it
calls, crashing a host's or a bundler's load of it: found by the harness as
it loaded one (#1068, P1).

## Alternatives

- **Every object the program makes counted, not only the library's.**
  A struct the program builds is its own data, read in place (ADR 1022):
  a rewrite may read it twice, and would fail on every case.
- **The library's reads of what the program gives it.** What JS calls an
  exported function with is read as the program's data, the user's
  decision in ADR 1022; a getter there is no wrong answer of rust-js's.
- **Order as a failure.** The least isn't Rust's order; failing on order
  would fail where the JS as built is right. A Rust-side record of each
  read, from MIR, would be the oracle of order; it's left for a later net.

## Consequences

- `test/object-model.test.ts`, `test/object-model-run.ts` and
  `test/object-model-known.txt`; the nightly's `object-model` job, here
  and on CI; the `check:fast` job; `OBJECT_MODEL` and `OBJECT_MODEL_TALLY`
  in `scripts/shard.ts`.
- [TESTS.md](../../TESTS.md) and
  [finding-wrong-answers.md](../finding-wrong-answers.md) say what it found.

## Amendment: the program is built with `--js-made-all` (2026-11-09)

The first run's two P0s, #1065 and #1066, were no bugs: a struct its
library builds is the program's own data, read in place (ADR 1022's rd221:
`own` takes a library's built structs as the crate's), and ADR 1022 covers
structs only, not a library's enums (the user, #1066). Every struct the
`library` rewrite gives is built by its library, so the net compared reads
the decisions let tidy move.

**The program is compiled with `--js-made-all`**, a test hook beside
`--tidy-until` and `--std-iterators`, which `rust-js build` never passes:
every struct the crate doesn't build itself, a library's too, is one JS may
make (`fn_pointers.rs`, `take_every_outside_struct_for_js_made`), its
fields read where Rust reads them. What the built JS then reads otherwise
than the least is a pass that ignores `Known::Getter`, wrong for a struct JS
did make. Under it, `refcell_quiet_borrows` reads `inner.types` once, as the
least does; the enum rows stay, listed under #1066 as decided; and two new
cases read otherwise: a function a struct JS may make holds, `(shared.getter)(3)`,
called as its method, `shared.value.getter(3)`, given the struct as its
`this`, where the least calls it apart. That's #404's decided form, a field
the program never writes being an object JS made's own method, which JS
calls through it (`may_read_this`), so it's listed as decided under #1070,
closed.
The net's own test holds the hook: a library's `Pair`'s fields read once
under it; mutations `js-made-all-ignored` and `js-made-all-unread`.
