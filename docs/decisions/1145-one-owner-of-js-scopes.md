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
