// Mutations of crates/rust-js-lower/src/noted/made.rs, the `const`s MIR's
// lowering made of a body, a held handle among them (#2429).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [

  {
    name: "handle-alias-unnoted",
    breaks: "a variable made another name of a held handle isn't noted, so what's lent of its `value` is a handle on it that assigns its place as it's given (#2429)",
    file: "crates/rust-js-lower/src/noted/made.rs",
    find: "            rust_js_program::js::ExprKind::Var(held) => self.handles.get(held).cloned(),\n",
    replace: "            rust_js_program::js::ExprKind::Var(_) => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_lent_through_what_holds_it.rs$"],
  },
  {
    name: "held-ok-unnoted",
    breaks: "an `Ok` made of a held handle, `try_borrow_mut()`'s, isn't noted, so its payload is read as `_6._0` and lends a handle that assigns the variable holding the class's object (#2429)",
    file: "crates/rust-js-lower/src/noted/made.rs",
    find: "            self.oks.insert(name.clone(), payload.clone());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_lent_through_what_holds_it.rs$"],
  },
];
