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

S6, S7 and S8 are one step since 2026-11-16 ("Names are ids" below): each
needs a name's identity apart from its word, which none of them can have
alone.

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
#1776's, its branch unreachable since #1760, and retired with it since
(ADR 0532, amended).

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

## S5 (2026-11-16)

A block's label MIR's lowering makes, `bb7`, is a name of its form,
`Name::block_label`, made by `mir::queries::label`, and tidy asks
`is_block_label` of the label, its form, never its word, where it parsed
`bb` and digits: whether a label is MIR's, to be dropped or named, or one
the program wrote, `'rows`, which stays as it's written. So the form goes
on, what moves a jump from one label to another gives it the label itself
(`tails::relabeled`, `case_rests::placed`, `branch_ends::ending`,
`threaded_jumps`), never a word; a label tidy makes readable, `loop`, is
the program's. The census of labels, which counts block labels by their
words, reads a word's block by `block_of_label`, `label`'s spelling read
back. `RUST_JS_CHECK_NAMES` checks `is_block_label`'s answers against the
spelling as it does a temporary's. The corpus, the snapshots and the port
are as they were; eight mutations retargeted.

## Names are ids: S6–S8 as one step (2026-11-16, the lead's approval)

### Where M3 stands

S1–S5 put every name behind `js::Name` and gave the names whose kind a
pass asks a form decided once, as they're made: a MIR temporary, a held
argument, a block label, a global, a name lowering made (`value#1`). No
pass reads a kind off a word. What's left is identity: two names are one
where their words are, so a pass can't tell the program's inner `x` from
its outer `x`, nor a tree used twice from two trees.

### What identity needs, measured

Equality by id holds only if every read of a variable is made from its
declaration's name, never from its text. Today:

- of `Expr::var`'s 731 sites, 211 read a literal word, `Expr::var("x")`,
  how lowering writes its helpers' and roles' variables, and most of the
  rest a `String` or a `&str`; 90 `Const`s and `Let`s declare a name made
  of text (`.into()`), besides the patterns and parameters;
- the passes that ask about a variable key by its word: `mentions_in(&str)`,
  `substitute(&dyn Fn(&str))`, `Writes`' sets, `names_in`, tidy's 113
  `IndexSet<String>`s, preparation's maps;
- trees are cloned where needed twice (an arrow given twice, a closure's
  body inlined), so one declaration appears twice.

A name made of text can't know its id without the item's table, which
`From<&str>` hasn't got. So ids can't be added beside words and switched
on: every text-made read becomes a different variable, and the corpus
fails everywhere. Nor can ids be optional, `Some` comparing by id and
`None` by word: that equality isn't transitive (`a#1 == "a" == a#2` but
`a#1 != a#2`), which `IndexSet` and every `==` would break on.

### Decision proposed

