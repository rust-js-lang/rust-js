// Mutations of crates/rust-js-program/src/casing.rs (ADRs 0038, 0573).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "variable-case-unlowered",
    breaks: "a thread-local's words stay uppercase, `TASKQueue`",
    file: "crates/rust-js-program/src/casing.rs",
    find: "    words_joined(name, |word| word.to_ascii_lowercase())\n",
    replace: "    words_joined(name, |word| word.to_string())\n",
    tests: ["test/lowering-names.test.ts", "-t", "thread-local's SCREAMING_SNAKE"],
  },
  {
    name: "hyphenated-prop-field",
    breaks: "a component's `aria-label` is the field `r#aria-label`, which Rust can't parse",
    file: "crates/rust-js-program/src/casing.rs",
    find: "        if c == '-' {\n            result.push('_');\n        } else if c.is_ascii_uppercase() {\n",
    replace: "        if c.is_ascii_uppercase() {\n",
    tests: ["test/next.test.ts", "-t", "next build builds"],
  },
];
