# 1127. A fact an expression carries has a role, and the JS is right without each that only permits

Status: Accepted (2026-11-12, #1460, of the architecture review of that
day). Extends [1087](1087-each-readability-rewrite-is-checked-on-and-off.md),
whose oracle leaves out each rewrite, to the facts the rewrites read; and
[0889](0889-an-expressions-facts-are-a-set.md)'s set of facts with
what each is for.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Nothing a program
observes changes: three flags a build never passes, and tests. A build's
JS is the same byte for byte.

## Context

`js::Known` had 60 facts, each a bit an expression carries past the
lowering that knew it, read by tidy, preparation and the printer. Nothing
said which of them a reader may lose. Some only let a reader write a
plainer form (`Lasting`, a move; `NeverNaN`, a negated test); some warn it
off (`Getter`: a read that may run anything); some say what the JS *is*,
which its reader takes apart by (`IterLent`: `$lent(it)`). A rewrite that
rebuilds an expression and drops its facts is safe of the first kind and a
wrong answer of the others, and the type, one enum, told none apart. ADR
1087's oracle tries each rewrite off; it never tried a rewrite without
what it reads.

The experiment (2026-11-12): each of the corpus's 2,253 cases that run
compiled forgetting each fact a reader found in it, 8,172 compiles, each
JS that changed run against native Rust.

- **Every fact guessed to permit, or to say what was written, ran as
  native Rust does**, but two. `Iterator` didn't (`by_ref_steps`,
  `step_by_of_a_call_holds_its_items`): what isn't known to be a JS
  iterator is taken for an array, not for either, so it's a form. And
  `NeverThrows` crashed 66 compiles, none wrongly: tidy's contract "what
  unwinds is guarded" reads it to accept a `finally` the lowering wrote of
  what can't throw, so the check, not the JS, needs it.
- **Of those guessed a form or a warning, ten changed what some program
  does**: `InPlace` in 130 cases, `IterStepped` 50, `IterClosing` 15,
  `IterLent` 10, `LocalRead` 2, and `Getter`, `Live`, `HandledObject`,
  `IterOwn` and `IterRange` one each. Seven more forms were refused where
  forgotten, `IterKnowsBack` in 56 cases. `ClosesWhatItSteps`,
  `IterRepeatingN`, `Binding` and `Evidence` changed nothing the corpus
  shows; they keep their role by what their readers do without them: an
  iterator closed and dropped again, a binding's or a dictionary's method
  taken for JS's own of the same name, which only reads.
- Six are found by no corpus case, a binding's all: `Setter`,
  `RunsNothing`, `OwnMethodCall`, `KeyedMethod`, `NamespaceMethod`,
  `NullStored`.

Run again with the roles below, the 37 facts the net forgets over the
same cases: 5,680 compiles, 1,332 of them another JS, each running as
native Rust does; none refused, none crashed.

## Decision

**Each fact has a role, one of four, said by `Known::role`, a `match` of
every fact. A compile can forget facts, `--without-known`; and every
program compiled without a fact that permits, or says what was written,
runs as native Rust does.**

| Role | What it says | Unknown, its reader | Forgotten by the net |
|---|---|---|---|
| `Permits` | A plainer form is right here | writes the form that's right of any value | yes |
| `AsWritten` | How the program wrote it | makes its rewrite; the JS does the same | yes |
| `Warns` | Evaluating it may do what its kind doesn't say | takes it for a plain read or write | never |
| `Form` | What the JS is, a form the lowering chose | reads another form's JS | never |

- **`Known` is one list**, `knowns!` in `src/js/known.rs`: the enum and
  `Known::ALL` of the same lines, so none is left out. `rust-js --knowns`
  lists each with its role.
- **Forgetting is `Facts`' alone.** `--without-known <fact>`, any number,
  sets what the compile forgets before anything is lowered
  (`js::forget_known`). Every read of a `Facts`, `has`, its parts
  (`of_presence`, `of_evaluation`, `of_statement`) and `==`, is of what
  isn't forgotten (`known::seen`), so a fact forgotten is one never said,
  wherever it was said. No reader names the set. `--knowns-found <file>`
  writes which facts a reader found, so a variant is made only of a fact
  whose forgetting can change the JS, as `--prepare-fired` does of a
  rewrite.
- **A fact's test is `has`, not a comparison with a set.** `f ==
  Facts::of(NoneUndefined)` is true of a `NoneUndefined` forgotten, both
  sides nothing: `Facts::only_none` answers which one `None` an `Option`
  has, and `strict_presence`, `presence`, `same_none` and `null_apart` ask
  it.
- **A contract's check reads every fact** (`js::remembering`, in tidy's
  `run_stage`): forgetting tries the passes, not what checks them.
- **The net is ADR 1087's**, `test/tidy-stages.test.ts`: beside a
  program's variants of tidy's stages and preparation's rewrites, one
  `without knowing <fact>` for each fact found in it whose role is
  `Permits` or `AsWritten`. The suite's own run takes its few cases, the
  nightly the corpus and the generated programs. A variant that's
  refused, crashes or does otherwise names the fact: a reader that can't
  do without it.
- **A fact found to be needed is a form**, and moved to `Form` with the
  case that found it, as `Iterator` was. `Form` and `Warns` are what a
  typed IR carries as a type and an effect (#1459): the list of them is
  that issue's inventory.

## Consequences

- 37 facts are forgotten in turn by the net: 32 that permit, 5 that say
  what was written. 23 are not: 17 forms, 6 warnings.
- A new fact is given its role where it's declared, or `role` doesn't
  compile; one given `Permits` is tried without by the next run.
- A rewrite that drops a `Permits` fact can't make a wrong answer the net
  doesn't already try, on the programs it has. A `Form` or a `Warns`
  dropped still can: nothing here guards those.
- The six facts no corpus case finds are tried by no variant yet: the
  read cases (`test/getter-reads.test.ts`) are bindings' and could take
  the same variants.
- Mutations: `fact-forgotten-still-found` (`Facts::has` reads what's
  forgotten), `nothing-forgotten` (`forget` forgets nothing),
  `found-what-is-forgotten` (a fact forgotten is said found),
  `iterator-fact-only-permits` (`Iterator` back to `Permits`, caught by
  `by_ref_steps`), `contracts-forget-too` (a contract checked of what's
  forgotten, caught by a `finally` of what never throws),
  `without-known-any-name` and `knowns-found-unwritten`.
