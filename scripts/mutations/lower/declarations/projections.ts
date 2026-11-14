// Mutations of src/lower/declarations/projections.rs (ADR 0093): a binding
// trait's associated type declared through what its `#[rust_js::types]`
// writes, and the key a bound of it constrains (#1817).
import type { Mutation } from "../../../mutations";

const tests = ["test/webapi-event-map.test.ts", "-t", "a binding trait's projection"];

export const mutations: Mutation[] = [
  {
    name: "bound-constrains-no-key",
    breaks:
      "a bound `Target: Listen<Name>` puts no constraint on `Name`, so `on<Target, Name>(.., name: Name, ..)` takes any name and `EventMapOf<Target>[Name]` isn't a type tsc reads",
    file: "src/lower/declarations/projections.rs",
    find: "                        of.push(constraint);\n",
    replace: "",
    tests,
  },
  {
    name: "projection-of-the-trait-alone",
    breaks:
      "a projection's type is read without its `Self` and arguments, so `EventMapOf<Self>[E]` names no target and no key",
    file: "src/lower/declarations/projections.rs",
    find: "        Some(self.written(self.tcx.parent(assoc), &declared, args))",
    replace: "        Some(self.written(self.tcx.parent(assoc), &declared, ty::GenericArgs::empty()))",
    tests,
  },
];
