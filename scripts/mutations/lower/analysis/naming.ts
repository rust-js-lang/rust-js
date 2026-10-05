// Mutations of src/lower/analysis/naming.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "import-named-over-export",
    breaks: "an import takes the name of the crate's own function, which is renamed, so JS calling it by its Rust name finds none",
    file: "src/lower/analysis/naming.rs",
    find: "    let mut reserved: HashSet<String> = uses.globals.iter().chain(taken.values().flatten()).cloned().collect();\n",
    replace: "    let mut reserved: HashSet<String> = uses.globals.iter().chain(taken.values().flatten().filter(|_| false)).cloned().collect();\n",
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
];
