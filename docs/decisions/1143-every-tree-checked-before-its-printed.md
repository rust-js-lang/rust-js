# 1143. Every tree is checked before it's printed: its reads resolve, its names are declared once, its jumps land

Status: Accepted (2026-11-14, #1681's change 3, M1; the plan the lead
approved: on in every build the tests and CI run, by a flag the harness
sets, not by `debug_assertions` alone; in a user's release build only once
its cost is measured, under 2%. Amended the same day: measured, each
module's check is on in every build, the user's too).

Case ([0262](0262-when-rust-and-js-disagree.md)): none; this is a check of
rust-js's own trees, not a difference. What it refuses is rust-js's mistake.

## Context

Review #1671's change 3 makes a tree's names facts of the tree, not text:
M1 a validator, M2 unique names, M3 `Local(VarId)` and `Global(name)`, M4
statement facts, M5 node-keyed span forms. Before M1 nothing said what a
printed tree must be beyond `unprintable()`'s leftovers and tidy's
contracts, run in a debug build only: a read of a name nothing declares
was printed, a `ReferenceError` where it ran, and a global read where a
local of its name is in scope read the local, a wrong answer, with nothing
to catch either but a test that happened to run that line.

Tidy's two contracts that every body keeps, "each jump lands" and "each
name declared once in a list" ([0659](0659-tidy-is-a-list-of-stages-with-contracts.md)),
were each a walker of their own: the first pushed no loop for a `do ..
while`, and walked no closure; the second saw `let` and `const` alone, not
a destructuring's names nor a function's.

## Decision

**`js/validate.rs` is the one owner of what a tree must be**, each
module's tree checked in every build (the amendment below). It keeps no
list of names:

- each read resolves: to a declaration in scope, by JS's scopes, each
  block's declarations hoisted, a function's parameters and its own name,
  a `for`'s and a `catch`'s, an import's and a package's, the module's
  items; to the runtime's helper, by `runtime::exports`, the catalog
  `Expr::helper` asserts against; or to a global, a read `Expr::global`
  made (`Known::Global`);
- no read `Expr::global` made is of a name a declaration in scope takes,
  which JS would read instead;
- a block declares a name once, and a function's body doesn't declare a
  parameter's again; each case of a `switch` is a block of its own, as
  the printer braces each that declares (`case_body`);
- each `break` lands on a loop, a `switch` or the label it names, and each
  `continue` on a loop, within its function: none crosses a closure.

`validate::module` checks a module's tree after preparation, its reads
too; `validate::body` a body alone, but its reads, which only its module
resolves: tidy's two contracts are its answers, `JUMPS_LAND` its
`Unlanded` problems and `DECLARED_ONCE` its `Twice`, run after each stage
with tidy's others where `stages_checked()`: in a debug build, and where
`RUST_JS_VALIDATE` is set, as `.env.test` sets it of every process `bun
test` starts, the nightly's suites run with the release compiler
included, no longer `debug_assertions` alone.

**A global's read is made one way, `Expr::global(name)`**, which marks it
`Known::Global`, a form ([1127](1127-a-fact-has-a-role-and-the-js-is-right-without-one-that-permits.md)), never forgotten
by a variant; `js_global` is it and `BuiltIn`, which says no call changes
the global, `Math`, and is no binding's `document`'s. Lowering's reads of
JS's globals, `Array`, `Map`, `Error`, `console`, and the bindings' root
of a path, `document` of `document.title` (`rooted`), are made so; an
import's name, React's `useState`, is a plain read its import declares. A
pass that puts a global's read in a variable's place carries its mark:
`shadows`, of `const performance$1 = performance;`.

**A tree that breaks one is refused**, naming the module, the problem and
what broke it: "as it was lowered", where the module as lowered breaks it
too, else the pass of preparation without which it isn't broken
(`prepare::breaking_pass`, preparation run again on the lowered module
with each pass left out in turn, only of a module refused):

    rust-js made src/app.js wrong by preparation's `shadows`: `performance`
    is read, and nothing declares it nor made it a global: a bug of rust-js's

## Consequences

- Over the corpus, both builds, and every test file that compiles a
  program using React, the bindings or `js`, nothing is refused. Making it
  found two of rust-js's own mistakes, neither yet a wrong answer: the
  `shadows` pass dropped a global's mark where it inlined an alias of it;
  and the bindings' root made an import's name a global's.
- `--without-known Global` forgets every mark, so every global's read is
  refused: `test/validate.test.ts` holds the check on, its refusal's words
  and `.env.test`'s flag.
- Mutations: `js/validate.ts` (a global, a helper, a shadowed global, an
  arrow's parameters, a jump across a closure, a `switch`'s `break`, a
  `switch`'s cases, a destructuring's names), `js/make.ts`'s
  `global-unmarked`, `lower.ts`'s `import-read-made-global` and
  `global-binding-read-unmarked`, `prepare/shadows.ts`'s
  `shadow-of-a-global-unmarked`, `output.ts`'s `tree-unvalidated` and
  `prepare.ts`'s `breaking-pass-of-every-pass`.
- A user's release build checks each module's tree, the amendment below.
- M3 makes `Global(name)` a kind of its own, where the mark is a fact a
  pass may drop today; the check then refuses a local spelled as a global.

## Amendment: each module's tree is checked in every build (2026-11-14)

Measured with the release compiler on the Mac, the check's own work timed
in the compile, the copy of the lowered module that names a pass, the
module's check and tidy's stage contracts apart:

| Workload | Load (`uptime`, 1 min) | Compile | Module check (copy + check) | Tidy's stage contracts |
|---|---|---|---|---|
| The corpus's 120 largest programs, 3 rounds | under 4 | 36.92 s | 0.53% with the contracts | (in the 0.53%) |
| std's bodies (`std/lib.rs --std-bodies`), 3 rounds | 4.9 to 5.3, none of it compiling | 0.433 s | 1.9 ms, 0.44% | 8.5 ms of 7,822 checks, about 2% |

So each module's check, at about 0.5%, is under the 2% the plan set, and
is on in every build: a user's tree that breaks a rule is refused, not
printed. Tidy's stage contracts, a check of each body between stages,
about 2% of std's compile, stay where `stages_checked()`; a module's
check is of the tree it prints, its jumps' and declarations' too.
`RUST_JS_VALIDATE` is that flag alone.

## Amendment: its names are the scope resolver's (2026-11-14)

What each read resolves to, and which names a scope declares twice, are
`js::scopes`'s answers, the one owner of JS's scoping, which reclaim asks
too (ADR [1145](1145-one-owner-of-js-scopes.md)); `js::validate` keeps the
check of jumps alone, which isn't scoping.
