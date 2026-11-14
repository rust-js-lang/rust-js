// Mutations of src/lower/analysis/intervals/callers.rs, split from intervals.rs (ADR 1137, #1672).
import type { Mutation } from "../../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-callers-relations-unpassed",
    breaks: "a relation between arguments isn't passed, so `item(items, i)` under `i < items.len()` keeps its check",
    file: "src/lower/analysis/intervals/callers.rs",
    find: "                below.push((index, of, strict));\n",
    replace: "                let _ = (index, of, strict);\n",
    tests: ["test/lowering-functions.test.ts","-t","a function only calls reach is what its calls give it"],
  },
  {
    name: "intervals-callers-walk-every-body",
    breaks: "the callers' walk walks every body, though it calls no function only calls reach, so serde's derived bodies, thousands of blocks each, take gigabytes",
    file: "src/lower/analysis/intervals/callers.rs",
    find: "    if reaching.is_empty() {\n",
    replace: "    if reaching.is_empty() && false {\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "gigabytes"],
  },
  {
    name: "callers-recursion-unknown",
    breaks: "a function that calls itself is given nothing, so `k + 1` of `walk`'s `k`, the same at each call and at most 7, keeps its wrap, left to the bridge (#1742)",
    file: "src/lower/analysis/intervals/callers.rs",
    find: "        if !calls_itself {\n",
    replace: "        if true {\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "a function that calls itself"],
  },
  {
    name: "callers-recursion-never-forgets",
    breaks: "what a function's own calls still change of a parameter isn't forgotten, so `n + 1` of `up(n + 1, d - 1)`, which passes 255, is taken in range past 64 rounds, all of it then forgotten, or never settles",
    file: "src/lower/analysis/intervals/callers.rs",
    find: "                        if round > 64 || was.is_none_or(|was| was.intervals.get(at) != Some(interval)) {\n",
    replace: "                        if round > 64 {\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "a function that calls itself"],
  },
  {
    name: "callers-recursion-outside-unjoined",
    breaks: "a function that calls itself is given what its own calls give alone, not what its callers outside do, so `k` of `walk(0, 3)` and `walk(25, 7)` is known nothing, or wrongly",
    file: "src/lower/analysis/intervals/callers.rs",
    find: "                (Some(outside), Some(inner)) => Some(joined(outside, inner)),\n",
    replace: "                (Some(_), Some(inner)) => Some(inner.clone()),\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "a function that calls itself"],
  },
];
