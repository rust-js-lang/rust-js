// Mutations of src/lower/mir/cfg.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-no-back-edges",
    breaks:
      "an edge back to a loop's header isn't a `continue`, so a loop runs once",
    file: "src/lower/mir/cfg.rs",
    find: "        self.headers.contains(&to) && self.dominates(to, from)\n",
    replace: "        false\n",
    tests: ["test/corpus.test.ts", "-t", "loop_values"],
  },
  {
    name: "mir-merges-innermost-first",
    breaks:
      "a block's merges are labeled earliest outermost, so a `break` skips what follows",
    file: "src/lower/mir/cfg.rs",
    find: "        children.sort_by_key(|c| std::cmp::Reverse(self.order[c.as_usize()]));\n",
    replace: "        children.sort_by_key(|c| self.order[c.as_usize()]);\n",
    tests: ["test/corpus.test.ts", "-t", "labeled_blocks"],
  },
  {
    name: "loop-entered-mid-way-unrefused",
    breaks: "custom MIR's jump into a loop other than at its start is lowered, where tidy breaks its contracts, a crash (#1383)",
    file: "src/lower/mir/cfg.rs",
    find: "                .any(|&s| self.order[s.as_usize()] <= self.order[b.as_usize()] && !self.dominates(s, b))\n",
    replace: "                .any(|&s| self.order[s.as_usize()] <= self.order[b.as_usize()] && !self.dominates(s, b) && false)\n",
    tests: ["test/diagnostics.test.ts", "-t", "custom MIR's jump into a loop other than at its start is refused"],
  },
];
