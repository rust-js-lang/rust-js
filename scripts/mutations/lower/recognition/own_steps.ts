// Mutations of src/lower/recognition/own_steps.rs (#182, #505, #521).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lasts-its-own-not-through-skip",
    breaks:
      "`skip(n).last()` of the crate's iterator with its own `last` folds, not `nth(n - 1)` then its `last`",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Last, \"Skip\" | \"Fuse\" | \"Peekable\" | \"Copied\" | \"Cloned\") => own,\n",
    replace: "            (Last, \"Fuse\" | \"Peekable\" | \"Copied\" | \"Cloned\") => own,\n",
    tests: ["test/corpus.test.ts", "-t", "^own_last_and_nth_passed_on.rs$"],
  },
  {
    name: "lasts-its-own-not-through-chain",
    breaks:
      "a chain's `last` of the crate's iterators with their own `last` folds, not each side's `last`",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Last, \"Chain\") => &[(0, Last), (1, Last)],\n",
    replace: "            (Last, \"Chain\") => &[],\n",
    tests: ["test/corpus.test.ts", "-t", "^own_last_and_nth_passed_on.rs$"],
  },
  {
    name: "lasts-its-own-not-generic",
    breaks: "generic code's `last()` steps its iterator to its end, never asking the type's own `last` (#496)",
    file: "src/lower/recognition/own_steps.rs",
    find: "        matches!(self, OwnStep::Nth | OwnStep::Last | OwnStep::Count)\n",
    replace: "        matches!(self, OwnStep::Nth | OwnStep::Count)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_own_last.rs$"],
  },
  {
    name: "counts-its-own-not-through-adapters",
    breaks: "std's compiled `Peekable` of the crate's iterator with its own `count` is given a dictionary with none, so its `count` steps it (#505)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Count, \"Skip\" | \"Fuse\" | \"Peekable\" | \"Enumerate\" | \"Copied\" | \"Cloned\") => own,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_count_through_adapters.rs$"],
  },
  {
    name: "may-fold-its-own-not-generic",
    breaks:
      "generic code's `collect()` into a `String`, a `HashSet` or a `HashMap` steps its iterator by `next`, never asking the type's own `fold` (#521)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            return matches!(ty.kind(), ty::Param(_)) && (reach == Reach::May || step.asked_at_run_time());\n",
    replace: "            return matches!(ty.kind(), ty::Param(_)) && step.asked_at_run_time();\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_own_fold.rs$"],
  },
  {
    name: "may-fold-its-own-not-through-adapters",
    breaks:
      "generic code's `map(f).collect::<String>()` steps its iterator by `next`, never asking the type's own `fold` through the adapter (#521)",
    file: "src/lower/recognition/own_steps.rs",
    find: "self.reaches_its_own(source, asked, reach))",
    replace: "self.reaches_its_own(source, asked, Reach::Sure))",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_own_fold.rs$"],
  },
  {
    name: "folds-its-own-not-through-chain",
    breaks:
      "`last()` of a `chain` of the crate's iterator steps it by `next`, not by each side's own `fold`",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Fold, \"Chain\") => &[(0, Fold), (1, Fold)],\n",
    replace: "            (Fold, \"Chain\") => &[],\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "folds-its-own-never",
    breaks:
      "`collect` into a string or a hash map, and `last()`, of the crate's iterator step it by `next`, not by its own `fold`",
    file: "src/lower/recognition/own_steps.rs",
    find: "            return own.is_some();\n",
    replace: "            return own.is_some() && step != OwnStep::Fold;\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "nths-its-own-never",
    breaks: "nth of an adapter reaches the crate's own nth",
    file: "src/lower/recognition/own_steps.rs",
    find: "            return own.is_some();\n",
    replace: "            return own.is_some() && step != OwnStep::Nth;\n",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
  {
    name: "nths-its-own-no-take",
    breaks: "take's nth is its source's",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Nth, \"Skip\" | \"Take\" | \"StepBy\" | \"Enumerate\" | \"Fuse\" | \"Peekable\") => own,\n",
    replace: "            (Nth, \"Skip\" | \"StepBy\" | \"Enumerate\" | \"Fuse\" | \"Peekable\") => own,\n",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
  {
    name: "nths-its-own-no-mut-ref",
    breaks: "&mut I's nth is I's",
    file: "src/lower/recognition/own_steps.rs",
    find: "            return step.through_a_mut_ref() && self.reaches_its_own(*inner, step, reach);\n",
    replace: "            return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
  {
    name: "folds-by-own-nth-never",
    breaks: "a skip's fold asks its source's nth",
    file: "src/lower/recognition/own_steps.rs",
    find: "        if matches!(adapter, \"Skip\" | \"StepBy\") {\n",
    replace: "        if matches!(adapter, \"Never\") {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
  {
    name: "folds-by-own-nth-not-through-map",
    breaks: "a map's fold is its source's",
    file: "src/lower/recognition/own_steps.rs",
    find: "                sources.extend(self.nth_folded_sources(source));\n",
    replace: "                let _ = source;\n",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
  {
    name: "try-rfolds-not-through-fuse",
    breaks:
      "a `fuse()`'s `try_rfold` isn't said to be its source's, so a `take(3).fuse().skip(1).rev()` collected into a `String` steps by `next`, dropping 4 before 5 (#697)",
    file: "src/lower/recognition/own_steps.rs",
    find: '            (TryRfold, "Map" | "Filter" | "Enumerate" | "Skip" | "Fuse" | "Peekable") => own,\n',
    replace: '            (TryRfold, "Map" | "Filter" | "Enumerate" | "Skip" | "Peekable") => own,\n',
    tests: ["test/iterator-forms.test.ts", "-t", "^a take's rev, consumed by fold or by next"],
  },
  {
    name: "try-rfolds-not-through-peekable",
    breaks:
      "a `peekable()`'s `try_rfold` isn't said to be its source's, so a `take(3).peekable().skip(1).rev()` collected into a `String` steps by `next`, dropping 4 before 5 (#697)",
    file: "src/lower/recognition/own_steps.rs",
    find: '            (TryRfold, "Map" | "Filter" | "Enumerate" | "Skip" | "Fuse" | "Peekable") => own,\n',
    replace: '            (TryRfold, "Map" | "Filter" | "Enumerate" | "Skip" | "Fuse") => own,\n',
    tests: ["test/iterator-forms.test.ts", "-t", "^a take's rev, consumed by fold or by next"],
  },
  {
    name: "rfolds-not-through-peekable",
    breaks:
      "a `peekable()`'s `rfold` isn't said to be its source's, so generic code's `src.take(3).peekable().rev()` collected into a `Vec` steps by `next`, dropping 4 before 5 (#697)",
    file: "src/lower/recognition/own_steps.rs",
    find: '            (Rfold, "Map" | "Filter" | "Enumerate" | "Fuse" | "Peekable" | "FlatMap" | "Flatten") => own,\n',
    replace: '            (Rfold, "Map" | "Filter" | "Enumerate" | "Fuse" | "FlatMap" | "Flatten") => own,\n',
    tests: ["test/iterator-forms.test.ts", "-t", "^a take's rev, consumed by fold or by next"],
  },
  {
    name: "nths-its-own-no-peekable",
    breaks:
      "a `peekable()`'s `nth` isn't said to be its source's, so a `step_by` of a `Peekable` of a reversed `Vec` is a `filter`, which drops each item it passes, back to front, not as `nth_back`'s one slice (#261)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Nth, \"Skip\" | \"Take\" | \"StepBy\" | \"Enumerate\" | \"Fuse\" | \"Peekable\") => own,\n",
    replace: "            (Nth, \"Skip\" | \"Take\" | \"StepBy\" | \"Enumerate\" | \"Fuse\") => own,\n",
    tests: ["test/corpus.test.ts", "-t", "^step_by_compiled_source_nth.rs$"],
  },
  {
    name: "chain-nths-no-slice",
    breaks:
      "`rev().chain(..).step_by(2)` of a `Vec` is a filter dropping what it steps over from the back one at a time, not `$stepBy` asking the chain's `nth`, its first side's `advance_by`, one slice (#261)",
    file: "src/lower/recognition/own_steps.rs",
    find: '        (std_path(self.tcx, adt.did()) == "std::iter::Chain").then(|| args.types().next())?\n',
    replace: '        (std_path(self.tcx, adt.did()) == "std::iter::Never").then(|| args.types().next())?\n',
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
  {
    name: "nths-its-own-no-enumerate",
    breaks: "enumerate's nth is its source's",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Nth, \"Skip\" | \"Take\" | \"StepBy\" | \"Enumerate\" | \"Fuse\" | \"Peekable\") => own,\n",
    replace: "            (Nth, \"Skip\" | \"Take\" | \"StepBy\" | \"Fuse\" | \"Peekable\") => own,\n",
    tests: ["test/corpus.test.ts", "-t", "^(own_nth_through_adapters|step_by_compiled_source_nth).rs$"],
  },
  {
    name: "nths-its-own-no-fuse",
    breaks:
      "`fuse().nth(k)` of the crate's iterator with its own `nth` is `.drop(k).take(1)`, not `$nth(it, k)`",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Nth, \"Skip\" | \"Take\" | \"StepBy\" | \"Enumerate\" | \"Fuse\" | \"Peekable\") => own,\n",
    replace: "            (Nth, \"Skip\" | \"Take\" | \"StepBy\" | \"Enumerate\" | \"Peekable\") => own,\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^(own_last_and_nth_passed_on|step_by_compiled_source_nth).rs$"],
  },
  {
    name: "take-folds-unseen",
    breaks:
      "`last()` of a `Take` isn't by its fold, so a `Skip` inside asks the crate's own `nth(n)`, not `nth(n - 1)` (#213)",
    file: "src/lower/recognition/own_steps.rs",
    find: "                \"Map\" | \"Filter\" | \"FilterMap\" | \"Enumerate\" | \"Inspect\" | \"Skip\" | \"SkipWhile\" | \"Copied\" | \"Cloned\"\n                | \"Fuse\" | \"Peekable\" | \"FlatMap\" | \"Flatten\" | \"Take\" | \"TakeWhile\" | \"Scan\" | \"MapWhile\",\n",
    replace: "                \"Map\" | \"Filter\" | \"FilterMap\" | \"Enumerate\" | \"Inspect\" | \"Skip\" | \"SkipWhile\" | \"Copied\" | \"Cloned\"\n                | \"Fuse\" | \"Peekable\" | \"FlatMap\" | \"Flatten\" | \"TakeWhile\" | \"Scan\" | \"MapWhile\",\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_take_own_nth.rs$"],
  },
  {
    name: "take-while-folds-unseen",
    breaks:
      "`last()` of a `take_while` isn't by its fold, so a `skip` inside asks the crate's own `nth(n)`, not `nth(n - 1)` (#214)",
    file: "src/lower/recognition/own_steps.rs",
    find: "                \"Map\" | \"Filter\" | \"FilterMap\" | \"Enumerate\" | \"Inspect\" | \"Skip\" | \"SkipWhile\" | \"Copied\" | \"Cloned\"\n                | \"Fuse\" | \"Peekable\" | \"FlatMap\" | \"Flatten\" | \"Take\" | \"TakeWhile\" | \"Scan\" | \"MapWhile\",\n",
    replace: "                \"Map\" | \"Filter\" | \"FilterMap\" | \"Enumerate\" | \"Inspect\" | \"Skip\" | \"SkipWhile\" | \"Copied\" | \"Cloned\"\n                | \"Fuse\" | \"Peekable\" | \"FlatMap\" | \"Flatten\" | \"Take\" | \"Scan\" | \"MapWhile\",\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
  {
    name: "chain-nths-unseen",
    breaks:
      "a `chain`'s `nth` isn't said to reach its second side's own, so `chain(c()).step_by(2)` is a filter of indices that asks none (#214)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Nth, \"Chain\") => &[(1, Nth)],\n",
    replace: "            (Nth, \"Chain\") => &[],\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
  {
    name: "scan-folds-unseen",
    breaks:
      "`last()` of a `scan` or a `map_while` isn't by its fold, so a `step_by` inside asks the crate's own `nth(0)` first (#216)",
    file: "src/lower/recognition/own_steps.rs",
    find: "                \"Map\" | \"Filter\" | \"FilterMap\" | \"Enumerate\" | \"Inspect\" | \"Skip\" | \"SkipWhile\" | \"Copied\" | \"Cloned\"\n                | \"Fuse\" | \"Peekable\" | \"FlatMap\" | \"Flatten\" | \"Take\" | \"TakeWhile\" | \"Scan\" | \"MapWhile\",\n",
    replace: "                \"Map\" | \"Filter\" | \"FilterMap\" | \"Enumerate\" | \"Inspect\" | \"Skip\" | \"SkipWhile\" | \"Copied\" | \"Cloned\"\n                | \"Fuse\" | \"Peekable\" | \"FlatMap\" | \"Flatten\" | \"Take\" | \"TakeWhile\",\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
  {
    name: "folds-its-own-not-reversed",
    breaks:
      "`rev().last()` of the crate's iterator steps it by `next_back`, not its own `rfold` (#217)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Fold, \"Rev\") => &[(0, Rfold)],\n",
    replace: "            (Fold, \"Rev\") => &[],\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_own_back_methods.rs$"],
  },
  {
    name: "folds-its-own-reversed-through-mut-ref",
    breaks:
      "a `rev()` of a `&mut` of the crate's iterator is read as folding by its own `rfold`, `$folded(..)` of what has none (#217)",
    file: "src/lower/recognition/own_steps.rs",
    find: "        matches!(self, OwnStep::Nth | OwnStep::NthBack)\n",
    replace: "        matches!(self, OwnStep::Nth | OwnStep::NthBack | OwnStep::Rfold)\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_own_back_methods.rs$"],
    snapshots: true,
  },
  {
    name: "nths-its-own-not-reversed",
    breaks:
      "`rev().nth(n)` of the crate's iterator reads as a JS iterator's `drop(n)`, not its own `nth_back`, `$nth` (#217)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Nth, \"Rev\") => &[(0, NthBack)],\n",
    replace: "            (Nth, \"Rev\") => &[],\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_own_back_methods.rs$"],
    snapshots: true,
  },
  {
    name: "reversed-user-iterator-any-adapter",
    breaks:
      "an adapter of the crate's iterator, not only its `rev()`, folds by its `rfold` (#217)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Fold, \"Rev\") => &[(0, Rfold)],\n",
    replace: "            (Fold, _) => &[(0, Rfold)],\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(adapters_own_nth_fold|own_fold_through_adapters|rev_own_back_methods).rs$",
    ],
  },
  {
    name: "rfolds-its-own-never",
    breaks:
      "`last()` of a `rev()` of a map of the crate's iterator steps it by `next_back`, not its own `rfold` (#220)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            return own.is_some();\n",
    replace: "            return own.is_some() && step != OwnStep::Rfold;\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "rfolds-its-own-no-take",
    breaks:
      "a `take`'s `rfold` isn't said to trim by its source's own `nth_back` (#220)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Rfold, \"Take\") => &[(0, Rfold), (0, NthBack)],\n",
    replace: "            (Rfold, \"Take\") => &[(0, Rfold)],\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
    snapshots: true,
  },
  {
    name: "rfolds-its-own-no-skip",
    breaks:
      "a `skip`'s `rfold`, its `try_rfold`, isn't said to reach a `take`'s trim by the crate's `nth_back` (#220)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Rfold, \"Skip\") => &[(0, TryRfold)],\n",
    replace: "            (Rfold, \"Skip\") => &[],\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "nth-backs-its-own-never",
    breaks:
      "`rev().nth(n)` of an adapter of the crate's iterator isn't its own `nth_back` (#220)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            return own.is_some();\n",
    replace: "            return own.is_some() && step != OwnStep::NthBack;\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
    snapshots: true,
  },
  {
    name: "rfolds-its-own-no-fuse",
    breaks:
      "a `fuse()`'s `rfold` isn't said to be its source's, so `rev().last()` of one doesn't fold by the crate's own (#224)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            (Rfold, \"Map\" | \"Filter\" | \"Enumerate\" | \"Fuse\" | \"Peekable\") => own,\n",
    replace: "            (Rfold, \"Map\" | \"Filter\" | \"Enumerate\" | \"Peekable\") => own,\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "nths-its-own-not-generic",
    breaks: "generic code's dictionary of its iterator gives std's compiled `Fuse` no `nth`, so a fuse's `nth` never asks the type's own (#496)",
    file: "src/lower/recognition/own_steps.rs",
    find: "        matches!(self, OwnStep::Nth | OwnStep::Last | OwnStep::Count)\n",
    replace: "        matches!(self, OwnStep::Last | OwnStep::Count)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_own_last.rs$"],
    // The runtime asks the value's own `nth` either way, so it's how the
    // JS reads that's checked: `$nthSome(it, 1)`, not a `drop(1).take(1)`
    // collected (the Nightly of 3c71ee8f9).
    snapshots: true,
  },
];
