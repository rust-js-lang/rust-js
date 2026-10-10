// Mutations of src/lower/mir/cfg.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-no-back-edges",
    breaks: "an edge back to a loop's header isn't a `continue`, so a loop runs once",
    file: "src/lower/mir/cfg.rs",
    find: "        self.headers.contains(&to) && self.dominates(to, from)\n",
    replace: "        false\n",
    tests: ["test/mir.test.ts", "-t", "loop_values"],
  },
  {
    name: "mir-merges-innermost-first",
    breaks: "a block's merges are labeled earliest outermost, so a `break` skips what follows",
    file: "src/lower/mir/cfg.rs",
    find: "        children.sort_by_key(|c| std::cmp::Reverse(self.order[c.as_usize()]));\n",
    replace: "        children.sort_by_key(|c| self.order[c.as_usize()]);\n",
    tests: ["test/mir.test.ts", "-t", "labeled_blocks"],
  },
];
