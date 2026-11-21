// Mutations of src/lower/recognition/own_steps.rs (#182, #505, #521).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lasts-its-own-not-generic",
    breaks: "generic code's `last()` steps its iterator to its end, never asking the type's own `last` (#496)",
    file: "src/lower/recognition/own_steps.rs",
    find: "        matches!(self, OwnStep::Nth | OwnStep::Last | OwnStep::Count | OwnStep::NthBack)\n",
    replace: "        matches!(self, OwnStep::Nth | OwnStep::Count | OwnStep::NthBack)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_own_last.rs$"],
  },
  {
    name: "may-fold-its-own-not-generic",
    breaks:
      "generic code's `collect()` into a `String`, a `HashSet` or a `HashMap` steps its iterator by `next`, never asking the type's own `fold` (#521)",
    file: "src/lower/recognition/own_steps.rs",
    find: "                && (reach == Reach::May || step.asked_at_run_time() && self.may_write(ty, step));\n",
    replace: "                && (step.asked_at_run_time() && self.may_write(ty, step));\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_made_in_generic_code_asked_its_hint.rs$"],
    snapshots: true,
  },
  {
    name: "may-fold-its-own-not-through-adapters",
    breaks:
      "generic code's `map(f).collect::<String>()` steps its iterator by `next`, never asking the type's own `fold` through the adapter (#521)",
    file: "src/lower/recognition/own_steps.rs",
    find: "self.reaches_its_own(source, asked, reach))",
    replace: "self.reaches_its_own(source, asked, Reach::Sure))",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_own_fold.rs$"],
    snapshots: true,
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
    name: "nths-its-own-no-mut-ref",
    breaks: "&mut I's nth is I's",
    file: "src/lower/recognition/own_steps.rs",
    find: "            return step.through_a_mut_ref() && self.reaches_its_own(*inner, step, reach);\n",
    replace: "            return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
  {
    name: "folds-its-own-reversed-through-mut-ref",
    breaks:
      "a `rev()` of a `&mut` of the crate's iterator is read as folding by its own `rfold`, `$folded(..)` of what has none (#217)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            .all(|w| w.passes(self.method()) == Passed::Own)\n",
    replace: "            .any(|w| w.passes(self.method()) != Passed::Default)\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_own_back_methods.rs$"],
    snapshots: true,
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
    name: "nths-its-own-not-generic",
    breaks: "generic code's dictionary of its iterator gives std's compiled `Fuse` no `nth`, so a fuse's `nth` never asks the type's own (#496)",
    file: "src/lower/recognition/own_steps.rs",
    find: "        matches!(self, OwnStep::Nth | OwnStep::Last | OwnStep::Count | OwnStep::NthBack)\n",
    replace: "        matches!(self, OwnStep::Last | OwnStep::Count | OwnStep::NthBack)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_own_last.rs$"],
    // The runtime asks the value's own `nth` either way, so it's how the
    // JS reads that's checked: `$nthSome(it, 1)`, not a `drop(1).take(1)`
    // collected (the Nightly of 3c71ee8f9).
    snapshots: true,
  },
  {
    name: "flatten-inners-are-its-source",
    breaks: "what a flatten asks of its inner iterators is asked of its source, whose `count` is std's, so a flatten's `count` never reaches each inner one's own (#736)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            INNERS => self.flattened_inner(ty),\n",
    replace: "            INNERS => args.types().next(),\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_count_takes_each_inner_count.rs$"],
  },
  {
    name: "repeat-n-own-steps-unknown",
    breaks: "`RepeatN`'s own `last` and `count` aren't known to the lowering, which steps it by its `next`, cloning each (#1124)",
    file: "src/lower/recognition/own_steps.rs",
    find: "        Some(StdPath::IterRepeatN) => Some(&[OwnStep::Nth, OwnStep::Last, OwnStep::Count]),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^repeat_own_steps_clone_none.rs$"],
  },
  {
    name: "repeat-own-count-unknown",
    breaks: "`repeat(x).count()` is counted by `reduce`, stepping it forever, not by its own `count`, which panics (#1138)",
    file: "src/lower/recognition/own_steps.rs",
    find: "        Some(StdPath::IterRepeat) => Some(&[OwnStep::Nth, OwnStep::Last, OwnStep::Count]),\n",
    replace: "        Some(StdPath::IterRepeat) => Some(&[OwnStep::Nth]),\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_count_panics.rs$"],
  },
  {
    name: "identity-map-shows-own-count",
    breaks: "a `map(|x| x)` of a `fuse()` of the crate's iterator with its own `count` is left out, so a `count()` of it reaches the crate's `count` through the fuse, where std's `Map` counts by its `fold` (#1240)",
    file: "src/lower/recognition/own_steps.rs",
    find: "        [OwnStep::Nth, OwnStep::Last, OwnStep::NthBack, OwnStep::Count]",
    replace: "        [OwnStep::Nth, OwnStep::Last, OwnStep::NthBack]",
    tests: ["test/corpus.test.ts", "-t", "^identity_map_hides_its_sources_own_count\\.rs$"],
  },
  {
    name: "chunk-by-last-by-its-front",
    breaks: "a slice's `chunk_by(..).last()` by a predicate that runs code steps it from its front, where std's `ChunkBy::last` is its `next_back` (#1501)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            || step == OwnStep::Last\n                && self.search_runs_code(ty)\n",
    replace: "            || false\n                && self.search_runs_code(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_searches_ask_a_predicate_as_they_step.rs$"],
  },
  {
    name: "into-iter-fold-unreached",
    breaks:
      "generic code's `fuse()` of a `T::IntoIter` is given a dictionary with no `fold`, so std's compiled `Fuse` folds by the crate's own, which drops it, and then drops it again: `drop Up 1` twice (#1595)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            return (self.is_unknown(ty) || ty.is_trait())\n                && (reach == Reach::May",
    replace: "            return (matches!(ty.kind(), ty::Param(_)) || ty.is_trait())\n                && (reach == Reach::May",
    tests: ["test/corpus.test.ts", "-t", "^generic_consumer_of_an_into_iter_takes_it.rs$"],
  },
  {
    name: "asks-of-other-as-of-src",
    breaks: "what a chain or a zip asks of its second side is taken as asked of its first, so a chain's `last` never reaches its second side's own (#2112)",
    file: "src/lower/recognition/own_steps.rs",
    find: '                    "other" => 1,\n',
    replace: '                    "other" => 0,\n',
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_of_a_reference_given_to_generic_code.rs$"],
    snapshots: true,
  },
  {
    name: "asks-of-inner-as-of-src",
    breaks: "what a flatten asks of each inner iterator is taken as asked of its source, so a flatten's `count` never reaches its inners' own (#2112)",
    file: "src/lower/recognition/own_steps.rs",
    find: '                    "inner" => INNERS,\n',
    replace: '                    "inner" => 0,\n',
    tests: ["test/corpus.test.ts", "-t", "^(flatten_count_takes_each_inner_count|flatten_steps_its_inners_as_std).rs$"],
  },
  {
    name: "fold-reaches-its-own-nth",
    breaks:
      "a `step_by`'s or a `skip`'s `fold` is taken to reach the crate's own as its `nth` does, which `nth_folded_sources` answers, so a fold that drops what it folds is refused (#2112)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            .filter(|&(_, asked)| !(step.folds() && matches!(asked, OwnStep::Nth)))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(folded_functions_drop_before_the_rest|own_nth_through_adapters|adapters_own_nth_fold).rs$"],
  },
  {
    name: "cloned-as-its-source-whatever-it-hides",
    breaks:
      "a `cloned()` of the crate's iterator that writes `count` or `last` is that iterator in JS, so its `count` is the crate's own, where std's `Cloned` steps its `fold` (#2122)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            .all(|step| !self.reaches_its_own(source, step, Reach::Sure) || self.reaches_its_own(ty, step, Reach::Sure))\n",
    replace: "            .all(|_| true)\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_and_cloned_ask_what_std_does.rs$"],
  },
  {
    name: "param-nth-back-unreached",
    breaks: "a type parameter's `nth_back` is never said to reach its caller's own, so generic `by_ref().rev().nth(0)` steps `next_back` (#2134)",
    file: "src/lower/recognition/own_steps.rs",
    find: "        matches!(self, OwnStep::Nth | OwnStep::Last | OwnStep::Count | OwnStep::NthBack)\n",
    replace: "        matches!(self, OwnStep::Nth | OwnStep::Last | OwnStep::Count)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_nth_back_reaches_what_its_given.rs$"],
  },
  {
    name: "fold-not-through-try-fold",
    breaks:
      "what a fold asks of a source's `try_fold` isn't followed, so `fuse().skip(1).take(3).last()` steps by `next`, asking the crate's own `nth(1)`, where std's `Take::fold` is its source's `try_fold`, whose `Skip` asks `nth(0)` (#2195)",
    file: "src/lower/recognition/own_steps.rs",
    find: "                asked if asked.folds() => sources.extend(self.nth_folded_by(source, asked)),\n",
    replace: "                OwnStep::Fold => sources.extend(self.nth_folded_by(source, asked)),\n",
    tests: ["test/corpus.test.ts", "-t", "^fold_through_try_fold_reaches_own_nth.rs$"],
  },
  {
    name: "own-nth-within-never",
    breaks:
      "a flatten of `Option`s over a boxed iterator of the crate's with its own `nth` and no `size_hint` is a filter of the `Some`s, which steps it by `try_fold`, so a `skip(1)` between asks its `nth(0)` and a `next` where std's `Flatten` asks `nth(1)` (#2205)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            return self.own_iterator_method(ty, \"nth\").is_some() || self.own_back_method(ty, \"nth_back\").is_some();\n",
    replace: "            return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_of_options_steps_its_source_by_next.rs$"],
  },
  {
    name: "dyn-steps-seen",
    breaks:
      "a `dyn` iterator's steps are taken for std's defaults, so a `skip(n)`'s `last` of a `&mut dyn Iterator` is an array's last item, its `drop(n)` asking the crate's `nth(n)` where std's `Skip::last` asks `nth(n - 1)` (#2218)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            return (self.is_unknown(ty) || ty.is_trait())\n",
    replace: "            return self.is_unknown(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_last_of_a_lent_dyn.rs$"],
  },
  {
    name: "dyn-double-ended-unseen",
    breaks:
      "a `dyn DoubleEndedIterator` is taken for one no step is asked from its back, so a `rev().skip(n)`'s `last` of a `&mut dyn DoubleEndedIterator` asks the crate's `nth_back(n)` where std asks `nth_back(n - 1)` (#2218)",
    file: "src/lower/recognition/own_steps.rs",
    find: "                .chain(dyn_traits)\n",
    replace: "                .chain(dyn_traits.into_iter().filter(|_| false))\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_last_of_a_lent_dyn.rs$"],
  },
];
