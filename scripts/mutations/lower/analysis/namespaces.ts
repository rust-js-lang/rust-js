// Mutations of src/lower/analysis/namespaces.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "namespace-through-keyword",
    breaks: "a path through `super` alone, `super::extract_events(1)`, is read through its module's namespace, `Plugin.extractEvents(1)`, though it names no module of its own (rd114)",
    file: "src/lower/analysis/namespaces.rs",
    find: "        && !module.ident.is_path_segment_keyword()\n",
    replace: "",
    tests: ["test/modules.test.ts", "-t", "read through its namespace"],
  },
  {
    name: "namespace-through-reexport",
    breaks: "a function a module re-exports, `plugins::doubled()` of `pub use crate::shared::doubled`, is read through the namespace of the module that has it, `Shared.doubled`, where the path named another (rd114)",
    file: "src/lower/analysis/namespaces.rs",
    find: "        && tcx.parent(function) == named\n",
    replace: "",
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

  {
    name: "namespace-of-functions-alone",
    breaks: "a thread-local, a `static` or a `const` read through its module path, `plugin::CALLS.get()`, is its named import beside the module's namespace, `import { calls }` and `import * as Plugin`, two import forms of one module (rd114a, ADR 1018)",
    file: "src/lower/analysis/namespaces.rs",
    find: "    let Res::Def(DefKind::Fn | DefKind::Const { .. } | DefKind::Static { .. }, function) = path.res else {\n",
    replace: "    let Res::Def(DefKind::Fn, function) = path.res else {\n",
    tests: ["test/corpus.test.ts", "-t", "^module_path_namespace.rs$"],
    snapshots: true,
  },
  {
    name: "namespace-of-a-callable-value",
    breaks: "a thread-local of a `with_this` function read through its module, `report::REPORT.with(|report| report())`, is `Report.report()`, which gives it the namespace object as its `this`, where Rust gives it `None` (rd114a, ADR 1018)",
    file: "src/lower/analysis/namespaces.rs",
    find: "        && !holds_a_callable(tcx, function);\n",
    replace: "        && (true || !holds_a_callable(tcx, function));\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "called with no this"],
  },
  {
    name: "library-root-namespace-by-file",
    breaks: "a library's root's namespace is named by its JS file, `import * as Lib from \"../dep/lib.js\"`, not its crate, `Dep` (rd114c, ADR 1018)",
    file: "src/lower/analysis/namespaces.rs",
    find: "        Some((from, _)) if !module.is_crate_root() => {\n",
    replace: "        Some((from, _)) => {\n",
    tests: ["test/cross-crate-items.test.ts", "-t", "read through its namespace"],
  },
];
