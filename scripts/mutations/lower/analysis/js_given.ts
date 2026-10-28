// Mutations of src/lower/analysis/js_given.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "js-given-copy-unrouted",
    breaks: "a call of a function JS calls that gives it `()` calls its own JS, which unboxes, so `relay(()).is_some()` is `false`",
    file: "src/lower/analysis/js_given.rs",
    find: "        if main.unboxed.is_disjoint(&nullish) && js.is_subset(&main.js) {",
    replace: "        if true {",
    tests: ["test/corpus.test.ts", "-t", "js_given_copies"],
  },
  {
    name: "js-given-facing-boxed",
    breaks: "a function JS calls takes its type parameters as its crate's Rust gives them, so `pick(undefined, true)` gives JS `{ $someNone: 0 }`",
    file: "src/lower/analysis/js_given.rs",
    find: "                .filter(|&id| js_facing(tcx, recognition.foreign, id))",
    replace: "                .filter(|&id| false && js_facing(tcx, recognition.foreign, id))",
    tests: ["test/compiler.test.ts", "-t", "an Option of a generic T"],
  },
  {
    name: "js-given-passed-on-boxed",
    breaks: "what JS gave, passed on to the module's own function, is taken as never nullish, so `relay`'s JS calls `wrapped`, which boxes, and `relay(undefined)` gives JS a box",
    file: "src/lower/analysis/js_given.rs",
    find: "        ty::Param(param) if params.js.contains(&param.index) => Given::Js,",
    replace: "        ty::Param(param) if params.js.contains(&param.index) => Given::Defined,",
    tests: ["test/compiler.test.ts", "-t", "an Option of a generic T"],
  },
  {
    name: "mir-initializer-asked-visibility",
    breaks: "MIR asks a `const { .. }` block's visibility, which rustc has none of",
    file: "src/lower/analysis/js_given.rs",
    find: "    matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn)\n        && tcx.visibility(id)",
    replace: "    tcx.visibility(id)",
    tests: ["test/corpus.test.ts","-t","const_block_values"],
  },
  {
    name: "instance-exported",
    breaks: "a library's function the crate compiles itself is exported, `export function createContext`, as if JS or another crate could call it (ADR 0914)",
    file: "src/lower/analysis/js_given.rs",
    find: "        && !foreign.is_instance(id)\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "Copy bound takes a copy only where one is given"],
  },
];