**1. The table.** Each item lowered has a `Names` table, `FnCx::names`
extended from the words it takes today: `IndexVec<VarId, Entry>`, each
entry its provisional word and its form (S2–S5's). A `Name` is
`(VarId, Of)` with the item's table reachable where the name's read:
lowering through `FnCx`, tidy through its `Hints` (the table moved into
them as tidy starts, as `Named` and `LetNames` are), preparation through
`js::Item`, which takes its table from lowering, so printing spells each
id. A global's name stays its word, `Of::Global(String)`, as JS resolves
it by spelling; a label is an id of the same table.

**2. No text constructor, then equality by id.** First, while names
still compare by word, so each commit is byte-identical:
`From<&str>`/`From<String>` go, a variable declared with
`names.declare(base, form) -> Name` and read with `name.clone()`, a
helper's or a global's with `Name::global(word)` / `Expr::helper`. Each
text-made read is traced to its declaration, as S4a traced each lost
form, compiler error by compiler error, area by area (lowering, tidy,
preparation), each area one commit; `RUST_JS_CHECK_NAMES` gains a check
that a read's id is its declaration's. Last, one commit flips `Name: Eq`
to compare ids: the only commit whose output can differ, and the one the
judging below is of.

**3. Every keyed structure keys by `Name`.** `mentions_in`,
`substitute`, `Writes`, `names_in` and the sets take `&Name`; the pools of
taken words `fresh_in` picks from stay words, as they're spellings, and
go with step 5.

**4. A tree used again gets its own binders (S7).** Where a subtree is
cloned to be used twice, `js::rebound(&mut Expr, &mut Names)` gives each
binder it declares a fresh id and its reads with it; `js::validate`
refuses a module whose item declares one id twice (`Problem::DeclaredTwice`),
so a clone that misses one is a compile error naming the pass, never a
wrong answer.

**4b. A subtree moved into another item's table is remapped, by one
owner.** A pass that moves code from one item into another, or into the
module's scope, carries names whose ids are of the item it came from: each
is remapped into the item it goes to through `js::rebound` or its
cross-item sibling, `js::rebound_into(&mut Expr, from: &Names, to: &mut
Names)`, the one owner of it, never by a pass's own walk. The passes that
move code so, from the survey of 2026-11-16, each confirmed as step 1
reaches it: `prepare/initializers.rs` (a thread-local's initializer and a
closure's `const` made the module's statements), `prepare/forwarding.rs`
(a forwarding function's call taken apart where its value is made),
`lower/drops/inlined.rs` (a `Drop` impl's body where it's dropped),
`lower/pipeline/copies.rs` (a function's copy, alike another's),
`lower/classes/prototypes.rs` (a class's statements placed at module
scope), the dictionary caches the pipeline hoists (`keep_dictionary`,
`names::dictionary_cache`), and `output.rs`' registrations. `js::validate`
refuses an id that isn't in its item's table (`Problem::Foreign`), so a
move that skips the remap is a compile error naming the pass, never a read
of another item's variable; a pass found later is added to this list in
the commit that remaps it.

**4c. A name printed without its table.** Where a `Name` is shown with no
table to spell it, its `Debug`, a refusal's text, `--explain`'s and
`--knowns`' output, a contract's or a validator's `Problem`, it shows its
provisional word and its id, `value#3`, so two alike are told apart. Those
are the only outputs that may show an id: none reaches generated JS, which
preparation's spelling alone writes, nor a declaration file, a manifest or
a source map's names. An architecture test refuses `Name`'s `Display` in
`to_oxc` and the declarations' printer, and the validator refuses a module
whose printed word has `#`.

**5. Spelling is preparation's (S8).** Lowering's words become
suggestions (`Entry::base`): preparation spells every id once, apart from
what's in scope, as `prepare::reclaim::spelled` spells a made name today,
and `name.word()` is preparation's alone, an architecture test refusing
it elsewhere.

### How each output difference is judged (the lead's rule)

Only the flip of step 2 changes which names are one (steps 1, 2's
conversion, 3 and 4 are byte-identical, as names still compare by word): where two variables are spelled
alike, a pass that took them for one now tells them apart. That commit is run over the corpus, the snapshots, std and both ports
byte-diffed against main. Each difference is one of two:

- **a fix**: the old JS read or wrote the wrong variable, a shadowing or
  a capture bug. It gets a corpus row that shows it against native Rust
  (the old JS failing it), and it's reported to the lead before the push;
- **prevented**: a difference that changes readability only, where the old
  JS was right, is made not to happen (the pass given the old answer by
  the new key, e.g. the name's root), never accepted silently.

No difference lands unjudged. Step 5 changes spelling only where a word
would be spelled differently by preparation than by lowering's
`fresh_in`; preparation reproduces `fresh_in`'s order (first come, `$k`
apart), and a difference is a readability one, prevented or sent to the
lead as a proposed change of the guide's naming.

### Cost, honestly

This is the biggest change of M3: every `Expr::var` of a text, every
pass keyed by a word. My estimate is 4–6 working days, area by area:
lowering's helpers and roles (~2 days), tidy (~1.5), preparation and js
(~1), the flip and its judging (~0.5), spelling (~1). Each area lands
alone, byte-identical; the flip lands with its differences judged. The
alternative, stopping at S5, leaves a name its word for identity: correct
today (Rust's names can't collide with what lowering makes, since M2 and
S2), but tidy can't tell shadowed variables apart, which is where
#1681's review found the risk.

M4 (statement facts) doesn't touch names, so it goes alongside (the
lead's decision).

## Names are ids, step 1's first part: lowering's names stay names (2026-11-16)

A name lowering picks is a `Name` from where it's picked to each of its
reads, never its word again: `fresh` gives the `Name` of the form its base
makes (`Base::name`), where it gave a `String` that each read made a name
of anew, and `fresh_name`, the one that gave a `Name`, is gone into it, so
there's one way to pick one. THIR's `bind` gives the binding's `Name`, and
what holds a picked name holds it as one: a drop given (`GivenDrop`), each
drop flag (`NamingFacts::flags`), a catch's error name (`catch_names`),
`names_array`'s names, a pattern's rest. Tidy's passes that put a name of
their own in the place of another, its temporaries named (`temporaries`),
a binding a temporary is taken into, a shadow's binding (`shadows`), a
flag named after what it guards (`flags`), a loop's counter and its held
end (`loops`), a literal's fields' variables (`scopes`), make the name
once, `Name::declared`, and give the declaration and each read a clone of
it, where each made one of the word. A name declared of a word the pass
spells itself, a `catch (error)`'s, is `Name::declared` too.

`Name::declared` is what `From<&str>` is to become once step 1 has traced
each name made of a word to one: a name made of text, `Expr::var("x")`, is
what's left. Names still compare by word, so nothing reads which name a
name is yet: the corpus, the snapshots, std and the port are as they were.
A probe of the corpus that records where each name made of text was made
counted 176,065 uses of 250 sites at the start and 135,541 after this.
Most of what's left is names the module declares, its runtime's helpers,
its functions and its imports, which are no item's (the lead's decision of
2026-11-16): their form, `Name::module`, is the next part, with a row of a
local beside a helper and an imported function of its word.

## Names are ids, step 1's third area: preparation's and js's names stay names (2026-11-16)

Preparation's passes and `js`'s helpers that declare a variable of their
own, a word they pick, declare it once, `Name::declared`, and give each
read a clone of it; one that puts back a read, or moves a name aside and
back, keeps the name it had, never one made again of its word:

- **Declared once:** a handle's setter's parameter (`prepare.rs`, `value`,
  `newValue` or `value_`), a `catch`'s thrown payload's binding
  (`catches::thrown_name`, `thrown`) and its own `error`, `this`'s `const`
  a function's handles read (`js::this_outside_accessors`), and a
  parameter written again, given its own name apart (`js::applied`).
- **Kept as it was:** a shadow's reads, put back as the read the `const`
  shadowed was, a global's first (`shadows`); an advance's variable, the
  name its assignment writes (`do_whiles`); a parameter set apart while a
  renaming runs, respelled and given back, not replaced
  (`js::outside_shadows`).
- **A global's:** the `String` a template's conversion reads, which
  `reclaim::names_mut` gives as `Name::global("String")`.

Their word comparisons, `mentions_in`, `mentions_var` and `Writes::sets`
of a word, stay words, which step 3 keys by `Name`. The two placeholders a
pass writes and takes out again before it ends, `do_whiles`' `$given` hole
and `this_outside_accessors`' nested-body marks, are no variable and stay.

Measured by step 1's probe, run after `prepare::module`, over the
corpus: preparation's and js's sites, 17 of 202 with 2,183 of 133,783
uses, are none of 183 (131,638 uses). The uses that are left are
lowering's, its module names and its own, whose names the passes above now
keep where they replaced them. The corpus's JS is byte-identical under
`RUST_JS_CHECK_NAMES=1`, as are the snapshots and the React port.

## Names are ids, step 1's second part: the module's names (2026-11-16)

A name the module declares at its top is a form of its own,
`Name::module(word)`, the lead's decision: its functions' names and the
reads of them, its imports' locals, the linker's `Symbol`s made reads,
the trait impls' caches, and the runtime's helpers it imports,
`Expr::helper`'s. A function or a `const` written in a function's body,
or one a block makes and gives, is declared where it's written, so its
name is its body's (`CrateFacts::at_module_top`), as is a drop function,
declared at the top of the body that drops. They're no item's: an item's table, which step 2 gives ids, never holds one, and a
read of one is the module's wherever in the module it is, as JS resolves
it. `Name::global` stays what the module declares nowhere, `Array`'s or a
binding's `document`, which `js::validate` checks apart: a global's read
must find nothing in scope, a module name's read the module's top.

**Equality across forms.** A module name is never an item's local, even
of its word: a local of a module name's word reads the local in JS, so no
read meant for the module's may be inside it. Lowering names a local apart
from every name the module has, as it has since M2 (`fresh_in` starts of
`module_names`, its functions, imports and globals), and preparation's
`reclaim` respells one apart from them too; `js::validate` refuses a
module name's read that a nearer declaration takes
(`Problem::ModuleShadowed`), so a pass that would let a local capture
one is a compile error, never a read of the wrong variable. Whose name a
read is, a local's, a global's or the module's, is one state,
`js::scopes::Whose`, where a read had a flag of each. Corpus case
`locals_named_as_the_modules_names` holds it: `let parse = parse(a) ..`
of an imported `parse`, a local `cmp` beside the module's `cmp` and the
`$cmp` a comparison of strings reads, each against native Rust.

**One authority for the helpers a module imports.** The linker imports
each runtime helper its module's tree reads, by the read's word, as it
did: `Name::module` keeps the word, so what's imported is decided where it
was, of the same names, and no second list says it.

The corpus, the snapshots and the port are as they were, and no module
name's read was refused. The probe of part 1 counts 10,813 uses of names
made of text left, of 135,541: the rest are the items' own, lowering's
and tidy's (`cell/variables`' module cells, `destructuring`'s nested
patterns, a dictionary's shared entries), and preparation's, step 1's
third area, which ioCJ takes in parallel (the lead's split).

## Names are ids, step 1's third part: lowering's own names, first batch (2026-11-16)

Of lowering's modules that are mine in the lead's split (ioCJ takes drops,
classes, `std_types`, display, combinators and `std_impls`; m3-max tidy):

- A module's own functions and constants are the module's names: a
  `const` at the module's top (`pipeline/consts.rs`, `pipeline.rs`) and
  each function `module_fn` makes, a dictionary's, a function's value's,
  a `fn` pointer's or a recursive one's, are `Name::module`, one written in
  a function's body keeping its body's. `recursive_fn`, `module_fn` and
  `local_recursive_fn` give the declaration's own `Name` on each path, so a
  caller, `std_impls`' and `ordering`'s, reads it without asking which.
- A runtime helper is `Expr::helper`'s, `$rcClone` and `$weakClone` too.
- Parameters a pass spells itself, `(to, from)`, `(x, i)`, `[key, ..]`,
  `I{fact}`, a transmute's, `iterator` and a user `next`'s, a probe's
  `arg{i}`, are made once, `Name::declared`, and cloned into each read.

The corpus, the snapshots, std and the port are as they were. The probe
counts 3,454 uses of names made of text left in all areas, of 7,027 after
part 2.

## Names are ids, step 1's third part, second batch: a body's items (2026-11-16)

A crate function or `const` written in a function's body, or a function a
block makes and gives, is declared there by the item's own lowering and
read by the body's: two lowerings, neither of which holds the other's
`Name`, and nothing moved with its names, so no remap (`js::rebound_into`)
applies. Its name is the item's, `Name::item(word, item, copy)`, of its
`DefIndex` and of the copy lowering emits of it, `None` of the item
itself (the lead's decision, 2026-11-16): each lowering makes it of the
item it knows, so the declaration and its reads are one variable, and two
instances of one item, a generic inner function's copies, are two.
`CrateFacts::item_name` is the one place that asks which form an item's
name has, the module's at its top (part 2) or its body's, and the
declaration (`mir.rs`), its reads (`items.rs`) and a body's `const`s
(`pipeline`) ask it. A scope struct's methods, declared and read by one
lowering, the function's, are named once there, `Name::declared`, and
cloned.

When names compare by id, `js::validate` refuses two declarations of one
item's name in one scope, as it refuses two of one word today
(`Problem::Twice`). Corpus case `inner_generic_fn_of_two_types` holds a
generic inner function called with two types in one body, one function
given each type's dictionary, beside a plain one and one read from a
closure, against native Rust. A copy lowering emits of one, a dictionary
given in place (ADR 0051), is keyed by its copy, so it's never its item.

The rest of lowering's names are traced too: a parameter pattern's names
and a scope method's `self` are the bindings' own (`ParamPattern`,
`scope_self`), a closure's taken captures and a formatter it captured keep
their `Name`s, a writer's `options` is read through its parameter's, and
what a pass spells itself, `_`, `it`, `item`, `key`, `result`, `from`,
`dropI`, a variant's positional parameters, a named function expression's
own name, is made once, `Name::declared`, and cloned. The probe of part 1
counts no name of an item's made of text in the corpus as lowered, tidied
included: step 1 is done, the flip of `Name: Eq` to ids next, whose
output is judged as this ADR's "Names are ids" section says.

## Names are ids, step 1's measure at creation (2026-11-16)

The flip, run once, showed that the probe's zero held only for names left
in the finished module: a pass that makes a name of a word and then merges
or drops it, tidy's `flag_values`' `Expr::var(x.word())` beside the
temporary `_3`, leaves no trace there, and at the flip `declarations` took
the two for two variables and dropped the write, a wrong answer
(`flag_of_a_guarded_match`). So step 1's gate is the creation of names: no
name of an item's is made of a word (`From<&str>`, `From<String>`,
`Expr::var` of one) anywhere in a compile of the corpus, std or the
ports, which a probe of `#[track_caller]` on those reports by the line that
made it (the lead's decision). Of 19,196 such creations in the corpus,
lowering's are none now: a format's stand-ins (`_$argumentN`), lowering's
placeholders for no name, a dictionary's and a variant tag's module
`const`s, a namespace's, the sentinels a pass substitutes (`SUBJECT`,
`HELD_SOME`, `value`), and parameters a pass spells itself (`it`, `lazy`,
`cycle`, `location`). Tidy's and preparation's are m3-max's and ioCK's,
the labels the program writes taking the form approved for them.

## Names are ids, step 1 at creation across the whole check (2026-11-16)

The corpus alone reached too little of the compiler: the flip, judged
over the guide, the tests and React's port, stopped on a label tidy made
of its word, `getTag`, beside the program's `Name::label` of it. So
step 1's measure is the creation of a name of an item's from a word over
everything a push checks: `check:fast --all`, std, `bun run docs`, React's
port and alien-signals (the lead's decision). It counted 9,100 such
creations at 217 sites, which are none now but in unit tests, whose trees
are built of words alike and compare so: serde's writers and readers carry
their `json`, `content` and codecs' names as names; an import's local, a
namespace's, a held module read, a test's registration and a module's held
`const` are the module's (`Name::module`); a labeled `switch`'s label is
the program's (`Name::label`); a variable written back is the variable's,
its reads its `Name`; and what a pass spells itself, a left-out field's
`_`, a named function expression's property, a `Deref`'s parameter, an
`Error`'s held field, a dynamic import's `m`, `peekable`, `it`, `m` and
`drop`, is made once and cloned. Names still compare by word: the corpus,
the snapshots, std and the port are as they were.

## Names are ids, step 2: the flip (2026-11-16)

`Name` has an id: each constructor that declares a variable makes a fresh
one (`declared`, `temporary`, `held_arg`, `made`, `given_apart`), its
clones keep it, and `respell` keeps it. Two names are one where their
forms and ids are; a name whose form and word say which it is has
`Id::WORD` and is one by its form and word: a global's, the module's, a
body item's, a label the program wrote, a block's label (each block's
word its own), and what's still made of a word. `Ord` and `Borrow<str>`,
whose contracts a key by id breaks, are gone, their four uses keyed by
the word as before. `Debug` shows `word#id` where a name has an id.

Judged over the corpus, the snapshots, std, the guide's fences, React's
port and alien-signals (the lead's run), and run over the corpus before
anything else changed, 1,460 cases' JS
differed, then 485 once block labels were by their word; among them a
wrong answer, `flag_of_a_guarded_match`, whose cause, a name tidy made of
a word mid-pass, step 1's measure at creation found and m3-max's tidy
commit removed. Every other difference was readability, the old JS right,
and each was prevented, none accepted (the lead's rule):

1. Trees made apart compared as trees, not as variables:
   `js::scopes::alpha_eq` blanks each name whole, not its spelling, so its
   id goes too; `stmt_walk::canonical`, `stmts_alike`'s and `stmts_same`'s,
   numbers each binder by a name of its number alone; and the comparisons
   of trees made apart are `alpha_eq`'s, `copies::settle`'s of a copy and
   its function (`alpha_eq_functions`), a class's default methods', and
   `enumerating()`'s. Of `copies::settle`, a copy wasn't found alike its
   function, `Stack.createCursorBoxed` kept beside `createCursor`.
2. A pass that makes two variables one gives the reads the one left:
   `js::merged(stmts, from, into)`, tidy's shadows, a parameter applied in
   place, a value set into its variable; `js::renamed` respells one
   variable.
3. A tree used again gets declarations of its own: a body applied in
   place, an inlined `Drop`'s at each drop, `js::rebound` (`Name::rebound`)
   as this ADR's point 4 planned.
4. Algorithms keyed by a word dedupe and match by the word, as main does:
   `reclaim::spelled`'s and `reclaim::locals`' declarations,
   `reclaim::aliased_imports`' count of a name declared elsewhere in the
   module, and `drops::inlined`'s copies (`copies_read_as`), a guard's
   fields' copies (`kept_variables`) and its `const` of itself
   (`fields_of`), which JS reads before it's made whatever its id. They are
   step 3's, whose structures key by `Name`, to move to ids, on record here
   so they aren't forgotten.
5. Preparation's record of which of its rewrites fired asked `==` alone,
   which a respelled name, its id kept, leaves alike: a rewrite fired where
   the tree or its names' spellings changed (`passes::Spelled`), so
   `--prepare-without` of one that fired nowhere leaves the JS as it is.
6. What's noted of a function's lowerings is gathered from each: a
   function lowered as two copies declares the drops it's given twice,
   each `dropE` a name of its own, and the names `drops::uses` leaves out
   where they go unused (ADR 0300) are each copy's, not the last one's.
   The crate corpus found it, not the corpus, whose generic functions are
   a binary's: base64's `ChunkedEncoder::encode` kept `dropE`, wasn't
   merged with its copy, and its `const`s' hole was left unfilled, the
   crate refused (`CHUNK_SIZE` read, nothing declaring it). Pair case
   `library_copies_unused_drops_each_lowering` holds it against native Rust.

A label tidy makes where the program wrote none, `loop`, is spelled apart
from every label around it and in it (`switches.rs`), so two nested are
two words and two labels (#1969's rule); none reaches JS, and corpus case
`switches_leaving_their_loops_nested` holds the nested loops' exits
against native Rust.

## A local apart from a helper its scope reads (2026-11-16, #1975)

Lowering names a local apart from the module's functions, imports and
globals (`fresh_in` of `module_names`), but not from the runtime's
helpers: which a module imports is decided by the reads its linked tree
has, and std's 510 helpers without a `$`, `copiedClone`, `mapClone`,
`chain`, `lines`, are words a program's locals have too, so naming every
local apart from all of them would respell `lines` wherever it's written.
A local of a helper's word in whose scope the helper is read, `main`'s
dictionary `copiedClone` that `declare_repeated` hoists (ADR 0979) beside
a `show(..)` whose dictionary reads std's `copiedClone(..)`, or a program's
own `let copied_clone`, was refused as `Problem::ModuleShadowed`.

The linker, once a module's tree is linked and its reads are all there,
gives each such local way: `js::scopes::locals_apart_from_module_reads`
asks the module's scopes for each variable a module name's read is in
the scope of (`Tree::taking_module_reads`, the validator's
`ModuleShadowed` case, by the variable's `Name`, so its id), and respells
it, its declaration and its reads, by its id, apart from every name the
module has (`js::respelled`): `const copiedClone$1`. The apart spelling is
`names::fresh_in`'s, the one lowering's locals are given apart by: no
second way to pick a `$1`. A helper is imported by its word, and passes
ask a helper's read by it (`helper_name()`), so the local gives way, as a
local gives way to a function of the module's (`cmp$1`), not the import as
a crate's import does (`parse$1`). It changes only modules refused before,
and corpus cases `local_named_as_a_helper_its_scope_reads` (a program's
`copied_clone`, `chain` and a loop's `map_clone`) and
`clones_beside_a_generic_clone_of_a_copied_map` hold it against native Rust.

It's the linker's because only the linker knows the final reads (the
lead's decision). When preparation becomes the one speller, the spelling
step's end, this moves into it, the linked module's reads given to it as
an input, rather than staying a respelling at link time.

## Names are ids, step 3: what a tree mentions, by id (2026-11-16)

Step 3 moves the questions lowering and tidy ask by a variable's word to
its id, a slice at a time. The first is what a tree mentions. The one
walk of what an expression or statements read yields each read's `Name`
(`Expr::visit_names`, `js::visit_stmt_names`, `Function::visit_names`),
so its id; `visit_vars` and `visit_stmts` are its words, a view of it, not
a second walk. `js::mentions_of(stmts, &Name)` and `Expr::mentions_name`
count and find a variable by its id; `js::mentions_in` and
`Expr::mentions_var` are kept for what JS reads by a word, preparation's
questions of spelling and shadowing until the spelling step. Lowering's
and tidy's 138 asks of a variable by its word, `mentions_in(x,
name.word())`, ask `mentions_of(x, name)`.

Asked by its word, a variable another of its word was in the scope of
looked mentioned where it wasn't, and tidy kept what it could take out.
Four corpus cases read better, each as native Rust prints: std_enumerate's
`while (true) { const value = $next(chars); .. }` is a `for .. of`, a later
`value` having blocked it; max_by_drops_its_function_first's `const value
= result; let m = value;` is `let m = result;`; temporaries_taken_apart's
`const value = make(4); loudDrop_drop(value);` is
`loudDrop_drop(make(4));`; write_result_items' hoisted `let fails;` is
declared where it's set, a closure's own `fails` having blocked it. React's
port's `renderSuspenseListRows` declares `const i` in its loop, as upstream
does, not a hoisted `let i`.

Reading a string literal's text as a variable, which mutation
strings-read-as-vars stood for, is a type error now: the walk yields
`&Name`, and a literal holds a `String`. The mutation, which no longer
builds, is gone; the type holds it.

The block asked by a word wasn't guarding anything else by accident: a
moved read is the variable it is, and preparation spells two variables of
one word apart by their ids, so it never lands where another of its word
takes it. Corpus case `moved_reads_beside_variables_of_their_word` holds
it: `make(4)`'s temporary moved into its drop beside the program's own
`value`, one shadowing it in a block, a closure's own `value` and `fails`,
and a loop's `value` after a `while let`, each against native Rust. The
four changed cases and it pass the tidy-stages net (each stage of tidy,
and preparation without each of its rewrites, against native) and the
metamorphic net's rewrites in both builds. And one guard of tidy's
`sunk_lets` that asked by a word is gone: `reads_past_shadows` kept a
`let` from sinking to a closure whose parameter of its word shadowed it,
`(a, b) => ..`; asked by its id, the closure's `b` is never the `let`'s
variable, so the guard decided nothing, and its mutation,
sunk-let-at-shadowing-closure, survived with every snapshot compared. Its
part for an index loop's `for (let i = 0; ..)` in a closure, another `i`
than the program's later `let mut i` (rd356), went with it: that loop's
`i` is a variable of its own, and the test that held it,
`lowering-index-loops`' "an index loop's counter is no program's later
let", still passes; its mutation, sunk-let-past-closure-counter-unseen, had
nothing left to change. Nothing else moved: the
corpus, std, the guide's fences, alien-signals and the Vite example are as
they were.

These readability gains are the step's expected effect, the lead's
decision: a later slice of step 3 whose only differences are of this kind,
a rewrite of tidy's or preparation's that asking by a word blocked where
another variable shared it, each case still as native Rust prints, may
push under the standing approval without asking. Any other kind of
difference still goes to the lead first.

## The program's names first (2026-11-16)

The spelling step's first rule, landed early (the lead's decision): where
a local the program named and a name rust-js made share a word in a
scope, the program's keeps the plain spelling and rust-js's gives way.
Generated names are a product feature, and a program's `state` taking
`$1` because a role word, a flattened state's spill `state`, got there
first reads as if the program had named it so.

Which names are the program's is rustc's answer, decided where a name is
made, as a name's other forms are, never kept by hand: `Name::program`,
of `Base::Program`, is a MIR local whose debuginfo is a user variable's
(`mir_locals`) and each variable a pattern binds, rustc's `LocalVarId`
(`FnCx::bind`), a destructured parameter's too; a crate item written in a
body, of its `DefId`, is the program's too (`Name::is_the_programs`); and
two names made of one, a scope struct's method of the program's method
(ADR 1043) and a closure's own copy of a captured local the program named,
the variable moved in, are the program's as what they're of. In a debug
build where `RUST_JS_CHECK_NAMES` is set, each local rustc says the
program named is checked to be the program's.

Preparation's `reclaim::locals`, the one pass that gives a name back its
spelling, gives way first: a local rust-js made whose spelling is the base
a local the program named, given apart, would reclaim, `state` of
`state$3`, is spelled apart from every name its item has (`names::fresh_in`,
the one apart spelling), where its scopes let it be; then the program's
names reclaim theirs before rust-js's do. Where none contends, every
spelling is as it was. Five corpus cases and a snapshot changed, each as
native Rust prints, the program's name now plain: taken_and_returned's
`result` (the return's `result$1`), dyn_fn_generic_closure_drops' `result`
parameter (its closure's function `result$1`), copied_default_box_named_apart's
own `boxed` (the parameter's box `boxed$1`), std_mixed_enum_mut's loop
`bound`, the dynamic snapshot's loop `value`, and as_ref_and_borrow_through_a_mut's
boxes a suffix shorter; React's port, `processReply`'s `formData$2` a
suffix shorter (react-in-rust d546994). Mutations
program-name-without-way-given, program-names-not-first and
program-names-not-the-programs hold it.

## Names are ids, step 3's second slice: the program's names asked by id (2026-11-16)

Tidy's `Named` sets, the locals the program named (`user`), those a
`#[rust_js::name]` names (`given`) and the `String`s (`text`), hold each
local's `Name`, and the passes that ask them, `aliases`, `copied_bindings`,
`destructured`, `tested_options`, `shadowed` and `appended`, ask by its id.
Asked by its word, a `state` rust-js made, a flattened state's spill
(`Storage::written_in_place`), was found among the program's names by its
word alone, and the shadow pass merged the program's later `state` into
it, one `let` for two values; asked by id it's rust-js's, and the program's
name keeps the plain spelling by the rule above: flattened_state_handed_on's
`let state` is the program's handle and the spill `const state$1`, as
native Rust prints. Nothing else moved: the corpus, the snapshots, std,
the guide and React's port are as they were.

## Names are ids, step 3's third slice: what a body's `let`s bind, by id (2026-11-16)

Tidy's `LetNames`, the variables a body's `let` patterns bind, those its
`let`s name, the mutable, those whose borrow ends, the frozen and those of
an own field, hold each local's `Name`, and the passes asking them,
`destructured`, `tested_reads`, `ended_borrows`, `fallback_reads`,
`receiver_reads`, `and_tests` and `read_handles`, ask by its id; and
`read_handles` keeps each handle by its `Name`, not its word, from where
it's found to where it's read. Byte-identical: the corpus, the snapshots,
std, the guide and React's port are as they were. One ask by the word is
left there, `js::Writes`' changed variables, a set of words of `js::writes`'
own, which step 3 moves with that module.

## Names are ids, step 3's fourth slice: callbacks, kept captures and drop flags, by id (2026-11-16)

Tidy's hints of the closures a `let` names that a hook's function alone is
(`callbacks`), of the captures a closure keeps (`kept`) and of a body's drop
flags (`flags`) hold each variable's `Name`, made where lowering names it
(`callback_names`, `kept_captures`, `drops::flag_names`), and
`named_callbacks`, `aliases`' fixed names and `flags`' passes ask whether a
variable is one by its id. Byte-identical. Within `flags`, what's known of
each flag once it's found, `Known` and `assigned`, is still keyed by its
word, as its reads are counted (`read`): a flag's name is lowering's own,
apart from every other, so its word is one variable; they move with
`js::Writes` and the walks of what's read by a word.

## Names are ids, step 3's fifth slice: falsy, tested and what lowering moved in, by id (2026-11-16)

The temporaries a binding's call skips what's falsy of (`falsy`), those
read for their truth alone (`tested`), noted where they're made
(`note_falsy`, `note_tested`), and the values lowering made where they're
used (`Made::captured`), hold each variable's `Name`, and `falsy_values`,
`aliases`, `plain_reads`, `tuple_reads`, `guarded_lets` and `moved` ask
whether a variable is one by its id. Byte-identical.

## Names are ids, step 3's sixth slice: a place is a `js::Path` (2026-11-16)

Tidy's typed sets and its plain values held words, and a place's key was a
name's word spelled with its fields: `field_path`'s "element.editable",
`call_path`'s "isItem()", plain's "q.title" and "...items", jsx's
`path_of`. A place is a `js::Path` now (`js/path.rs`, the lead's
decision): its root a variable by its id, or `this`, its fields property
keys, which are words, and what it names, the place, what a call of it
gives or an array's items. `Expr::path` and `Expr::called_path` make one
of a tree, the one way; a chain with a computed member, an index or a call
in it is none, as `field_path`'s was. `field_path`, `call_path` and the
spelled keys are gone, and an architecture test refuses a function of an
`Expr` that keys a place by a `String` and a spelled `...items`.

`booleans` is a set of `Path`s, a variable one with no fields;
`numbers` and `floats` sets of variables; `plain`, `locals.mir.plain` and
`locals.mir.given` sets of `Path`s, the module's imports rooted at their
module-form names; `locals.mir.paths` keyed by `Path`. The typed sets'
subtraction stays, by id: several MIR locals can be one variable, which is
a `bool` only where each of them is one.

Asked by its word, a closure's own variable was found plain where its
enclosing function had a plain variable spelled the same, `result$2` of
each: a read of the closure's `Result` moved past calls by a coincidence of
words. Asked by id it isn't, and React's port's `processReply` keeps the
payload's `const`, so preparation makes its three `$catchUnwind` closures
`try { .. } catch`, as upstream 83b9c29 writes them. Mutation
plain-field-blocks-inline, whose JSX tests reached the plain root only
through that coincidence, is held by two corpus cases' snapshots.

## Names are ids, step 3's seventh slice: what a tree writes, by id (2026-11-16)

`js::Writes` holds the variables a tree writes as `Name`s, its replaced,
changed and handled ones, each kind in closures too, and how often each is
set; its questions take a `Name`, `may_replace`, `sets`, `count`, and
`replaces` and `changes` of the sets a pass read directly. Tidy's
`set_elsewhere` and `changing`, made of them, are sets of `Name`s, and the
passes asking them ask by id: `read_past_calls`, `moved`, `decided`,
`destructuring`'s parts set, `implied_tests`, `filtered_guards`,
`tested_options`, `unwinds`' closures' sets. Byte-identical, and a probe
that asked each question by the variable's id and by its word over the
corpus and React's port found no question they answer apart.

A helper that still holds a word, not a `Name`, asks step 3's migration
bridge, `Writes::may_replace_spelled`,
`replaces_in_closures_spelled` and `handles_spelled` (`sets_spelled`
too, until its last caller, `mir::scopes`, asked by id) (the lead's decision):
`drops::inlined`'s declared bindings. `test/writes-spelled-bridge.txt`
counts their calls, a ratchet that only falls; step 3 isn't done until
it's empty. Preparation's `shadows` asks what a closure may replace by a
variable's spelling, as JS reads it, its own question, the spelling step's
API (`replaced_in_closures_spelled`, `may_replace_spelled` in
`prepare/shadows.rs`), not the bridge.

The bridge's first group, tidy's `bindings.rs`, asks by id: `only_read`,
`set_once_in_try`, `read_before_set` (and `copied`'s `set_before_read`,
given the `Name`s it had) and `variable_clones`' and `clone_aliases`'
fixed variables, made of the parameters' and kept captures' `Name`s.
Byte-identical; a probe asking each by id and by the word over the corpus
and React's port found none apart. The ratchet falls from 13 to 9.

The bridge's second group, tidy's `shadows.rs`: `set_after_last_read`
asks whether the shadow is set and the old variable read by their ids.
Byte-identical, and the probe found none apart. The ratchet falls to 5.

The bridge's third group: `values.rs`' `unread_resets` keeps each reset's
`Name` and asks whether a handle is made on it, and whether a `finally`
reads it, by its id; `mir::scopes` asks whether a parameter is handled or
set by its id, and whether a nested declaration takes its spelling by its
word, as JS reads it. Byte-identical, and the probe found none apart. The
ratchet falls to 3, `drops::inlined`'s.

## Names are ids, step 3's bridge ends: `kept_variables` asks JS's spelling (2026-11-16)

The bridge's last asker, `drops::inlined`'s `kept_variables`, asks of the
variables' spellings by design, a question of JS's names as preparation's
shadows' is, and keeps asking it: `drops::inlined`'s own, not the bridge,
which is gone, its ratchet with it. A guard's `Drop` binds its fields,
`let Self { previous, .. } = self`, as variables named as the guard's
fields' variables; inlining the guard makes each the one JS variable by
their shared word, so a copy of a field into one of its name declares no
other. And a variable whose word another declaration has isn't kept,
since a read of it put where the field's read is may be in that one's
scope, which JS reads instead.

An attempt to ask it by an inlined copy's origin, a name `js::rebound`
made keeping the id it's a copy of, broke test/lowering-drop-guards.ts'
"a guard's fields are its variables, its scope read in place": the
`Drop`'s binding is no rebound copy, but another Rust variable, so by
origin the field's reads became the outer `previous` inside the binding's
`const previous = (previous * 2) >>> 0`, which throws. It found that only
because the whole suite ran, not the corpus alone: from now on a probe of
by id against by the word runs over all of `check:fast --all`. The
attempt, `origin` and `same_origin`, didn't land.

What ends the word here is the spelling step's: the `Drop`'s binding that
destructures a guard's field made that field's variable where it's
lowered, and two variables of one word spelled apart by their ids, after
which the guard needs neither the word's identity nor its capture guard.

## Names are ids, the spelling step (from 2026-11-16)

Step 3 asked the questions that decide meaning by a variable's id. The
spelling step moves what's left of a name's word to where names are
spelled: preparation and printing read a name's word; every other phase
asks of its `Name`, and makes a name made of another's, a derived one, as
a form of that `Name`, never a word spelled anew. `Name::shown()` writes
one in a diagnostic and never in JS. Its measure is
`test/word-reads.txt`: each file's reads of a word, `.word()` and
`.into_word()`, outside preparation and printing, 577 in 112 files when it
began, a ratchet that only falls (`test/architecture-names.test.ts`); the
step is done when only diagnostics read one. Its parts, each probed by id
against the word over all of `check:fast --all`: the naming-only sets
(`suggested`, `items_read`, `own_names`, `alike`, `settled`); lowering's
asks of `visit_vars` and `mentions_in`; preparation's by-word counts, as
`catches`' read of a `Result`; #1992, a `Drop`'s binding of a guard's
field made that field's variable where it's lowered; and the link's
respelling of a local apart from a helper it reads (#1975), which moves
into preparation, given the linked module's reads.


So far (2026-11-17), each slice byte-identical and probed: the body's own
sets (`own_names`, `alike`, `settled`, `borrowed_names`); tidy's asks of
where a `let` goes, of what's read, set, rethrown, captured, copied and
destructured; labels, each a `Name` (a label has no id of its own, `Id::WORD`,
so its form and word are what tell two apart); `js::declares`; and
lowering's comparisons of two names, `a.is(b.word())`, made `a == b`. Where
the word's answer was apart from the id's, it was the cautious one, another
variable of the word, a closure's own say. A local MIR made that nothing
names is `Name::default()`, asked by `Name::is_unnamed`, not by its empty
word; probed over the suite, the two never disagreed. A local that may have
no name would read better as an `Option<Name>` than as that sentinel: a
possible follow-up, not this step's. The ratchet stands at 290.

## Names are ids: link decides what's imported, preparation spells it (2026-11-17)

Link spelled each import of the crate's modules: it walked the module for
every word a binding or a read takes and gave each import the first word
none took, `useState$1`, then rewrote each read of what it imports to that
word, and respelled a local a runtime helper's read is in the scope of
(#1975). It decided both what a module imports and what it's called.

Now it decides only the first. Each import is a `Name` of its own,
`Name::imported`, a module's name with an id of its own, so two imports
of one word are two variables; each read of what it imports is that
`Name`; and link hands preparation what it spells them apart from,
`js::ImportNames`: lowering's reserved words, and the imports that may be
renamed in the order the module asked for them. `prepare/import_names.rs`,
before any other pass, spells each with link's rule and walk, then gives
a local way to a helper's read, as link did after it. A JS module's
import and a helper's keep their words, a local giving way to them, as a
reader expects `useState` imported as it is; two JS modules' of one word
are imported apart, `f` and `f$1`, as before (`link.test.ts`'s row).
Link reads no word (architecture test). Byte-identical over all of
`check:fast --all` and React's port: an import's spelling is printed in
its import and at each read, so the same JS is the same spelling, import
by import.

The word-reads ratchet falls by link's nine, but two files rise for now:
`output.rs`, 0 → 2, whose test harness picks its imports' aliases, `test`
and each module's namespace, against the module's names at output, and
`js/scopes.rs`, 7 → 8, its tree's top words for `js::validate`, each
reading an import's word now its local is a `Name`. The test harness's
aliases are the next step: imports' `Name`s on `js::ImportNames`, spelled
by preparation as link's are, which takes `output.rs` back to none.

The test harness's aliases are spelled so now too (2026-11-17): each other
module's namespace a module's tests reach is an import of its own,
`Name::imported`, on `js::ImportNames`, spelled apart by preparation after
the crate's imports, and read as that `Name`; only the runner's `test`,
a JS module's import, is given its word at output, against the names the
module reads and declares. `output.rs` reads no word.

JS's scoping is by spelling: its one owner reads words (2026-11-17, the
lead's decision). `js::scopes` models how JS resolves a name, by the word
it's printed as, which is what reclaim asks of a rename, `js::validate` of
a printed read and `alpha_eq` of two trees alike but for their names; its
reads of words are the spelling view, as preparation's and printing's
are, and the ratchet counts them with theirs. So that isn't a way around
it, an architecture test holds that what's asked of `js::scopes` by words,
its trees, renames and a local given apart from a module's read, is
asked only in preparation, printing and `js::validate`; lowering and
tidy ask it only `alpha_eq`, which asks no word. Tidy's drop flags are
asked of by their `Name`s, what each is known to hold, what's assigned
it and whether it's read or tested; the word a flag is named after, of
the variable whose drop it guards, stays its spelling. The ratchet: 263.

## Names are ids: kin by id, and a shadow by its Rust identifier (2026-11-17)

Whether a name is another's given apart, `x$1` of `x`, was asked of the
words' shapes, and tidy's shadow pass found the binding a `let` shadows by
the root of their words. Now a name records what it was made of, its
origin, apart from its identity (`Name::originated`, `given_apart`), and
`given_apart_of` answers by it; and a variable the program named records
the identifier its Rust gave it, rustc's `Symbol` as an opaque number
(`Of::Program(source)`, `Name::source`), from the pattern's binding or the
MIR local's debug info, so two variables of one identifier, a shadow and
what it shadows, are kin by it (`Name::same_source`), with no map of words.
What's still asked by parsing a word is counted and falls to none (the
architecture test "a name's kinship and form are asked by parsing its word
only as counted").

The output moved where the shadow pass found its binding. The root of the
words found the nearest name of that root, often one lowering made, which
isn't the program's, so the pass merged nothing; by the identifier it finds
the variable itself, and its own checks merge the two as one variable
reassigned, as the Rust shadows it: React's `reportGlobalError`,
`let message = error?.message; message = typeof message === 'string' ?
message : String(error);` for `const message$1 = ..; const message = ..`,
a `$1` gone, in 13 of the port's files and 7 of the corpus's cases, their
runs as native Rust's (the lead's decision). A shadow of another type
merges as one of its type does: React narrows a parameter in place,
`message = '' + (message || defaultMessage)`, which the pass on main already
wrote; a rule merging only shadows of one type moved 44 of the port's files
from upstream's form, and was dropped.

## What Rust leaves `_` is a form: `Name::is_unread` (2026-11-17)

An `async` closure's last parameters read nowhere were dropped where their
word was `_` or began `_$`: the program's `|_|`, which the coroutine's MIR
names `_`, and a value lowering names for what nothing reads,
`Role::Unread`'s `_`, given apart `_$1`, the first a declared name and the
second a temporary by its word's shape. Now each is made of the form
`Unread` (`js::Name::unread`), the program's by its Rust identifier,
`kw::Underscore`, and lowering's placeholders, `(_, i) => ..`, too:
`is_unread` answers it. Tidy takes one for a temporary, so a copy nothing
reads goes, but never names it for what it holds, `const _ = borrow[i]` of
`for _ in ..` (`unread-named-for-what-it-holds`); a range's unread count is
`i` by it. The output is as it was, the corpus's, the tests' and React's
port's, which a probe of the old answer against the new held over all of
them.

## rustc's `__arg0` is no name the program gives (2026-11-17)

An `async fn`'s parameter of a pattern, `Ctx { params, .. }: Ctx`, rustc
binds as `__arg0`, which the program never wrote; it was a name the
program gave, `Of::Program`, and taking a parameter apart where it's given
asked its word for `__arg`. Now `mir_locals` takes rustc's own, `__arg`
and its index, at the boundary where lowering reads rustc's identifiers, for
none the program names: a temporary, which a parameter of a pattern is in
a function that isn't `async`, so `destructured_params` asks `is_temporary`
alone. Taken apart where it's given as a function's is, one after another
parameter is too, past the copy the body holds of that one first, `const
n$1 = n;`: `async function total(n, [a, b])`, where it stayed `const a =
_2[0]` (`test/lowering-async.test.ts`). Its parts taken apart in its body
first, `const { params } = __arg0;`, no longer happen, and that branch went:
none of the corpus, the tests or React's port reached it.

## A holder's name is spelled in preparation, of its source (2026-11-17, step (a))

A variable lowering makes to hold another's value, `const a$1 = a;`, was
spelled where it was made, given apart from its source's word then, so its
spelling was fixed before the program's names were all known and was read
back, `a$1` of `a`, by passes that asked whether it was the source's. Now
it's made unspelled, `a#1`, as any name lowering makes for itself is, by
`js::Name::derived`, of its source by its id (`is_origin_of`), and of the
source's form: a temporary's holder is a temporary, a held argument's one.
`FnCx::unspelled_as` is the one way lowering makes an unspelled name, and
preparation's `reclaim::spelled` spells it of what its source is of, the
source found by its id, given apart from it as JS's scopes need. Preparation's
`shadows`, which folds a holder into its source and runs before spelling,
asks the origin too. The output moved only where a holder had taken a `$k`
of lowering's, two corpus files renumbered (`n$6$live` is `n$4$live`).

A parameter's shadow in a body applied where it's called, `const held$1 =
$unwrap(held);` (`js::applied_call`), is such a holder too, made by
`Name::derived_apart` of what the parameter reads: the variable it's given,
or the variable whose place a handle it's given reads, a public
thread-local's of `with_borrow_mut(|current| ..)`, which the parameter
becomes; else the parameter. Of one made of a name its word says, a
module's, which has no id, its origin says so (`Id::OF_WORD`) and it's of
the name its base word is, a module's name's word its identity
(`thread_local_shadow_of_its_handle`).

## Reclaiming is per variable (2026-11-17)

Preparation reclaimed a word, every declaration spelled so at once, in
declaration order: where one word spelled two variables, an arrow's
parameter and another's of its function, `item$2`, neither could be `item`
if one couldn't, and tidy's `unsuffixed_params` renamed an arrow's
parameters by word before preparation to make up for it. Now `reclaim::locals`
gives each variable spelled as another its own spelling first, by its id,
where the word is given apart (a plain word, `knownKeys` of two sibling
functions, or a word JS keeps, `arguments$`, is as plain as it gets), and
reclaims each by itself, an arrow's parameters first; a closure's parameter
lowering names as the variable it captures, one `Name` declared twice, is
bound again inside its closure, its holders with it (`js::rebound` carries a
holder's origin too). `unsuffixed_params` goes. So that the moved spelling
reads no less plainly:

- `shadows` folds a copy given apart by its word's root only into a
  closure's own parameter, `const p$13 = p$12;` of `|p|`, never a plain
  alias into a variable given apart, `const tail = tail$1;`;
- a word JS keeps given apart, `arguments$$1`, is reclaimed to its own
  spelling, `arguments$`, never the word, which no module may bind;
- an item's locals are reclaimed before the module's imports, and an
  import given apart takes the first free spelling below its own, its old
  word freed, `Card$1` and `Card$2` where they were `Card$2` and `Card$3`;
- an inner binding may shadow a name the module's scope declares, an
  `on_load!` body's `const value`, as JS's scopes resolve it, never where
  its scope reads the module's name of that word, which `js::validate`
  refuses (`ModuleShadowed`, a unit row in `prepare/reclaim.rs`);
- the scope tree and reclaiming walk a namespace's constants too.

The output moved where spellings got plainer: calculator's `result$1` is
`result` in its blocks, 20 suffixed lines to 4; interpreter 21 to 7; React's
ReactFlightServer 6 to 2, ReactFlightReplyClient 5 to 1; a few renumbered
alike; the lexer's loop is the stepped `while ((c = $next(..)) != null)`,
`c` its own; two tests' rows (the lead's approval).

## A read is of its own variable, by its id (2026-11-17, #2026)

Some reads carried another variable's id than the declaration they resolve
to, the words alike, so JS read the right one while a respelling by id
would have parted them: tidy's `copied bindings` named a variable rust-js
made for the binding the program named, `old` of `take()` as
`updatesForLane`, and left the binding's reads its own id; a `Drop`'s body
applied in place dropped a field's copy of the variable's own word, `const
prevProps = prevProps;`, its reads the copy's. Each now gives a read the
variable's id where it makes one of the other, and `copies_read_as`
substitutes a field's copies by their ids, which ended the hazard the guard
kept since per-variable reclaiming held (its exception, recorded then,
closed). `js::validate` refuses a read a declaration of another variable
spelled as it is takes nearer than its own (`Problem::Captured`), which
found these over the whole check, the metamorphic net and React's port; a
unit row holds it. Byte-identical.

## A drop's flag is a `js::Suffix` of its variable (2026-11-17)

A drop's flag, `noisy$live`, was spelled in lowering of its variable's word,
given apart where it was, `a$1$live`, and tidy named a temporary's again by
splitting words, `_14$live` into `value$live`. Now it's a name of its own
form, `Of::Suffixed(js::Suffix::Live)`, of a closed kind (the lead's
decision; a unit row names each kind by a `match`, and an architecture row
refuses a suffix written as text outside `js/name.rs`), made unspelled of
its variable's root and its part's keys, `pair$a$live#1`
(`Name::suffixed`), the `$` of a word JS keeps left out, `new$0$live`; one of
a temporary, or of what Rust leaves `_`, records no origin, as the
temporary may be gone by tidy's end, and tidy names it for the variable its
drop reads, by its `Name` (`suffixed_again`); preparation spells it as any
unspelled name. The flags read plainer, `a$live` for `a$1$live` and
`tryBorrowMut$0$live` for `tryBorrowMut$3$0$live`, 37 corpus files, 459
lines with a `$` and a number to 151, std and React's port as they were; a
variable and its shadow each flagged in one function are kept apart,
`a$live` and `a$live$1` (`shadow_and_its_variable_each_flagged`).

## A name given apart is of the first of its base, by its id (2026-11-17)

Whether a name was another given apart, `result$2` of `result`, `lo$1` of
`lo`, was asked of their words where neither said what it was made of.
Now `FnCx::fresh` keeps the first name it gives out of each base in the
item, and one it gives apart from that base records it as its origin
(`Name::given_apart_from`): `given_apart_of` asks the origin alone, a
holder's source, the first of its base, or what it was given apart from,
and a name of none is kin of none. Directional, as the words were: `lo$1`
is of `lo`, never `lo` of `lo$1`. Over the whole check, the metamorphic
net and React's port, its 3,450 answers were the words' answers, and the
output is as it was.

A held argument's name, `_arg`, is one by its base, `Base::HeldArg`, which
`spill` keeps, never by its word: `spill` made every base but a holder's a
name of its word, so a held argument's was told by its spelling,
`spelled_as_held_arg`, which goes. Preparation, the spelling's owner, joins
`js::name`, `names`, the runtime and printing in parsing a word, only to
spell or respell it; a question of kinship it asks goes through `Name`
(the architecture row counting the rest, at 2, the last of them tidy's).

## The module's spelling in lowering goes to preparation (2026-11-17, step (d))

A function written in an expression was named in itself apart from the
words its body reads, `function Count$1` in `const Count = memo(function
Count() { .. <Count /> .. })`, and a recursive function's lifted body took
its parameters apart from the module's names, each by its words, in
lowering. Now the first is a name of its word not spelled yet
(`Name::spelled_later`, its own in its item by its index there), and the
second's parameters `FnCx::unspelled`'s, which preparation spells apart
from what their scopes read, as JS's scopes resolve it; the aliases a
module's function values settle to are keyed by the function's module
name, and an alias's export is asked of its item's `Name` (`Name::is`).
Byte-identical, the corpus, the tests and React's port.

A key's entry's value, `keyValue` of `key` (`std_types/cell.rs`), was
spelled in lowering of the key's word and the role's; it's a `js::Suffix`
too now, `Suffix::Value`, joined as camelCase, `Value` its text after the
key's word (`suffix_tail`), made unspelled of the key by its id
(`Base::Suffixed(&Name, Suffix)` through `FnCx::fresh`), spelled by
preparation: a new kind, a deliberate one, which the unit row's `match`
names. Byte-identical.

## `word()` is a name's spelling alone; an exemption says why (2026-11-17)

`Role::word`, `Base::word` and `TypeFact::word` gave a role's or a fact's
text, `value`, `Size`, not a name's spelling, which `word` means here:
they're `text()` now (the lead's decision, a misnomer each), so what reads a
name's word is what the ratchet counts. A read of one that stays, a name JS
sees as it's written, a module's constant or a class's method, says so on
its line, `// word: <reason>`, the one form an exemption has, which the
ratchet's row skips; a word reserved in a naming table goes through
`Name::reserve_in`. The ratchet: 201 → 180.

Slice 8's (2026-11-17): a drop function known never to throw is one of a
set of `Name`s (`drops.rs`), an `Option`'s chain's parameter one by its id
(`optional_chain`), a map's call JS's own by the runtime's helper it
names (`Name::helper`), and a name made and taken away again gives its
word back to the naming table (`Name::release_in`, `reserve_in`'s
inverse). A module's static, kept in sets of its item's name, at a
module's scope its identity, is read so by one `item_word`, marked (`cell
/variables.rs`); a drop function by its item's name too
(`drops::inlined`). Byte-identical; the ratchet 176 → 165.
`drops::inlined`'s own questions of words follow as their own step.

## A guard inlined is asked of by ids (step (e), 2026-11-17)

`drops::inlined`, which writes a guard's `Drop` where it's dropped and
takes the guard apart into its fields' variables (ADRs 0589, 0705), asked
its questions of words until now, this ADR's recorded exception: the guard
was a word, the `Drop`'s binding, `let Self { previous, .. }`, made the
field's variable by their shared word, a variable whose word was declared
twice never kept, a field's own `const` named its key where no name of the
function's had that word, and a closure called where its guard's dropped
moved to its call where no declaration of a word it reads stood between,
each a second scope engine beside `js::scopes`.

Now the guard is a `Name`, read by `==` and `mentions_of`; the binding is
a copy of the field, its reads the variable's by id (`copies_read_as`); a
variable's kept where nothing writes it once the guard's made, counted by
its id; a field's own `const`, and the guard's own once read, `restore` of
`_restore`, are names spelled later, `Name::spelled_later`, of a `k` none
of the function's names has (`Name::free_made_k`), spelled by preparation,
`previous$1` where `previous` isn't free (the lead's decision: a field
taken apart as `key$1` reads better than the object left whole, and
`restore$1` is spelled as every other is). The `_` comes off by the one
rule a parameter's does, `parameter_names::unmarked`, which now also
refuses what's left where it's no JS name, `_9`, a parameter's too; a
global's name, `_window` of a guard, is spelled apart from the global's
reads in its scope by preparation, `escape$1` of `_escape` beside a call
of `escape`, as the copy kept `_escape` by asking the words its function
read.

What scoping it asks is `js::scopes`', by ids, no word: whether each
variable a closure reads is in reach where it's called (`reach`), and
whether each kept variable's reads, put where the field's are, resolve to
it (`not_read_as_themselves`). The second is what a variable's word declared
twice was a stand-in for: lowering names two of the program's variables of
one word apart only along a scope chain, which preparation relies on, so a
field's read put beside the `Drop`'s own `let previous` would read that one;
such a variable isn't kept, its field a `const` of its own. The guard's
word is read once, to take its `_` off, marked (`// word:`).

A `&mut` handle's setter's parameter, chosen in preparation before names
are spelled, counts a name spelled later of base `value` as `value`, which
it's spelled where free, so it's `newValue` beside the place's `value`
still.

Output: guards whose fields share a word with a variable in the function
are taken apart, `let prevHandler$1 = prevHandler;` for React's Flight
client's `_restore = { prevHandler, prevChunk }`, the guard a plain
variable, `const n` of `const _n`; React's `ReactChildFiber`'s restore
reads `prevDebugInfo` as upstream does. Native and JS agree on every
corpus case, with rows for a binding sharing its word with variables in
scope, a variable written after the guard's made, a panic between, and
guards named as globals.

Slice 9's (2026-11-17): what a function was given, the dictionaries and
values its top can read, is a set of `Name`s, not of words
(`FnCx::given_names`, `given_dictionaries`), asked of each read by its id
(`reads_only_at_top`, `entries_to_declare`), a JS global's by its form,
`Name::is_global`, not by a list of words; a map of a variable's items is
of the variable (`iterators::mapped_var`); `enumerate()`'s pairing is
`(x, i) => [i, x]` by its parameters' ids, not their words; an array a
runtime helper made is the helper's by `Name::helper`; `Iterator::fuse`'s
import, its word as its `use` names it (rd650), is read so, marked. Each
old and new answer probed over the corpus, the tests and React's port,
none differing; byte-identical.

And `library.rs`'s: the names a library's modules export, what its
consumers import, are read by one marked `exported_as`, an item's export
name its word at a module's scope.

## Preparation's shadows by id, their scoping asked of `js::scopes` (2026-11-17)

`prepare::shadows`, which makes `const n$1 = n;` `n` (ADR 0277), asked of
words: its variables, the function's own and a closure's given, sets of
words; the copy and what it's of, words; and three checks stood in for
one question, whether `n`'s reads, put where the copy's were, are `n`'s
still: a later declaration of `n`'s word (#1078), a closure's parameter
of it, and `js::outside_shadows`, which set a closure or a function that
declares its own of the copy's word apart (#457, #1766). It runs before
names are spelled. Now each is a `Name`: the fold makes each read of the
copy, by its id, a read of `n`, so no closure's own of its word is ever
renamed; the order and write questions ask by id (`mentions_of`,
`js::Writes`); and the one scoping question is `js::scopes`':
`not_read_as_themselves` of the fold made on a copy, which keeps the copy
where any read would be another variable's of that spelling. That isn't
spare: React's Flight server's `decodeReplyFromBusboy` copies its stream,
`const busboyStream$1 = busboyStream`, and the function `flush` declared
beside it reads the copy into its own, `const busboyStream =
busboyStream$1`, which the fold would make read itself. `js::validate`
refuses such a read at every build (`output.rs`, after preparation;
`validate.rs`, `Problem::Captured`), but only of names with ids of their
own: a copy of a variable made of its word alone, which neither tells
from another of its spelling, isn't folded. `outside_shadows`, with no
caller left, goes; `written_back`'s fold, whose copy no closure reads
(`read_later`), asks nothing more. Byte-identical.

## Every local the compiler makes is told apart (#2034, 2026-11-17)

`js::validate`'s capture check (`Problem::Captured`) asked only names with
ids of their own, so a local made of its word alone, which `From<&str>`
for `Name` made, and a body's item (`Name::item`), had no check that a
read of it isn't another's of its spelling: a wrong answer where a pass
put one there. The compiler makes neither now: the eleven places that made
a name of a word name the variable they are, a pattern's bound names, a
held value unspelled, a chain's variable by its `Name`, a parameter
respelled keeping its id, an arrow's parameters declared; and the check
asks `Name::is_identified`, an id or a body item's item and copy, so it
covers every local. `From<&str>`, `From<String>` and `From<&String>` for
`Name`, and `Pattern`'s, are a test-only fixture (`#[cfg(test)]`): some
270 unit tests build their trees of words, a shape the compiler can't
make; they go when those tests make identified names.

Slice 5's (2026-11-17), tidy's jumps: a label, a block's or a loop's, is
a `Name` where a pass keeps it, the labels a loop holds, the blocks a
`switch` leaves by, where each block's turn rests, the blocks a `return`
was joined past, and a jump's is asked of them by `==`, which of a
label, made of its word alone, is its word and its form; a variable a
pass asks of, a `Result` held (`js::result::held_of`, of which
`reads_held` asks by word for preparation), a copy read in place
(`substitute_names`), what a block declares and what reads it after
(`declared_names`, `mentions_of`), by its id. Each old and new answer
probed over the corpus, the tests and React's port, none differing; a
tidy-made label's spelling, `loop$k`, and the order a loop's given
variables are declared in, by spelling, are read so, marked.
Byte-identical.

