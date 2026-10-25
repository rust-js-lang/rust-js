// Mutations of src/lower/analysis/unwinding.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "assert-steps-on",
    breaks: "a function whose bound or divisor check may panic is taken for one that never unwinds, so a panic in it skips the drops of what its caller holds",
    file: "src/lower/analysis/unwinding.rs",
    find: "            | TerminatorKind::Return\n",
    replace: "            | TerminatorKind::Return\n            | TerminatorKind::Assert { .. }\n",
    tests: ["test/corpus.test.ts", "-t", "^never_unwinding_calls.rs$"],
  },
  {
    name: "user-display-shown-whole",
    breaks: "a `Display` of the crate's is taken for one that can't fail, so a function printing one that panics is taken for one that never unwinds",
    file: "src/lower/analysis/unwinding.rs",
    find: "    ty.is_primitive() || ty.is_str() || recognition.is_lang_adt(ty, LangItem::String)\n",
    replace: "    let _ = (ty, recognition);\n    true\n",
    tests: ["test/corpus.test.ts", "-t", "^never_unwinding_calls.rs$"],
  },
];
