// Mutations of src/runtime.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "pure-helpers-act",
    breaks: "the runtime's helpers that only read, `$jsonIndex(v, \"a\")`, count as doing something, held apart from MIR",
    file: "src/runtime.rs",
    find: "    \"$jsonIndex\",\n",
    replace: "",
    tests: ["test/serde.test.ts","-t","a Value's reads"],
  },
  {
    name: "range-step-may-throw",
    breaks: "a range's step, which never throws, is taken for a call that may, and a loop with a drop in it is a `while (true)` of a `try`, not a `for`",
    file: "src/runtime.rs",
    find: "const NEVER_THROW: &[&str] = &[\"$rangeNext\", \"$rangeNextBack\"];\n",
    replace: "const NEVER_THROW: &[&str] = &[];\n",
    tests: ["test/corpus.test.ts", "-t", "drop_flag_loops"],
    snapshots: true,
  },
];
