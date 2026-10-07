// Mutations of src/lower/support.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dyn-node-unsupported",
    breaks: "`Box<dyn ReactNode>` is refused, though it's the node itself, and a list of text and elements has no type",
    file: "src/lower/support.rs",
    find: "                    .is_some_and(|t| super::bindings::is_jsx_node(self.tcx, t)) =>",
    replace: "                    .is_some_and(|t| false && super::bindings::is_jsx_node(self.tcx, t)) =>",
    tests: ["test/jsx.test.ts", "-t", "dyn ReactNode is the node"],
  },
];
