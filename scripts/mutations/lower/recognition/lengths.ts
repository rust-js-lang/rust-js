// Mutations of src/lower/recognition/lengths.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const endlessOnce = [
  "test/corpus.test.ts",
  "-t",
  "^collect_endless_once_stepped.rs$",
];

export const mutations: Mutation[] = [
  {
    name: "cycle-endless-unseen",
    breaks:
      "a `cycle()` of an array's items isn't taken to have no end once it gives an item, so `items.iter().cycle().map(f).collect::<Vec<_>>()` maps without end where Rust panics `capacity overflow` after the first (#433, ADR 0940)",
    file: "src/lower/recognition/lengths.rs",
    find: '            "std::iter::Cycle" => sources.next().map_or(Some(false), |of| self.lower_bound_kept(of)),\n',
    replace: '            "std::iter::Cycle" => Some(false),\n',
    tests: endlessOnce,
  },
  {
    name: "skip-of-endless-unseen",
    breaks:
      "a `skip(n)` of what has no end isn't taken to have none once it gives an item, so `(0..).skip(2).map(f).collect::<Vec<_>>()` maps without end where Rust panics `capacity overflow` after the first (#433, ADR 0940)",
    file: "src/lower/recognition/lengths.rs",
    find: '            | "std::iter::Skip"\n            | "std::iter::Enumerate"\n',
    replace: '            | "std::iter::Enumerate"\n',
    tests: endlessOnce,
  },
  {
    name: "chain-of-cycle-and-an-end-collected",
    breaks:
      "`collect()` of a `cycle()` chained with what has an end is compiled as one that has none, so a cycle with no items panics `capacity overflow` where Rust collects what follows (#433, ADR 0940)",
    file: "src/lower/recognition/lengths.rs",
    find: "                (Some(a), Some(b)) if a == b => Some(a),\n                _ => None,\n            },\n            // An item of both, each stepped.\n",
    replace:
      "                (Some(a), Some(b)) => Some(a || b),\n                _ => None,\n            },\n            // An item of both, each stepped.\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_cycle_chained_refused.rs$"],
  },
  {
    name: "zip-of-one-endless",
    breaks:
      "a zip of a `cycle()` and what has an end is taken to have none, so its `collect()` panics `capacity overflow` where its lower bound, the other side's, fits (#433, ADR 0940)",
    file: "src/lower/recognition/lengths.rs",
    find: '            // An item of both, each stepped.\n            "std::iter::Zip" => match (source(), source()) {\n                (Some(false), _) | (_, Some(false)) => Some(false),\n                (Some(true), Some(true)) => Some(true),\n',
    replace:
      '            // An item of both, each stepped.\n            "std::iter::Zip" => match (source(), source()) {\n                (Some(true), _) | (_, Some(true)) => Some(true),\n                (Some(false), Some(false)) => Some(false),\n',
    tests: endlessOnce,
  },
  {
    name: "step-by-of-endless-collected",
    breaks:
      "`collect()` of a `step_by(n)` of what has no end is compiled as one with no end once stepped, its `capacity overflow` asked of `usize::MAX`, not of `usize::MAX / n` (#433, ADR 0940)",
    file: "src/lower/recognition/lengths.rs",
    find: "                Some(true) => None,\n                other => other,\n",
    replace: "                other => other,\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^collect_step_by_endless_refused.rs$",
    ],
  },
  {
    name: "cycle-of-any-source-endless",
    breaks:
      "a `cycle()` of what's filtered, whose hint has a lower bound of 0, is taken to have no end once it gives an item, so its `collect()` panics `capacity overflow` where Rust asks a little room and goes on (#433, ADR 0940)",
    file: "src/lower/recognition/lengths.rs",
    find: "        if exact\n            || crate::lower::recognition::is_trusted_len(self.tcx, self.typing_env, ty)\n",
    replace:
      "        if true\n            || crate::lower::recognition::is_trusted_len(self.tcx, self.typing_env, ty)\n",
    tests: endlessOnce,
  },
  {
    name: "cycle-of-partly-filtered-collected",
    breaks:
      "a `cycle()` of an array's items chained with what's filtered is taken to have a lower bound above 0 wherever it has items, so its `collect()` panics `capacity overflow` where only the filter gives any (#433, ADR 0940)",
    file: "src/lower/recognition/lengths.rs",
    find: '                (Some(a), Some(b)) if a == b => Some(a),\n                _ => None,\n            },\n            "std::iter::Zip"',
    replace:
      '                (Some(a), Some(b)) => Some(a || b),\n                _ => None,\n            },\n            "std::iter::Zip"',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^collect_cycle_partly_filtered_refused.rs$",
    ],
  },
  {
    name: "std-size-hint-taken-as-own",
    breaks:
      "an iterator of the crate's with std's `size_hint` is given it as its own, a call of the provided method",
    file: "src/lower/recognition/lengths.rs",
    find: "                instance.def_id() != size_hint\n                    && (instance.def_id().is_local() || self.foreign.compiled(instance.def_id()))\n",
    replace: "                true\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_size_hints.rs$"],
  },
  {
    name: "hint-reaches-own-never",
    breaks: "a collect into a `String` of the crate's iterator's `char`s asks its hint not before it steps, as `Extend<char>` does (#733)",
    file: "src/lower/recognition/lengths.rs",
    find: "            return self.writes_size_hint(ty);\n",
    replace: "            return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^size_hint_of_adapters.rs$"],
  },
  {
    name: "hint-reaches-own-not-through-adapters",
    breaks: "a collect into a `HashSet` of a `filter()` of the crate's iterator asks its hint not first, as hashbrown's `extend` does (#733)",
    file: "src/lower/recognition/lengths.rs",
    find: "            .is_some_and(|held| held.into_iter().any(|held| self.hint_reaches_own(held)))\n",
    replace: "            .is_some_and(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^size_hint_of_adapters.rs$"],
  },
  {
    name: "hint-unasked-of-a-caller",
    breaks: "generic code's collect into a `String` of a type parameter's iterator asks no hint, so the crate's own its caller gives never runs, where std's `Extend<char>` asks it (#733)",
    file: "src/lower/recognition/lengths.rs",
    find: "        if self.is_unknown(ty) {\n            return true;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^collect_asks_hint_as_it_grows.rs$"],
  },
  {
    name: "eq-side-into-iter-built",
    breaks: "`<I as IntoIterator>::IntoIter` isn't the side a type parameter's caller gives, so itertools' `equal` is refused as a side made of type parameters (#1182)",
    file: "src/lower/recognition/lengths.rs",
    find: "                Some(EqSide::Given(alias.self_ty()))\n",
    replace: "                Some(EqSide::Built)\n",
    tests: ["test/crates.test.ts", "-t", "length_first_eq"],
  },
  {
    name: "counted-where-given-none",
    breaks: "an array's iterator given to generic code's `eq` isn't counted, so a range given it is refused (#1182)",
    file: "src/lower/recognition/lengths.rs",
    find: "            \"std::slice::Iter\" | \"std::slice::IterMut\" | \"std::vec::IntoIter\" | \"std::array::IntoIter\" | \"std::ops::Range\"\n",
    replace: "            \"std::slice::IterMut\"\n",
    tests: ["test/crates.test.ts", "-t", "length_first_eq"],
  },
  {
    name: "lower-bound-of-counted-arrays-stds",
    breaks: "a slice's `split(..)`, which JS counts, is taken to have std's lower bound, so a `Vec` collected of it in a crate that reads a capacity compiles, its capacity another (#1180)",
    file: "src/lower/recognition/lengths.rs",
    find: "        !self.is_array_iter(ty)\n            || self\n",
    replace: "        true || !self.is_array_iter(ty)\n            || self\n",
    tests: ["test/diagnostics.test.ts", "-t", "a Vec collected of a slice's split"],
  },
  {
    name: "lower-bound-of-lazy-chars-stds",
    breaks: "a lazy `chars()`, whose JS adapters keep no hint, is taken to have std's lower bound, so a `Vec` collected of its `skip(n)` in a crate that reads a capacity compiles, its capacity another (#1186, #1200)",
    file: "src/lower/recognition/lengths.rs",
    find: "        if self.is_str_chars(ty) || self.is_char_indices(ty) {\n            return false;\n",
    replace: "        if false {\n            return false;\n",
    tests: ["test/diagnostics.test.ts", "-t", "a Vec collected of chars"],
  },
  {
    name: "chain-of-an-exact-adapter-trusted",
    breaks: "in a crate that reads a capacity, a `Vec` collected of a chain whose part is an exact-sized adapter, `a.iter().skip(1).chain(rest)`, compiles, grown by JS's own `drop(n)`'s hint, none, its capacity another (#1200)",
    file: "src/lower/recognition/lengths.rs",
    find: "            if self.grows_by_exact_len(ty) {\n                return false;\n            }\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "a Vec collected of a chain of a skip"],
  },
  {
    name: "range-hinted-as-counted",
    breaks: "a B-tree's `range(..)` collected in a crate that reads a capacity is grown by JS's count of what's in range, 38, where std's hint has no lower bound and its capacity is 64 (#1215)",
    file: "src/lower/recognition/lengths.rs",
    find: "                    | \"std::collections::btree_map::Range\"\n                    | \"std::collections::btree_map::RangeMut\"\n                    | \"std::collections::btree_set::Range\"\n",
    replace: "                    | \"std::collections::btree_map::RangeMut\"\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_what_std_hints_less.rs$"],
  },
  {
    name: "merged-lower-bound-unrefused",
    breaks: "a B-tree set's `difference(..)` collected in a crate that reads a capacity compiles, grown by JS's count where std's merge gives another lower bound (#1215)",
    file: "src/lower/recognition/lengths.rs",
    find: "        if self.hints_merged_lower_bound(ty) {\n            return false;\n        }\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "a Vec collected of a set's difference"],
  },
  {
    name: "text-patterns-hinted-as-counted",
    breaks: "a string's `split(..)` given to generic code that collects it, in a crate that reads a capacity, is grown by JS's count of its parts, where std's hint has no lower bound (#1180's (3))",
    file: "src/lower/recognition/lengths.rs",
    find: "        self.is_str_split(ty)\n            || matches!(\n",
    replace: "        false\n            || matches!(\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_text_patterns.rs$"],
  },
  {
    name: "eq-side-of-an-opaque-type-unrevealed",
    breaks:
      "an `eq` side an `impl Iterator` hides is asked as the opaque type, made of type parameters, so generic code's `eq` of one its caller gives is refused, where std's specialization sees the type it hides (#1229)",
    file: "src/lower/recognition/lengths.rs",
    find: "            let ty = crate::lower::mir::revealed(self.tcx, ty);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^eq_of_opaque_iterators.rs$"],
  },
];
