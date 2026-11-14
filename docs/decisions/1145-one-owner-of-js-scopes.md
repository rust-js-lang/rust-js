# 1145. One owner of JS's scopes; trees compared but for their names

Status: Accepted (2026-11-14, #1681's change 3, M2's first step; the
lead's approval of the plan: one resolver serving the validator,
`alpha_eq` and later M3's `VarId`s).

Case ([0262](0262-when-rust-and-js-disagree.md)): none; the JS does
what it did.

## Context

Change 3's M2 makes each name a function declares its own, unique in its
item, as the step before M3 makes names `VarId`s. Measured where tidy
starts, 3,082 of the corpus's 48,450 bodies declare a name twice, each of
them a name lowering writes as it is, a helper's `(iterator, init, f,
unwind, ends) =>` or a `catch (error)`; MIR's own locals are each the
item's already, a closure's too, which shares its item's names.

Making the first family unique showed two things:

- **Two owners of one rule.** JS's scoping was resolved twice: by
  `prepare/scopes.rs`, which says whether reclaim's rename keeps each read
  its own (ADR 0357), and by `js::validate`'s own walk (ADR 1143), which
  said whether each read resolves. Two walks of the same rules, which a
  change to one leaves the other behind.
- **Trees compared by their names.** A dictionary written twice is
  declared once (ADR 0979), found by `==`: two of the same helper whose
  closures' parameters are named apart are no longer the same, and the
  dictionary is written out twice. A name made unique can only make `==`
  say "different" where it said "the same", never the other way, so no
  comparison becomes wrong, but each that compares closures reads worse.

## Decision

**`js/scopes.rs` is the one owner of JS's scoping**: each scope's
declarations, and each read, where it is, the declaration it's of or a
name from outside, and whether it's a global's (`Known::Global`, or a
name the printer writes, `String(x)`'s). Reclaim asks it whether a rename
keeps each read (`Tree::of`, `renames`); `js::validate` asks it each
read's answer and each name a scope declares twice (`Tree::of_module`,
`Tree::of_body`), keeping only the check of jumps, which isn't scoping.

**`js::scopes::alpha_eq(a, b)`**: the same but for what declarations are
named. Alike with every name left out, and each read, in the walk's
order, of the declaration the same number into each, or of the same name
from outside. So `(x) => x` is `(y) => y`, but `[(x) => x, x]` isn't
`[(y) => y, y]`: their last reads are of different names from outside,
which a rename of names alone would take for one, a wrong answer once one
replaced the other. Leaving a name in that it doesn't know as a
declaration's only makes trees unlike: it never takes two for one that
aren't.

Dictionaries are compared by it: `declare_repeated`'s count and
replacement, `entries_to_declare`'s, and `keep_dictionary`'s, each a
dictionary's closures that a family made unique would otherwise tell
apart.

## Consequences

- 13 corpus cases read shorter: a dictionary written two or more times
  whose closures' parameters were named apart, `(x) =>` and `(x$1) =>`,
  is declared once, as ADR 0979 says of one written twice,
  `const rangeFullHash = { hash(value, state, H) {}, .. }` read by its
  name where it was written out three times. Each still does what native
  Rust does; the port and the guide's fences are what they were. Its unit
  tests: the counterexample, shadowing, a capture of an outer declaration
  and of a name from outside; the moved resolver's own.
- Mutations: `js/scopes.ts`'s, moved with it, `alpha-eq-without-reads`
  and `alpha-eq-reads-of-any-declaration`; `js/validate.ts`'s retargeted
  to the resolver.
- Of the 30 comparisons of expressions by `==` outside `js/`, found by
  taking `PartialEq` off `Expr`: 9 tell whether a pass changed its tree
  (`passes::run`), which `==` asks rightly; 17 compare against a tree no
  declaration is in, `Expr::undefined()`, a variable, a place; and the 4
  of dictionaries are `alpha_eq`'s.

## Amendment: the first family, the compiled folds' parameters (2026-11-14)

A std struct's compiled `fold` and `try_fold`, given as its steps' `rfold`
and `tryRfold` (`compiled_steps.rs`), name their parameters, `(iterator,
init, f, broke, unwind, ends) =>`, of the item's names (`fresh`), each
its own, where each was written as it is: 214 of the bodies that declared
a name twice where tidy starts. Spelled as preparation spells them
(ADR 0352), the corpus is what it was but one case, each dictionary of
them alike by `alpha_eq`; `held_enumerate.rs` reads shorter, an outer
`f` no longer named apart from a parameter of its name, `const f$1 = f;`
gone.

## Amendment: the second family, a catch's parameter (2026-11-14)

What lowering makes to catch a panic as it unwinds, `try { .. } catch
(error) { ..; throw error; }`, `dropped_after`'s, `dropped_unwinding`'s,
`each_dropped`'s and the `$cannotUnwind` of a function that can't unwind,
names its parameter of the item's names, `FnCx::catch_names`, each its own,
where each was `error` as it is. The corpus is what it was, byte for byte.
Tidy's own catches, `aborting_finally`'s and a poisoned lock's, are made
after M2's names are, where tidy starts; M3's `VarId`s name them.

## Amendment: the third family, the parameters of arrows lowering makes (2026-11-14)

An arrow lowering makes around an item, a copy's, a clone's, a drop's,
a `Debug`'s or `Any`'s, and the arrows of `next_if_eq(&item)` and of a
`for_each(x)` that hold the program's values, names its parameter of the
item's names, `FnCx::fresh`'s, each its own, where each was written as it
is, `item`, `a`, `b`, `each`, `x`. The item's names hold every name such a
value can read, a global it binds too, so `Expr::param_apart`, which gave
the parameter apart from those, is gone (ADR 0352, amended).
Byte for byte, the corpus is what it was but four cases, each still
native's. In each, one of tidy's own rules now applies where a
parameter written `b`, or `item`, kept a program binding of that
spelling from it, as it applies wherever nothing collides:
`borrow_mut_of_a_value_as_itself` reuses its `let b` for the first `b`
too, as it already did for the later ones; `arrow_parameters_named_apart`
reuses `let each$1`; `ref_mut_bindings_copied` writes `nested[0][0] =
5`; and `user_deref` writes its `println!`'s arguments in its template,
in their order. Keeping the old bytes would mean keeping a spelling
shared on purpose, the name-as-identity M2 removes.

The program's own names were not reserved first. Reserving them gave
none of the four back, and gave a temporary that had a name free in its
own scope, `d`, `l` or `result`, `tmp$1` or `value` instead.

Two compared expressions became alike but for their parameters' names:
`vec![p; n]` gives `$repeatItems` a hand-written clone only where it
isn't the copy, and the two are compared with `alpha_eq`
(`slice.rs`), where `!=` gave the copy twice.

`clone_fn` and `key_parts_fn` still name their parameter for their own
scope alone. Given the item's names, they are named before the program's
locals and take the program's own `key` and `value`, `key$1` for the
program's. They are the next family: compiler-made names of the start of
M3's names table, spelled once by preparation, and refused by the
validator if one reaches printing.

## Amendment: the fourth family, names spelled once the program's are made (2026-11-14)

`clone_fn`'s and `key_parts_fn`'s parameter, `(value) => ..` of a clone
and `(item) => ..` of a key's parts, is a name lowering makes before
every name the program's is made: a closure's clone is made before the
closure's own `key` is named. Given of the item's names as it's made, it
took the program's `key`, and the program's was `key$1`. So it is made
not spelled yet, the start of M3's names table for the names the
compiler makes:

- `names::unspelled(base, k)` writes one, `value#1`, which no JS reads and
  `names::apart` doesn't take apart, so tidy takes it for no kin of a
  program's `value`; `names::unspelled_base` reads it.
