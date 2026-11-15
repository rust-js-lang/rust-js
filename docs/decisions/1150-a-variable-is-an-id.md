# 1150. A variable is an id; its spelling is preparation's

Status: Accepted (2026-11-14, #1681's change 3, M3, the plan the lead
approved; in progress, a step a commit, each recorded here).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This is about how
rust-js's JS tree names its variables. No output changes: each step is
byte for byte what was printed before.

## Context

A variable of the JS tree was its spelling, a `String`: `ExprKind::Var`,
a `Const`'s and a `Let`'s name, a parameter's, a label's. Two variables
spelled alike were taken for one, and a pass that asked which variable
it had read it off the spelling: tidy's kinship, `b$1` given apart from
`b`, and preparation's reclaiming of `b$1` to `b`. M2 (ADR 1145) made each
compiler-made name unique in its item, the last of them not spelled yet
till preparation; what's left, a tree used again where it's needed twice
and the names tidy and three functions of no lowering make, needs a name
that's not its spelling.

## Decision

A name is `js::Name` (`src/js/name.rs`), the one type a read and a
declaration have, and its identity is its own: an id of its item's names,
of a global, or of a label, with its word, spelled once by preparation.
It's reached in eight steps, each byte-identical:

1. **S1, the accessor.** `Name` takes the place of the `String`s, one
   form for now, its spelling. S1a puts it in the tree with its word
   readable as before, `Deref` to a `String`; S1b takes that away a part
   at a time, each site then asking `Name`'s API.
