// Mutations of src/lower/bindings/whole.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "whole-bounds-unordered",
    breaks: "`#[rust_js::whole = \"5..=1\"]`, bounds out of order, is taken",
    file: "src/lower/bindings/whole.rs",
    find: "    (lo <= hi).then_some((lo as f64, hi as f64))\n",
    replace: "    Some((lo as f64, hi as f64))\n",
    tests: ["test/webapi-bindings.test.ts", "-t", "WebIDL integer"],
  },
  {
    name: "whole-position-taken",
    breaks: "`#[rust_js::position]`, which `whole` replaced, is taken without a word",
    file: "src/lower/bindings/whole.rs",
    find: "    if tcx.get_attrs_by_path(def_id, &position).next().is_some() {\n",
    replace: "    if false {\n",
    tests: ["test/webapi-bindings.test.ts", "-t", "WebIDL integer"],
  },
  {
    name: "whole-own-function-taken",
    breaks: "`#[rust_js::whole]` on the crate's own function, not a binding, is taken",
    file: "src/lower/bindings/whole.rs",
    find: "        && is_binding(tcx, def_id)\n",
    replace: "",
    tests: ["test/webapi-bindings.test.ts", "-t", "WebIDL integer"],
  },
];
