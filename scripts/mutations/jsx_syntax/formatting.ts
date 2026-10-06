// Mutations of src/jsx_syntax/formatting.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "format-tags-unseen",
    breaks: "the formatter takes `<Comp>` for a component, and refuses the attributes after its spread",
    file: "src/jsx_syntax/formatting.rs",
    find: "        self.layout.tags.extend(tags_of(&kind));\n",
    replace: "",
    tests: ["test/format.test.ts", "-t", "tag value"],
  },
];
