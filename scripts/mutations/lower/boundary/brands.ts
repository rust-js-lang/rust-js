// Mutations of crates/rust-js-lower/src/boundary/brands.rs (ADR 1196).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/boundary/brands.rs";

export const mutations: Mutation[] = [
  {
    name: "unreached-value-branded",
    breaks: "a value that never leaves Rust, a guard holding a borrow, is branded, a WeakSet's add for nothing (#2403, ADR 1196)",
    file,
    find: "            ty::Adt(adt, _) if !self.may_reach_js(adt.did()) => made,",
    replace: "            ty::Adt(adt, _) if false && !self.may_reach_js(adt.did()) => made,",
    tests: ["test/boundary-brands.test.ts", "-t", "never leaves Rust"],
  },
  {
    name: "unmarked-brand",
    breaks: "a brand is a call around its literal, not a mark on it, so a drop guard of a sealed type is made, `const _exit = [strict]` (#2403, ADR 1196)",
    file,
    find: "            rust_js_program::js::ExprKind::Object(_, layout) => layout.brand = Some(key),",
    replace: "            rust_js_program::js::ExprKind::Object(_, layout) if false => layout.brand = Some(key),",
    tests: ["test/boundary-brands.test.ts"],
  },
];
