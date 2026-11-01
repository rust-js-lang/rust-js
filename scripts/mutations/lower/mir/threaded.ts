// Mutations of src/lower/mir/threaded.rs (ADR 0093): an enum one `match`
// makes and the next takes apart at once, each arm going on to its case
// (rd433, ADR 1055).
import type { Mutation } from "../../../mutations";

const corpus = ["test/corpus.test.ts", "-t", "match_of_what_a_match_made"];

export const mutations: Mutation[] = [
  {
    name: "threaded-matches-made-whole",
    breaks: "an enum one `match` makes and the next takes apart is an object of a tag, `settled = { TAG: \"Ok\", _0: value }`, tested again (rd433)",
    file: "src/lower/mir/threaded.rs",
    find: "            let Some(threading) = self.threading(current) else {\n",
    replace: "            let Some(threading) = self.threading(current).filter(|_| false) else {\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "threaded-case-of-another-variant",
    breaks: "an arm goes on to the switch's `otherwise`, not its variant's case: code a program that's right never reaches, which throws",
    file: "src/lower/mir/threaded.rs",
    find: "                case: targets.target_for_value(value),\n",
    replace: "                case: targets.otherwise(),\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "threaded-read-whole",
    breaks: "an enum read whole too, `kept`'s `settled` given back, is threaded, and what's given back was never made",
    file: "src/lower/mir/threaded.rs",
    find: "        if other || tested_reads != 2 || writes.is_empty() {\n",
    replace: "        if tested_reads != 2 || writes.is_empty() {\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "threaded-second-field-lost",
    breaks: "a variant of two fields, `Pair::Both(a, b)`, is threaded, and only its first is written",
    file: "src/lower/mir/threaded.rs",
    find: "            if operands.len() > 1 {\n",
    replace: "            if false {\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "threaded-drops-dropped",
    breaks: "an enum that holds what drops, `Ok(Loud(..))`, is threaded, its drop gone with it: `dropped zero` never printed",
    file: "src/lower/mir/threaded.rs",
    find: "        if !adt.is_enum() || self.drops(ty) != Drops::Nothing {\n",
    replace: "        if !adt.is_enum() {\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "threaded-past-code",
    breaks: "what runs between the two `match`es, `let small = count < 10`, is copied to each arm",
    file: "src/lower/mir/threaded.rs",
    find: "        if !(data.statements.iter().enumerate()).all(|(i, s)| i == read_at || quiet(s)) {\n",
    replace: "        if false {\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "threaded-parts-apart",
    breaks: "a payload's parts bound apart, `Some((text, count))`, are threaded, each read of its own local, not `const [text, count] = first`",
    file: "src/lower/mir/threaded.rs",
    find: "] if !matches!(rest.first(), Some(PlaceElem::Field(..))) => {\n",
    replace: "] if true => {\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "threaded-comments-lost",
    breaks: "the comments before the second `match`, `// A callback may read the chunk again`, are lost with it",
    file: "src/lower/mir/threaded.rs",
    find: "            cases.insert(ended, written);\n",
    replace: "",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "threaded-binding-apart",
    breaks: "a field read once by the variable its case binds, `Err(reason)`, is read into a local of its own, `let unknown; .. const reason = unknown;`",
    file: "src/lower/mir/threaded.rs",
    find: "            let local = match self.taken_by.get(&key) {\n",
    replace: "            let local = match None::<&(Local, mir::Location)> {\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "threaded-cases-labeled",
    breaks: "`Ok` and `Err` each made twice, whose cases go on to what follows, are threaded, each case's way out of the first `match` a labeled block's `break`",
    file: "src/lower/mir/threaded.rs",
    find: "        if makers.values().any(|&n| n > 1) && !ends_apart(body, &makers.keys().copied().collect::<Vec<_>>()) {\n",
    replace: "        if false {\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "threaded-unboxed-option-of-no-borrow",
    breaks:
      "an `Option` an `if` chain makes and a `let`-`else` takes, with no borrow between, is threaded: ReactART's `instance` renamed `instance$1` and its `if (!instance) throw` gone (rd531)",
    file: "src/lower/mir/threaded.rs",
    find: "        if unboxed && !ends_a_borrow() && !in_a_loop() {\n",
    replace: "        if false && unboxed && !ends_a_borrow() && !in_a_loop() {\n",
    tests: ["test/corpus.test.ts", "-t", "^match_of_what_a_match_made.rs$"],
    snapshots: true,
  },
  {
    name: "threaded-unboxed-option-past-a-borrow-kept",
    breaks:
      "an `Option` a `match` of a borrowed state makes is never threaded, `const done = .. ? state._0 : undefined; if (done !== undefined)` where the arm's case is one dispatch (rd531)",
    file: "src/lower/mir/threaded.rs",
    find: "                    if self.recognition().is_guard(place.ty(&body.local_decls, tcx).ty))\n",
    replace: "                    if self.recognition().is_guard(place.ty(&body.local_decls, tcx).ty) && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^match_of_what_a_match_made.rs$"],
    snapshots: true,
  },
  {
    name: "threaded-loop-value-kept",
    breaks:
      "an `Option` a `loop` breaks with, `let next = loop { .. break Some(next) .. }`, with no borrow between, is left a variable its breaks write, `next = undefined; .. if (next !== undefined)`, where its case went on from each break (rd531)",
    file: "src/lower/mir/threaded.rs",
    find: "        if unboxed && !ends_a_borrow() && !in_a_loop() {\n",
    replace: "        if unboxed && !ends_a_borrow() {\n",
    tests: ["test/corpus.test.ts", "-t", "^loop_value_given_in_its_test.rs$"],
    snapshots: true,
  },
];
