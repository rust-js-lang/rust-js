// Mutations of src/lower/casing.rs (ADRs 0038, 0573).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "variable-case-unlowered",
    breaks: "a thread-local's words stay uppercase, `TASKQueue`",
    file: "src/lower/casing.rs",
    find: "    words_joined(name, |word| word.to_ascii_lowercase())\n",
    replace: "    words_joined(name, |word| word.to_string())\n",
    tests: ["test/lowering-names.test.ts", "-t", "thread-local's SCREAMING_SNAKE"],
  },
];
