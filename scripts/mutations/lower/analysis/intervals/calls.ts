// Mutations of src/lower/analysis/intervals/calls.rs, split from intervals.rs (ADR 1137, #1672).
import type { Mutation } from "../../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-range-next-unknown",
    breaks: "a for loop's value isn't known below its range's end, so `path[i]` of `for i in 1..len` keeps its check",
    file: "src/lower/analysis/intervals/calls.rs",
    find: "                (Some(RangeStep::Next), Held::Lent) if !matches!(given.end, End::Items(_)) => {\n",
    replace: "                (Some(RangeStep::Next), Held::Lent) if false => {\n",
    tests,
  },
  {
    name: "intervals-min-unrelated",
    breaks: "an integer's `min` isn't below what either operand is below, so `xs[i.min(xs.len() - 1)]` keeps its check",
    file: "src/lower/analysis/intervals/calls.rs",
    find: "                false => ra.union(&rb).copied().collect(),\n",
    replace: "                false => Vec::new(),\n",
    tests: ["test/lowering-collections.test.ts","-t","a table as long as another is indexed as it is"],
  },
  {
    name: "intervals-rest-deref-unknown",
    breaks: "a rest value's `Deref` isn't the array it is, so `value[0]` under `value.is_empty()`'s test keeps its check",
    file: "src/lower/analysis/intervals/calls.rs",
    find: "            after.copies.insert(local, state.root(of));\n",
    replace: "            let _ = of;\n",
    tests: ["test/rest-parameters.test.ts","-t","rest arguments as a value indexed in bounds"],
  },
  {
    name: "intervals-enumerate-uncounted",
    breaks: "`enumerate()` of a slice's iterator isn't counted, so `terraces[i]` of its index keeps its check",
    file: "src/lower/analysis/intervals/calls.rs",
    find: "                (Some(RangeStep::Enumerate), Held::Range)\n",
    replace: "                (Some(RangeStep::Enumerate), Held::Lent)\n",
    tests: ["test/lowering-collections.test.ts","-t","an enumerate's index is below its slice's length"],
  },
  {
    name: "intervals-text-length-unbounded",
    breaks: "text's `len()` may be `usize::MAX`, so `a.len() + b.len()` of two `&str`s may wrap",
    file: "src/lower/analysis/intervals/calls.rs",
    find: "            } else if text_length(self.tcx, callee) {\n",
    replace: "            } else if false && text_length(self.tcx, callee) {\n",
    tests: ["test/intervals-shadow.test.ts"],
  },
];
