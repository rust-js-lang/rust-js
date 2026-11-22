// Mutations of crates/rust-js-program/src/js/stmt_facts.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rebuilt-write-answer-kept",
    breaks: "a write rebuilt of another value keeps what MIR said of the first, so a value that may be `undefined` is stored as it is (#2112 S4b)",
    file: "crates/rust-js-program/src/js/stmt_facts.rs",
    find: "            true => self.facts.written,\n            false => Undefined::ByForm,\n",
    replace: "            _ => self.facts.written,\n",
    unitTests: "js::stmt_facts::tests::a_write_rebuilt_of_another_value_forgets_its_answer",
  },
  {
    name: "rebuilt-forgets-facts",
    breaks: "a statement rebuilt in its place, an `if` made again of its branches, forgets what's known of it: its written `else` dropped, `if (a) { ..; return; } else { b }` printed `if (a) { ..; return; } b`, an `else` of only comments lost (ADR 1167)",
    file: "crates/rust-js-program/src/js/stmt_facts.rs",
    find: "            facts: StmtFacts { written, ..self.facts },\n",
    replace: "            facts: StmtFacts { written, ..StmtFacts::default() },\n",
    unitTests: "js::stmt_facts::tests::a_statement_rebuilt_keeps_its_facts",
  },
];
