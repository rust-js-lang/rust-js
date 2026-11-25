// Mutations of crates/rust-js-lower/src/offered.rs (#2481, the export net).
import type { Mutation } from "../../mutations";

const file = "crates/rust-js-lower/src/offered.rs";

export const mutations: Mutation[] = [
  {
    name: "offered-namespace-unwalked",
    breaks: "a module re-exported is a namespace of nothing: what it offers in turn, `s::d::seed` of `pub use sub as s;`, isn't the crate's, so a path the JS drops of it isn't found missing",
    file,
    find: "                walk(tcx, foreign, (id, &within), (&rust_path, &js_path), paths);\n",
    replace: "",
    tests: ["test/export-net.test.ts", "-t", "the paths a crate offers JS are its root's and each re-exported module's"],
  },
  {
    name: "offered-alias-ignored",
    breaks: "a `pub use` is read by JS by its item's own name, not the name the use gives it, so the net asks the JS for a path it doesn't have",
    file,
    find: "                crate::bindings::alias_name(tcx, used, name)\n",
    replace: "                crate::bindings::fn_name(tcx, id)\n",
    tests: ["test/export-net.test.ts", "-t", "export net: reexports_alone"],
  },
];
