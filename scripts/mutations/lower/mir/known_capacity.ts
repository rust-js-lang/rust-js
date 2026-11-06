// Mutations of src/lower/mir/known_capacity.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "reserve-forgets-capacity",
    breaks: "a `reserve` before an `extend` is taken to change its capacity unknowingly, so a `Vec` reserved first is refused (#849)",
    file: "src/lower/mir/known_capacity.rs",
    find: "                let reserves = matches!(capacity_op(self.tcx, def_id), Some(CapacityOp::Reserve { .. }));\n",
    replace: "                let reserves = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^extend_asks_hint_as_it_fills.rs$"],
  },
  {
    name: "new-vec-capacity-unknown",
    breaks: "a `Vec::new()` isn't a maker whose capacity is known, so its `extend` by the crate's own hint is refused (#849)",
    file: "src/lower/mir/known_capacity.rs",
    find: "            Some(Std::VecNew | Std::VecMacro | Std::FromElem)\n",
    replace: "            Some(Std::VecMacro | Std::FromElem)\n",
    tests: ["test/corpus.test.ts", "-t", "^extend_asks_hint_as_it_fills.rs$"],
  },
  {
    name: "any-collect-capacity-known",
    breaks: "a collect of what isn't `TrustedLen`, whose capacity std's growth by its hints decides, is taken as known, its length (#849)",
    file: "src/lower/mir/known_capacity.rs",
    find: "                is_trusted_len(self.tcx, self.typing_env, iter)\n",
    replace: "                true\n",
    tests: ["test/diagnostics.test.ts", "-t", "an extend of a Vec collected of what"],
  },
];
