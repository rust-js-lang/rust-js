// Mutations of src/lower/analysis/intervals/callers.rs, split from intervals.rs (ADR 1137, #1672).
import type { Mutation } from "../../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-given-before-every-call",
    breaks: "a function is given what its calls seen so far pass, before its own call of itself is, so a recursive function is taken to be given only its outer calls' values",
    file: "src/lower/analysis/intervals/callers.rs",
    find: "        true => given.get(&function).cloned().unwrap_or_default(),\n",
    replace: "        true | false if given.contains_key(&function) => given.get(&function).cloned().unwrap_or_default(),\n        true => Vec::new(),\n",
    tests,
  },
  {
    name: "intervals-callers-relations-unpassed",
    breaks: "a relation between arguments isn't passed, so `item(items, i)` under `i < items.len()` keeps its check",
    file: "src/lower/analysis/intervals/callers.rs",
    find: "                        below.push((index, of, strict));\n",
    replace: "                        let _ = (index, of, strict);\n",
    tests: ["test/lowering-functions.test.ts","-t","a function only calls reach is what its calls give it"],
  },
];
