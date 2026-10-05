// Mutations of src/lower/aggregates.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "failing-user-writer-allowed",
    breaks: "a `fmt::Write` of the crate's that fails is compiled, given each `write!`'s text whole where Rust fails at a piece",
    file: "src/lower/aggregates.rs",
    find: "                        .is_some_and(|tr| is_std_def(self.tcx, tr, StdItem::FmtWrite)) =>",
    replace: "                        .is_some_and(|tr| false && is_std_def(self.tcx, tr, StdItem::FmtWrite)) =>",
    tests: ["test/diagnostics.test.ts","-t","a writer of the crate's that fails"],
  },
  {
    name: "replaced-node-default-made",
    breaks: "an update's base makes the default of the children it gives, `C::default()`, which needs a dictionary",
    file: "src/lower/aggregates.rs",
    find: "            let value = if replaced && self.is_node_param(field_ty) {",
    replace: "            let value = if false && replaced && self.is_node_param(field_ty) {",
    tests: ["test/jsx.test.ts","-t","and the component no dictionary"],
  },
  {
    name: "replaced-default-kept",
    breaks: "an update's base keeps the default of a field it gives, `tags: []`, so the base is kept, `base.id`",
    file: "src/lower/aggregates.rs",
    find: "                if replaced && !value.has_effects() {",
    replace: "                if false && replaced && !value.has_effects() {",
    tests: ["test/jsx.test.ts","-t","and the component no dictionary"],
  },
  {
    name: "derived-default-base-lowered",
    breaks: "an update's derived `Default` base is lowered whole, its children's default too",
    file: "src/lower/aggregates.rs",
    find: "                    let base = match self.derived_default_fields(fru.base, ty) {",
    replace: "                    let base = match None::<Vec<(String, Ty<'tcx>)>> {",
    tests: ["test/jsx.test.ts","-t","and the component no dictionary"],
  },
  {
    name: "update-defaults-in-base",
    breaks: "an update's derived defaults are kept in a base, `const base = { n: 0, .. }`, and read from it, `base.items`",
    file: "src/lower/aggregates.rs",
    find: "                            if !kept {",
    replace: "                            if false && !kept {",
    tests: ["test/corpus.test.ts", "-t", "replace_through_mut"],
    snapshots: true,
  },
];
