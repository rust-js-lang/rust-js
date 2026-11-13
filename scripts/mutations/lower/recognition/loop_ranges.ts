// Mutations of src/lower/recognition/loop_ranges.rs, what a loop over a range or a slice's items calls of it (ADR 1137, #1672 M3).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "loop-ranges-rev-unrecognized",
    breaks: "`rev()` of a range isn't recognized, so `v[i]` of `for i in (0..v.len()).rev()` keeps its check",
    file: "src/lower/recognition/loop_ranges.rs",
    find: '            "rev" => RangeStep::Rev,\n',
    replace: '            "rev_" => RangeStep::Rev,\n',
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "an integer below another's value doesn't wrap"],
  },
  {
    name: "loop-ranges-min-unrecognized",
    breaks: "an integer's `min` isn't recognized, so `xs[i.min(xs.len() - 1)]` keeps its check",
    file: "src/lower/recognition/loop_ranges.rs",
    find: '        "cmp_ord_min" | "cmp_min" => false,\n',
    replace: '        "cmp_ord_min_" => false,\n',
    tests: ["test/lowering-collections.test.ts", "-t", "a table as long as another is indexed as it is"],
  },
  {
    name: "loop-ranges-step-by-unrecognized",
    breaks: "`step_by(n)` of a range isn't recognized, so `v[i]` of `for i in (0..v.len()).step_by(2)` keeps its check",
    file: "src/lower/recognition/loop_ranges.rs",
    find: '            "step_by" => RangeStep::StepBy,\n',
    replace: '            "step_by_" => RangeStep::StepBy,\n',
    tests: ["test/intervals-shadow.test.ts"],
  },
];
