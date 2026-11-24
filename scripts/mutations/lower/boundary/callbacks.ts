// Mutations of crates/rust-js-lower/src/boundary/callbacks.rs (ADR 1195).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/boundary/callbacks.rs";

export const mutations: Mutation[] = [
  {
    name: "callback-result-unchecked",
    breaks: "what a callback JS gives returns isn't checked, so a `u32` discount of -100 reaches Rust (#2403)",
    file,
    find: "            _ => self.checker(output),",
    replace: "            _ => None,",
    tests: ["test/boundary-callbacks.test.ts", "-t", "a callback whose result is checked"],
  },
  {
    name: "dyn-fn-not-a-callback",
    breaks: "a `dyn Fn` JS gives isn't checked a function, so a string reaches Rust's call of it (#2403)",
    file,
    find: "                Contract::DynFn { output, .. } => Some(output),",
    replace: "                Contract::DynFn { .. } => None,",
    tests: ["test/boundary-callbacks.test.ts", "-t", "a value that isn't a function"],
  },
];
