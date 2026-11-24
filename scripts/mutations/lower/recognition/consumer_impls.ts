// Mutations of crates/rust-js-lower/src/recognition/consumer_impls.rs.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "from-iter-of-values-item",
    breaks:
      "a fallible `collect` into the crate's own collection asks its `FromIterator` of the wrong item, and finds none",
    file: "crates/rust-js-lower/src/recognition/consumer_impls.rs",
    find: "    Some((from_iter, tcx.mk_args(&[collection.into(), item.into(), given.into()])))\n",
    replace: "    Some((from_iter, tcx.mk_args(&[collection.into(), given.into(), given.into()])))\n",
    tests: ["test/corpus.test.ts", "-t", "collect_fallible_into"],
  },
  {
    name: "infallible-an-enum",
    breaks: "`Infallible`, `!` since 1.100, is taken for the enum it was: rustc panics asking its ADT (ADR 0109, amended)",
    file: "crates/rust-js-lower/src/recognition/consumer_impls.rs",
    find: "    let infallible = tcx.types.never;\n",
    replace: "    let infallible = Ty::new_adt(tcx, tcx.adt_def(super::registry::by_path(tcx, StdCrate::Core, &[\"convert\", \"Infallible\"])?), ty::List::empty());\n",
    tests: ["test/corpus.test.ts", "-t", "^calculator.rs$"],
  },
  {
    name: "own-sum-unanswered",
    breaks:
      "`sum()` into the crate's own `Sum` isn't answered as calling it, so what it sums isn't seen given there (#2376)",
    file: "crates/rust-js-lower/src/recognition/consumer_impls.rs",
    find: "        name @ (\"sum\" | \"product\") => {\n",
    replace: "        name @ \"product\" => {\n",
    tests: ["test/corpus.test.ts", "-t", "^hint_asked_by_a_crates_own_collect_and_sum.rs$"],
  },
  {
    name: "fallible-collect-not-shunted",
    breaks:
      "a `collect()` into a `Result` of the crate's collection is answered as std's `Result`'s `from_iter`, not its own given std's `GenericShunt`, so what it collects isn't seen given there (#2376)",
    file: "crates/rust-js-lower/src/recognition/consumer_impls.rs",
    find: "            Some((collection, value, shunt)) => values_from_iter(tcx, collection, value, Some(shunt))?,\n",
    replace: "            Some(_) => values_from_iter(tcx, output, item, Some(iter))?,\n",
    tests: ["test/corpus.test.ts", "-t", "^hint_asked_by_a_crates_own_collect_and_sum.rs$"],
  },
  {
    name: "partition-extend-unanswered",
    breaks:
      "a `partition`'s side's `extend`, which std's compiled body calls of the crate's impl, isn't answered, so the callers' carrier misses what it's given and the debug build's check stops a compile that succeeds (#2387)",
    file: "crates/rust-js-lower/src/recognition/consumer_impls.rs",
    find: "            \"partition\" => {\n",
    replace: "            \"partition\" if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^partition_fills_each_side_as_std_does.rs$"],
  },
  {
    name: "unzip-crate-extend-unnamed",
    breaks: "`unzip()`'s sides' `Extend::extend` isn't named as what std's compiled body calls of the crate's, so the crate's `extend` is never compiled as it's called (#2395)",
    file: "crates/rust-js-lower/src/recognition/consumer_impls.rs",
    find: "                vec![side(a?, from_a?), side(b?, from_b?)]\n",
    replace: "                vec![]\n",
    tests: ["test/corpus.test.ts", "-t", "^unzip_fills_each_side_as_std_extends_it.rs$"],
  },
];
