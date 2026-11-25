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
  {
    name: "offered-type-leaf-flipped",
    breaks: "a `bool` is recorded as a `u8`, a leaf of the type tree flipped, which both nets read: the export net gives a function of a `bool` numbers, which native Rust refuses, and declarations-conformance's caller gives it numbers its `.d.ts` refuses (#2573)",
    file,
    find: "        ty::Bool => OfferedType::Bool,\n",
    replace: "        ty::Bool => OfferedType::Int { rust: \"u8\".to_string() },\n",
    tests: ["test/export-net.test.ts", "test/declarations-conformance.test.ts", "-t", "export net: one_crate|each exported function's TypeScript caller"],
  },
  {
    name: "offered-dyn-fn-unread",
    breaks: "a `dyn Fn(u32) -> u32` is recorded as any other type, not the function it is, so a caller of an `Option<&dyn Fn(u32)>` is given no function (#2573)",
    file,
    find: "                && tcx.is_fn_trait(principal.def_id()) =>\n",
    replace: "                && tcx.is_fn_trait(principal.def_id())\n                && false =>\n",
    tests: ["test/declarations-conformance.test.ts", "-t", "a dyn Fn is read as the function it is"],
  },
  {
    name: "offered-symbol-field-named",
    breaks: "a struct of a field a JS symbol names, a binding's, is asked its fields' text names, which it has none of, and the compiler panics (ADR 1051, #2573)",
    file,
    find: "crate::bindings::field_key(tcx, field).name().is_none())",
    replace: "crate::bindings::field_key(tcx, field).name().is_none() && false)",
    tests: ["test/docs.test.ts", "-t", "each guide page's output fences are what the compiler makes"],
  },
  {
    name: "offered-symbol-keyed-unsaid",
    breaks: "a struct of a field a JS symbol names records no fields and no reason, its silence read as no fields at all, the hole the lead asked be said (#2573)",
    file,
    find: "Some(rust_js_program::offered::UnofferedFields::SymbolKeyed)",
    replace: "None",
    tests: ["test/declarations-conformance.test.ts", "-t", "a struct of a symbol-keyed field says its fields aren't offered"],
  },
];
