// Mutations of src/lower/named_constants.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "named-text-left-its-literal",
    breaks:
      "a `match` of `&str` constants tests their text once tidy's done, `case \"fulfilled\":`, not the constant the program wrote, `case INITIALIZED:` (ADR 0590, amended)",
    file: "src/lower/named_constants.rs",
    find: "    if named.is_empty() {\n        return;\n    }\n",
    replace: "    if true {\n        return;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^str_const_patterns.rs$"],
    snapshots: true,
  },
  {
    name: "named-text-before-tidy",
    breaks:
      "a `&str` constant a pattern names is its name to tidy, which takes no name for a value never `null`: an or-pattern's tests keep a labeled block and `value != null`, `bb10: { if (value != null) { .. switch (value) {..} } }` (ADR 0590, amended)",
    file: "src/lower/named_constants.rs",
    find: "        Ok(Some(Expr {\n            span: at,\n            ..text.clone()\n        }))\n",
    replace: "        Ok(Some(self.named_texts().last().expect(\"noted\").2.clone()))\n",
    tests: ["test/corpus.test.ts", "-t", "^(str_const_patterns|presence_said_by_chain).rs$"],
    snapshots: true,
  },
  {
    name: "named-text-of-a-written-eq",
    breaks:
      "a `==` the program writes in an arm of a `match` of `&str` constants names its literal by the constant too, `other === INITIALIZED` of `other == \"fulfilled\"` (ADR 0590, amended)",
    file: "src/lower/named_constants.rs",
    find: "        if at.is_none() || !self.frame.facts.constant_patterns.pattern_at(span) {\n",
    replace: "        if at.is_none() {\n",
    tests: ["test/corpus.test.ts", "-t", "^str_const_patterns.rs$"],
    snapshots: true,
  },
];
