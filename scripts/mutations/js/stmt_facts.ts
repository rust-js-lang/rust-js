// Mutations of src/js/stmt_facts.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rebuilt-forgets-facts",
    breaks: "a statement rebuilt in its place, an `if` made again of its branches, forgets what's known of it: its written `else` dropped, `if (a) { ..; return; } else { b }` printed `if (a) { ..; return; } b`, an `else` of only comments lost (ADR 1167)",
    file: "src/js/stmt_facts.rs",
    find: "            span: self.span,\n            facts: self.facts,\n",
    replace: "            span: self.span,\n            facts: StmtFacts::default(),\n",
    unitTests: "js::stmt_facts::tests::a_statement_rebuilt_keeps_its_facts",
  },
];
