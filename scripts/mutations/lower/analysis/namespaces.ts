// Mutations of src/lower/analysis/namespaces.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "namespace-through-keyword",
    breaks: "a path through `super` alone, `super::extract_events(1)`, is read through its module's namespace, `Plugin.extractEvents(1)`, though it names no module of its own (rd114)",
    file: "src/lower/analysis/namespaces.rs",
    find: "    let through = listed && !module.ident.is_path_segment_keyword() && tcx.parent(function) == named;\n",
    replace: "    let through = listed && tcx.parent(function) == named;\n",
    tests: ["test/modules.test.ts", "-t", "read through its namespace"],
  },
  {
    name: "namespace-through-reexport",
    breaks: "a function a module re-exports, `plugins::doubled()` of `pub use crate::shared::doubled`, is read through the namespace of the module that has it, `Shared.doubled`, where the path named another (rd114)",
    file: "src/lower/analysis/namespaces.rs",
    find: "    let through = listed && !module.ident.is_path_segment_keyword() && tcx.parent(function) == named;\n",
    replace: "    let through = listed && !module.ident.is_path_segment_keyword();\n",
    tests: ["test/modules.test.ts", "-t", "read through its namespace"],
  },
  {
    name: "namespace-of-a-value-path",
    breaks: "a function given as a value through its module, `.map(plugin::doubled)`, is its named import while its calls are the namespace's (rd114)",
    file: "src/lower/analysis/namespaces.rs",
    find: "            hir::ExprKind::Path(ref qpath) => Some(qpath),\n            hir::ExprKind::Call(callee, _) => match callee.kind {\n",
    replace: "            hir::ExprKind::Call(callee, _) => match callee.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "^module_path_namespace.rs$"],
    snapshots: true,
  },

];
