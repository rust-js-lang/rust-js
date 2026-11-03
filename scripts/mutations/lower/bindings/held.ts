// Mutations of src/lower/bindings/held.rs (ADRs 0622, 0633).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "held-at-home-in-place",
    breaks:
      "a module's own read of a binding it holds is its path again, `if (React.__SERVER_INTERNALS_.. == null)`, not its `const` (rd659)",
    file: "src/lower/bindings/held.rs",
    find: "            let found = held.iter().find(own)?;\n",
    replace: "            let found = held.iter().find(own).filter(|_| false)?;\n",
    tests: [
      "test/lowering-binding-paths.test.ts",
      "-t",
      "a module that holds a binding reads it by its const|a private use renaming a global binding",
    ],
  },
  {
    name: "bound-global-unexported",
    breaks:
      "a `pub use` of a global binding is its module's `const`, but not exported",
    file: "src/lower/bindings/held.rs",
    find: "            path: read,\n            export,\n",
    replace: "            path: read,\n            export: false,\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "a global binding a module names",
    ],
  },
  {
    name: "bound-global-of-a-method",
    breaks:
      "a method binding, `this.name(..)`, is taken for a global function a module can hold",
    file: "src/lower/bindings/held.rs",
    find: "            JsForm::Call(name) if is_binding(tcx, def_id) && !is_method(tcx, def_id) && js_import(&name).is_none() => {\n",
    replace:
      "            JsForm::Call(name) if is_binding(tcx, def_id) && js_import(&name).is_none() => {\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "a global binding a module names",
    ],
  },
  {
    name: "held-read-in-place",
    breaks:
      "a caller that names a binding through the module that holds it reads it in place, `Object.assign(target, source)`",
    file: "src/lower/bindings/held.rs",
    find: "                self.reads.insert(expr.span, read);\n",
    replace: "                let _ = read;\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "named through the module"],
  },
  {
    name: "held-name-alone-unfollowed",
    breaks:
      "a binding named alone, through its module's `use crate::assign::assign;`, is read in place",
    file: "src/lower/bindings/held.rs",
    find: "        let [only] = segments else { return None };\n",
    replace:
      "        let [only] = segments else { return None };\n        if true {\n            return None;\n        }\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "named through the module"],
  },
  {
    name: "held-name-not-default",
    breaks:
      "a caller imports a binding a module holds as its default and by `pub use` by its name, not as the default its functions are",
    file: "src/lower/bindings/held.rs",
    find: '            (true, _) => Some("default"),\n            (false, true) => Some(&self.name),\n',
    replace:
      '            (_, true) => Some(&self.name),\n            (true, _) => Some("default"),\n',
    tests: ["test/cargo-link.test.ts", "-t", "holds a binding"],
  },
  {
    name: "held-static-unheld",
    breaks:
      "`js::export_default!(REACT_SHARED_INTERNALS)` of a binding's static is refused",
    file: "src/lower/bindings/held.rs",
    find: "        DefKind::Static { .. } if is_binding(tcx, def_id) => {\n",
    replace: "        DefKind::Static { .. } if false => {\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "named through the module"],
  },
  {
    name: "held-reexport-a-const",
    breaks:
      "a `pub use` of a binding through the module that holds it is a `const` of its own, not that module's re-exported",
    file: "src/lower/bindings/held.rs",
    find: "        let through = named_through(tcx, foreign, module, path.segments, binding, modules);\n",
    replace: "        let through: Option<HeldRead> = None;\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "named through the module"],
  },
  {
    name: "held-default-exported-by-name",
    breaks:
      "a library's `pub use` of a binding its default names is `export const assign` too, which no caller reads and upstream's `shared/assign` hasn't",
    file: "src/lower/bindings/held.rs",
    find: "        self.export && !self.default\n",
    replace: "        self.export\n",
    tests: [
      "test/cargo-link.test.ts",
      "-t",
      "a library's module that holds a binding",
    ],
  },
  {
    name: "private-renamed-global-unheld",
    breaks: "a private `use` that renames a global binding, `use js::array::SLICE as slice;`, isn't the module's `const slice = Array.prototype.slice;` (ADR 0622, amended)",
    file: "src/lower/bindings/held.rs",
    find: "        if !export && !named_default && !renamed_global {\n",
    replace: "        if !export && !named_default {\n",
    tests: ["test/lowering-binding-paths.test.ts", "-t", "private use renaming"],
  },
  {
    name: "underscore-use-held",
    breaks: "a `use .. as _;` of a global binding is a `const` of its own, `const { assign, assign: _ } = Object;` (ADR 0622, amended)",
    file: "src/lower/bindings/held.rs",
    find: "            && ident.name != rustc_span::symbol::kw::Underscore\n            && js_import(&read).is_none();\n",
    replace: "            && js_import(&read).is_none();\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "a global binding a module names as its default"],
  },
  {
    name: "library-uses-none",
    breaks:
      "no `use` is taken to name a binding through a library's module, so its `use .. as` keeps the `const`'s name (rd651)",
    file: "src/lower/bindings/held.rs",
    find: "            Some((binding, (from, read.import)))\n",
    replace: "            let _ = (binding, from, read.import);\n            None\n",
    tests: ["test/cargo-link.test.ts", "-t", "holds a binding"],
  },
];
