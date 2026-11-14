// Mutations of src/lower/analysis/intervals/session.rs, split from intervals.rs (ADR 1137, #1672).
import type { Mutation } from "../../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-mir-only-unreported",
    breaks: "what MIR proves that THIR's facts keep, which M3 leaves out, isn't in the shadow to be triaged",
    file: "src/lower/analysis/intervals/session.rs",
    find: "        (false, true) => &format!(\"mir-only-{kind}\"),\n",
    replace: "        (false, true) => return,\n",
    tests,
  },
  {
    name: "intervals-casts-unwalked",
    breaks: "a body whose only question is an integer cast isn't walked, so `s.len() as i32` is `s.length | 0` (#1672 M3)",
    file: "src/lower/analysis/intervals/session.rs",
    find: " | Rvalue::Cast(mir::CastKind::IntToInt, ..)))\n",
    replace: "))\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "an integer cast MIR's intervals show"],
  },
  {
    name: "intervals-constants-call-nothing",
    breaks: "a constant's MIR isn't one of its `const fn`s' callers, so `add`, only `const SUM` calls, is given nothing and `a + b` keeps its wrap, left to the bridge (#1742)",
    file: "src/lower/analysis/intervals/session.rs",
    find: "            callers.insert(owner, tcx.mir_for_ctfe(owner));\n",
    replace: "            let _ = owner;\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "a const fn constants and patterns call"],
  },
];
