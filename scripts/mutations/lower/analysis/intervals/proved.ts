// Mutations of src/lower/analysis/intervals/proved.rs, what the MIR
// intervals prove of a body at its fixed point (ADR 1137).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/intervals/proved.rs";
const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-exact-unproved",
    breaks: "no operator is proved in its type's range, so a loop's counter below its bound is logged as one THIR alone leaves out",
    file,
    find: "                    proofs.exact.insert(Location {\n                        block,\n                        statement_index: at,\n                    });\n",
    replace: "                    let _ = at;\n",
    tests,
  },
  {
    name: "intervals-bounds-unproved",
    breaks: "no bounds check is proved by MIR, so a slice's index under `i < s.len()` is logged as one THIR alone leaves out",
    file,
    find: "                proofs.bounds.insert(block);\n",
    replace: "                let _ = block;\n",
    tests,
  },
  {
    name: "intervals-casts-unproved",
    breaks: "no integer cast is proved in its target's range, so `n as u8` under `n < 200` is `n & 255` (#1672 M3)",
    file,
    find: "                    && held_by_a_number(operand.ty(&*self.body, self.tcx))\n",
    replace: "                    && false\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "an integer cast MIR's intervals show"],
  },
  {
    name: "intervals-cast-to-a-bigint-kept",
    breaks: "a cast to a 64-bit integer whose value fits is kept as it is, so `n as u64` is the number `n`, not `BigInt(n)`, and mixing it with a `BigInt` throws",
    file,
    find: "                    && held_by_a_number(*to)\n",
    replace: "",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "an integer cast MIR's intervals show"],
  },
];