2. **S2**, compiler-made names are ids (M2's `value#1` gone).
3. **S3**, a global is a form of its own, not a fact a read has.
4. **S4**, the program's locals are ids; tidy's kinship asks the table.
5. **S5**, labels are ids.
6. **S6**, tidy's own names are ids.
7. **S7**, a tree used again is given binders of its own; a module that
   declares an id twice is refused.
8. **S8**, a spelled name is preparation's alone; `Var(String)` is gone.

Then M4, the statement facts, and M5, the node-keyed forms, finish
change 3.

## Consequences

Follow-ups the lead recorded, outside S4, which keeps tidy's answers as
they are: once names are ids, tidy can ask whether two names are one Rust
variable rather than whether they're spelled alike, and tell a program's
names (`Local`) from the compiler's (`Made`).

## S1a (2026-11-14)

`ExprKind::Var(String)` is `ExprKind::Var(Name)`; `Name` derefs to its
word, a `String`, so every pass reads it as before, and each site that
gave or took a `String` converts, 53 files, no logic changed. The corpus,
the snapshots and the port are as they were.

Mutation `wrap-call-an-effect` asked `value_keys.rs`, whose 64-bit
constant is folded now (216a72342), so no `BigInt.asIntN` was left in it
and the mutation survived main's run; it asks `stock_orders.rs` and
`byte_constants_large.rs`, whose wraps are held where they'd be read
after what else they do, and is caught.

## S1a-2 and S1b's first part (2026-11-14)

Every other name the tree holds is a `Name` too: a pattern's names, its
rest and its defaults; `Const`'s and `Let`'s; a catch's parameter; each
label; a function's, a module constant's and a namespace's name. Then
`Name` derefs to `str`, not a `String`, and nothing writes through it: a
rename is `Name::respell`, and each walker that renames takes `&mut Name`.
An `Option<Name>` reads as `.as_deref()` again, so no file grew. The
corpus, the snapshots and the port are as they were.

Two mutations no program reaches since M2 made each callback's name the
item's own, `js-substitute-shadowed` and `mir-tidy-plain-cell-shadowed`,
are each held by a unit test of a tree that binds a name twice, as a tree
used again still does till S7. `held-builder-finished-first-lost` is
#1776's, its branch unreachable since #1760.

## S1b's second part (2026-11-15)

`Name` no longer derefs to its word. A pass asks `word()` for it, and
compares a name with a given word by `is("Boolean")`, the one way to: the
lead's choice for Tu, so `PartialEq<str>`, `PartialEq<String>` and their
mirrors are gone, as are `as_str`, `word`'s synonym, and `From<Name> for
String`, `into_word`'s. A name compared with a name is still `==`, as
both are one type. The 1,328 sites rustc then refused, in 139 files, were
rewritten from its errors, each comparison `x == y` of a name the name's
`is`, with no logic changed. The corpus, the snapshots and the port are
as they were.

The 290 mutations whose text changed with them are retargeted: each one's
find moved to the rewritten text, and its replace carried across by the
same rewrite, every one of them built to prove it. `name-unlike-its-word`
and `name-as-no-word` now break `is` and `word`.
Five whose replace was written of the old forms, `name.as_str()` or
`v == "x"`, were rewritten too, and are caught. Two architecture rules
that found a search by name, `ExprKind::Var(v) if v == name`, find its
`v.is(name)` as well, so neither misses one written the new way. Three
files the rewrite would have grown past the cap's 1,350 lines were each
split by concept first, in commits of their own.

## S2 (2026-11-16)

A name lowering makes is `Name::made(base, k)`, and what it's of is
`made_base()`, where lowering made a `String` of `names::unspelled` and
preparation's speller parsed it. The word's form, `value#1`, is still
`names.rs`'s, which writes it (`made`) and reads it (`made_base`), as
`fresh_apart` there and the validator's scopes, which keep words, ask of
a word. Lowering's `unspelled` gives a `Name`, which goes on as one into
each arrow's parameters and reads it makes, `Pattern: From<Name>` and
`Expr::var(impl Into<Name>)`, so no made name passes through a `String`
there.

Its word is still `value#1`, as the lead decided (2026-11-16): tidy's
kinship and the passes' sets of names still compare words, where a made
`value` must not be taken for the program's, so each step stays
byte-identical. A flag beside the word, a second answer to whether a name
is made, would differ from what the word-keeping passes read, so there is
none; the name's identity apart from its word is S4's table. The corpus,
the snapshots and the port are as they were; the architecture test of the
form names `names::made`, `Name::made` and `made_base` as its one writer,
maker and readers.

## S3 (2026-11-16)

A global's read, `Array` or a binding's `document`, is a name of its own
form, `Name::global(word)`, where it was a fact of the read,
`Known::Global`, which is gone: `Expr::global` makes one, `js::scopes`
takes a read for a global's by its name's `is_global()`, and preparation's
`shadows`, which reads a global's `const performance$1 = performance;`
alias as `performance`, gives each read the global's name. As a name, it's
the read's wherever the read goes, and no fact of its expression, which a
pass rebuilding one may set afresh, can lose it. It's the one answer to
whether a read is a global's, the fact it replaces gone, unlike a flag of
S2's made names beside their word, which the word-keeping passes would
have answered apart.

Two names are still one where their words are, a global's beside a
local's of its word too: every pass compares them so, and which one a
read is of is JS's scopes', whose resolution `js::validate` checks, so
each step stays byte-identical. A local of a global's word is S4's to
tell apart. The corpus, the snapshots and the port are as they were; the
four mutations that took the mark away take the name's form instead.
`--without-known Global`, which forgot every mark so `test/validate.test.ts`
could see a read refused, has no fact to forget: the refusal is
`output::refusal`'s, a function of the module, whose unit test gives it a
local's read of `console`, and `tree-unvalidated` is held by that test.

## S4a (2026-11-16)

A MIR local's name is a `js::Name` of its form, `Locals.names:
IndexVec<Local, Name>`: a temporary's, `_5`, is `Name::temporary`, and the
name MIR's lowering holds an argument by, `_arg`, `Name::held_arg`, each
made by `Base::name` as the name's word is picked, which decides the form
once: of a MIR local, `Temporary` or `HeldArg`, its own; of another base,
a temporary's where it's spelled as one, `_0` of a positional field's or
`_$1` of an unread parameter's `_` given apart, as tidy has always taken
them (`temporary_names::spelled_as_temporary`, that spelling's one owner).
Tidy's `is_temporary` and `is_held_arg` ask the name's form, not its word.
A rename that's a spelling of the same variable, reclaiming `node$1` as
`node`, unsuffixing a parameter or giving one apart (`Name::given_apart`),
is `respell`, which keeps the form; tidy's naming a temporary for what it
holds, `_5` as `items`, makes the program's name of it.

So the form goes on, a local's read is made of its name
(`temporaries::read_by_name`, `LocalFacts::variable`), as is each read
`Expr::var` makes of a name, never of its word; tidy's own temporaries,
`applied_values`' `_$1`, are temporaries' names; a writer's text is its
local's name from its making (`body_writer`) to its epilogue; an
assignment's target that tidy folds, `OneAssignment::target`,
`assigned_only`, `assigned_through` and `chained_arms`, is the name it
assigns; a name made after another's, a closure's own copy of what it
captured, `_24$1` of `_24`, is `FnCx::fresh_name`'s, of the form
`Base::name` gives; and what tidy takes apart or declares again keeps its
names, a destructuring's parts (`destructuring::Part`), a plain cell's
reads, and the `let`s a loop's way out declares before it
(`declared_names`, of `Pattern::bound`, whose words are `names()` and
`declared`). The unit tests that build a temporary name it as lowering
does (`Base::name`).

What still keeps a name as text, tidy's sets of names, asks
`spelled_as_temporary` of the word, its old answer, till S4b gives each a
name. `RUST_JS_CHECK_NAMES`, in a debug build, checks each of
`is_temporary`'s and `is_held_arg`'s answers against the word's, the
answer before names had forms, and fails where they differ: the corpus
passes with it set, and it stays, to be set again where a name's form
changes (the lead's decision). The corpus, the snapshots and the port are
as they were.

## S4b (2026-11-16)

Tidy asks no variable's word whether it's a temporary's: the four that
asked a name kept as text, a drop guard's `let` set once in a `try` and a
`const` a cell's value is taken apart into (`bindings::alias_variables`),
what a destructuring's parts are read of (`destructuring::part_of`, now
the name it reads), an unnamed parameter taken apart where it's given, and
a `for`'s item a drop guard binds (`jumps/tried.rs`' `binding`), ask the
name's form. `spelled_as_temporary` is asked only of a name as it's made
(`Base::name`), of a positional field's key, `_0`, and of a drop flag's
base, which are no variables, and by `RUST_JS_CHECK_NAMES`. Tidy's sets of
names, `captured`, `fixed` and the rest, hold words still: what's asked of
them is whether a name is in them, never its kind, so they're S6's, with
tidy's own names. The corpus, the snapshots, std and the port are as they
were; four mutations retargeted.

## S4c (2026-11-16)

A pass that holds a name asks it whether it was given apart, never its
word: `Name::apart_of` (`("node", 2)` of `node$2`), `root` (`node` of it
and of `node`) and `given_apart_of` (`node$1` of `node` or of `node$2`,
tidy's kinship, where `bindings::kin` parsed both words). Tidy's
unsuffixing of an arrow's parameters, its shadows, `js::applied`'s
parameter given apart and preparation's shadows ask so. The spelling is
`names.rs`' alone, which writes it (`given_apart`) and reads it (`apart`);
besides `name.rs`, the reader is preparation's speller, which reclaims
`node$1` as `node`, and the two passes that keep names as words till S6,
a `catch`'s declarations (`prepare/catches.rs`) and tidy's shadows' kin
of the words its statements declare. An architecture test holds that.

There's no root recorded beside the word, the lead's decision on this
step's scope (2026-11-16): a Rust name has no `$`, so the word answers
exactly, and a second record of it would be a second answer; the name's
identity apart from its word is S8's, as each word becomes preparation's.
The corpus, the snapshots and the port are as they were; five mutations
retargeted, `given-last-of-another-name` breaking `given_apart_of`.
