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
  {
    name: "local-key-with-unrecognized",
    breaks: "a thread-local's `with_borrow` gives nothing of its closure's result, so `i += 1` past `while let Some(_) = ITEMS.with_borrow(|items| items.get(i).copied())` keeps its wrap",
    file: "src/lower/recognition/loop_ranges.rs",
    find: '        "with" | "with_borrow" | "with_borrow_mut"\n',
    replace: '        "with" | "with_borrow" | "with_borrow_mut" if false\n',
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "tells its caller the index"],
  },
  {
    name: "range-index-unrecognized",
    breaks: "no `v[a..b]` is a range index, so `v[..]` and `v[a..n]` under `a <= n <= v.len()` aren't proved, no mir-only-call-bounds line",
    file: "src/lower/recognition/loop_ranges.rs",
    find: "    (held && ranged(index)).then_some(1)\n",
    replace: "    (held && ranged(index) && false).then_some(1)\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a range's bounds"],
  },
  {
    name: "skip-unrecognized",
    breaks: "`skip(n)` of an `enumerate()` gives no index below its slice's length, so `candidates[index]` in a loop over it keeps its check",
    file: "src/lower/recognition/loop_ranges.rs",
    find: "            \"skip\" => RangeStep::Skip,\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "0 before it"],
  },
  {
    name: "drain-unranged",
    breaks: "a `Vec`'s `drain(0..i)` isn't a range MIR proves, so a drain to an index known in bounds keeps its check, a call-bounds line (#1815)",
    file: "src/lower/recognition/loop_ranges.rs",
    find: "        return (drain && args.types().last().is_some_and(ranged)).then_some(1);\n",
    replace: "        return None;\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "guard keeps"],
  },
];
