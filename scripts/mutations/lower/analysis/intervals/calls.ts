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
  {
    name: "intervals-slice-get-unknown",
    breaks: "a slice's `get(i)`'s `Some` doesn't put `i` below its length, so `i += 1` of `while let Some(&item) = items.get(i)` keeps its wrap",
    file: "src/lower/analysis/intervals/calls.rs",
    find: "            after.gets.insert(local, (state.root(index), most.hi - 1));\n",
    replace: "            let _ = (index, most);\n",
    tests: ["test/lowering-integer-ranges.test.ts","-t","an index an item is there at counts on"],
  },
  {
    name: "intervals-option-copied-unknown",
    breaks: "an `Option`'s `cloned()` of a slice's `get(i)` isn't `Some` where that was, so `i += 1` of `while let Some(..) = items.get(i).cloned()` keeps its wrap",
    file: "src/lower/analysis/intervals/calls.rs",
    find: "            after.gets.insert(local, got);\n",
    replace: "            let _ = got;\n",
    tests: ["test/intervals-shadow.test.ts","-t","MIR decides"],
  },
  {
    name: "intervals-js-length-no-length",
    breaks: "a JS string's `length` of text isn't a length of it, so `i` of `while i + 1 < js::string::length(format)` stepping by two isn't kept below it and `i + 1` keeps its wrap",
    file: "src/lower/analysis/intervals/calls.rs",
    find: "                    after.lens.insert(local, of);\n                }\n            } else if text_length(self.tcx, callee) {\n",
    replace: "                    let _ = of;\n                }\n            } else if text_length(self.tcx, callee) {\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "a counter below a JS string"],
  },
  {
    name: "pushed-push-unknown",
    breaks: "a `push` makes nothing known pushed, so `items[items.len() - 1]` past one keeps its wrap (#1700)",
    file: "src/lower/analysis/intervals/calls.rs",
    find: "                        after.pushed.insert(root);\n",
    replace: "",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "a counter a loop breaks out of at a bound"],
  },
];
