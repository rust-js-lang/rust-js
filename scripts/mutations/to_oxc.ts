// Mutations of src/to_oxc.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "pair-without-impl",
    breaks: "a number's `&mut dyn` pair is printed without its `impl`, and `d.impl.bump` throws",
    file: "src/to_oxc.rs",
    find: "dictionary.into_iter().chain([get, set])",
    replace: "dictionary.into_iter().take(0).chain([get, set])",
    tests: ["test/corpus.test.ts", "-t", "dyn_mut"],
  },
  {
    name: "optional-member-plain",
    breaks: "`options?.alternate` is printed `options.alternate`, which throws for a writer given no options",
    file: "src/to_oxc.rs",
    find: "                    IdentifierName::new(SPAN, self.name(property), b),\n                    true,",
    replace: "                    IdentifierName::new(SPAN, self.name(property), b),\n                    false,",
    tests: ["test/corpus.test.ts", "-t", "pretty_debug"],
  },
  {
    name: "spread-argument-unwritten",
    breaks: "a call's `...values` argument isn't written, and printing it panics",
    file: "src/to_oxc.rs",
    find: "            ExprKind::Spread(all) => Argument::new_spread_element(span(a.span), self.expr(all), b),\n",
    replace: "",
    tests: ["test/compiler.test.ts", "-t", "variadic binding"],
  },
];
