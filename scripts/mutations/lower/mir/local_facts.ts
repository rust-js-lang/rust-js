// Mutations of src/lower/mir/local_facts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "facts-held-arms-unjoined",
    breaks: "a guard that holds still tests in each alternative of its arm, its graph made without the arms it goes on to (ADR 0790)",
    file: "src/lower/mir/local_facts.rs",
    find: "            graph: cfg::Graph::of(body, &held_arms),\n",
    replace: "            graph: cfg::Graph::of(body, &Default::default()),\n",
    tests: ["test/lowering-labels.test.ts", "-t", "a block the program labeled that ends a case"],
  },
  {
    name: "facts-patterned-none",
    breaks: "a temporary a `let` pattern of one field takes apart is folded, `const getProps = f().get_props`",
    file: "src/lower/mir/local_facts.rs",
    find: "            patterned: self.mir_patterned(body),\n",
    replace: "            patterned: Default::default(),\n",
    tests: ["test/lowering-matches.test.ts", "-t", "pattern of one field"],
  },
  {
    name: "facts-jsx-props-none",
    breaks: "a component's props with a flattened field are flattened before JSX takes them apart",
    file: "src/lower/mir/local_facts.rs",
    find: "            jsx_props: self.mir_jsx_props(body),\n",
    replace: "            jsx_props: Default::default(),\n",
    tests: ["test/jsx-props.test.ts"],
  },
];