- `FnCx::unspelled(base)` alone makes one, unique in its item's names.
- Preparation spells each once, `reclaim::spelled`, before reclaiming
  and outside the passes `--prepare-without` leaves out: its word, or
  given apart from it, the first that JS's scopes let it be, as `locals`
  reclaims (ADR 0357), the program's names all made by then keeping
  theirs. Of an item whose scopes declare a name twice, which has no tree
  to ask, apart from every name its module has.
- The validator refuses one that reaches printing, `Problem::Unspelled`.

An architecture test holds that `FnCx::unspelled` alone makes one and the
validator and the spelling alone read one. `clone_fn` keeps its body's
names apart from its parameter's word, so the parameter is spelled
`value` where it can be, `const value$1 = $someValue(value)` (#1246).

The corpus is what it was but two cases, each still native's, in each
of which two of the compiler's temporaries trade `value` and `value$1`:
a dictionary declared at `main`'s top, `{ clone: (value) => value }`,
whose parameter was written `value` as it was made, took `value` from
every later temporary of the item, and doesn't now. Mutations
`unspelled-never-known`, `unspelled-printed`, `unspelled-left-unspelled`,
`unspelled-spelled-over-a-read` and `unspelled-its-word-without-a-tree`
are caught; `clone-fn-parameter-free-to-its-body` now changes a snapshot,
the parameter spelled `value$1`, where the module didn't parse.
