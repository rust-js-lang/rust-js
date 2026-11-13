// Mutations of src/lower/analysis/intervals/state.rs, split from intervals.rs (ADR 1137, #1672).
import type { Mutation } from "../../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-copies-unnarrowed",
    breaks: "a comparison narrows only what's compared, not its copies, so `i += 1` under `i > 100` of `let mut i = start` may wrap",
    file: "src/lower/analysis/intervals/state.rs",
    find: "        for l in [local, root].into_iter().chain(copies) {\n",
    replace: "        for l in [root].into_iter().chain(copies.into_iter().take(0)) {\n",
    tests,
  },
];
