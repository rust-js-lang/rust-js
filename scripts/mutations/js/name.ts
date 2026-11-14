// Mutations of src/js/name.rs (ADR 0093): a variable's name, one form for
// now, its spelling (#1681 change 3, M3, S1).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "name-unlike-its-word",
    breaks: "a name compared with a word, `Var(v) if v == \"Boolean\"`, is never it: what a pass asks of a given name, a global's or a helper's, is never so",
    file: "src/js/name.rs",
    find: "    fn eq(&self, other: &str) -> bool {\n        self.0 == other\n    }\n",
    replace: "    fn eq(&self, other: &str) -> bool {\n        self.0 != other\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^(std_map|user_deref|clone_from_generic_std_own).rs$"],
  },
  {
    name: "name-as-no-word",
    breaks: "a name's word, what a pass asks of it, `names.contains(v.as_str())`, is nothing, so no name is the one it asks of",
    file: "src/js/name.rs",
    find: "    pub fn as_str(&self) -> &str {\n        &self.0\n    }\n",
    replace: "    pub fn as_str(&self) -> &str {\n        \"\"\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^(std_map|user_deref|clone_from_generic_std_own).rs$"],
  },
];
