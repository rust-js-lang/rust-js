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
];
