// Mutations of src/lower/analysis/naming.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "import-named-over-export",
    breaks: "an import takes the name of the crate's own function, which is renamed, so JS calling it by its Rust name finds none",
    file: "src/lower/analysis/naming.rs",
    find: "            let items: Vec<&HashSet<String>> = if importers.is_empty() {\n                taken.values().collect()",
    replace: "            let items: Vec<&HashSet<String>> = if importers.is_empty() {\n                Vec::new()",
    tests: ["test/crates.test.ts", "-t", "two crates: same_name"],
  },
  {
    name: "component-default-import-unnamed",
    breaks: "\`next/image#default\` of \`fn Image\` is \`image\`, which JSX takes for an element",
    file: "src/lower/analysis/naming.rs",
    find: "                [only] if tcx.def_kind(**only) == DefKind::Fn => Some(bindings::fn_name(tcx, **only)),",
    replace: "                [only] if false && tcx.def_kind(**only) == DefKind::Fn => Some(bindings::fn_name(tcx, **only)),",
    tests: ["test/next.test.ts", "-t", "build builds"],
  },
  {
    name: "import-named-around-every-module",
    breaks: "an import is renamed for another module's item, `import { join as join$1 }` for a `fn join` elsewhere",
    file: "src/lower/analysis/naming.rs",
    find: "                importers.iter().filter_map(|m| taken.get(m)).collect()",
    replace: "                taken.values().collect()",
    tests: ["test/snapshots.test.ts", "-t", "imports"],
    snapshots: true,
  },
  {
    name: "tested-variant-unimported",
    breaks: "a `match` of an element calls `isValidElement`, which the module doesn't import",
    file: "src/lower/analysis/naming.rs",
    find: "        for arm in body.thir.arms.iter() {\n            tested(&arm.pattern);\n        }\n",
    replace: "",
    tests: ["test/jsx.test.ts", "-t", "look inside their children"],
  },
];
