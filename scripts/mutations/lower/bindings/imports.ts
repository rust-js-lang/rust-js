// Mutations of src/lower/bindings/imports.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "namespace-name-unread",
    breaks:
      "`scheduler#* as Scheduler.unstable_now` is the export `* as Scheduler`, not the module itself named `Scheduler`",
    file: "src/lower/bindings/imports.rs",
    find: '    if let Some(named) = path.strip_prefix("* as ") {\n',
    replace:
      '    if let Some(named) = path.strip_prefix("* as ").filter(|_| false) {\n',
    tests: [
      "test/modules.test.ts",
      "-t",
      "a namespace import is named as its link_name",
    ],
  },
  {
    name: "required-namespace-read-as-property",
    breaks:
      "a required binding of a whole module, `ReactFiberErrorDialog#*`, is `require(\"ReactFiberErrorDialog\")[\"*\"]`, `undefined` (#572)",
    file: "src/lower/bindings/imports.rs",
    find: "    match export.1 == js::NAMESPACE {\n",
    replace: "    match false && export.1 == js::NAMESPACE {\n",
    tests: ["test/bindings-require.test.ts", "-t", "a required binding of a whole module"],
  },
  {
    name: "required-binding-specifier-raw",
    breaks: "a binding marked `#[rust_js::require]` in a module below the root requires its JS by its specifier from the crate root's file, `require(\"./helper.cjs\")` in `outer/inner.js`, a file that isn't there (#618, P0)",
    file: "src/lower/bindings/imports.rs",
    find: "    let module = Expr::require(js::ImportFrom::Specifier(export.0.clone()));",
    replace: "    let module = Expr::call(Expr::var(\"require\"), vec![Expr::str(&export.0)]);",
    tests: ["test/bindings-require.test.ts", "-t", "below the root"],
  },
];
