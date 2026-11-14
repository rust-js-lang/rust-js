// Mutations of src/lower/analysis/intervals/pushed.rs (ADR 1137, #1700).
import type { Mutation } from "../../../../mutations";

const tests = ["test/lowering-integer-ranges.test.ts", "-t", "a counter a loop breaks out of at a bound"];

export const mutations: Mutation[] = [
  {
    name: "pushed-kept-past-a-lend",
    breaks: "a vector pushed stays at least one long past a `&mut` of it, so `v.len() - 1` after `f(v)`, which may clear it, is JS's own `-` and gives `-1`",
    file: "src/lower/analysis/intervals/pushed.rs",
    find: "            state.pushed.shift_remove(&of.local);\n            state.pushed.shift_remove(&root);\n",
    replace: "",
    tests,
  },
  {
    name: "pushed-raw-pointed",
    breaks: "a vector a raw pointer is made of is known pushed, so `v.len() - 1` after `(*p).clear()` is JS's own `-` and gives `-1`",
    file: "src/lower/analysis/intervals/pushed.rs",
    find: "                    && !self.pointed.contains(&root)\n",
    replace: "",
    tests,
  },
  {
    name: "pushed-shared-reborrow-unknown",
    breaks: "a shared reborrow of a vector pushed isn't, so `properties.len()` past a push, of `&(*properties)`, is any length, and `prevLength - 1` keeps its wrap",
    file: "src/lower/analysis/intervals/pushed.rs",
    find: "                if whole(of) && state.pushed.contains(&of.local) =>\n",
    replace: "                if false && whole(of) && state.pushed.contains(&of.local) =>\n",
    tests,
  },
];
