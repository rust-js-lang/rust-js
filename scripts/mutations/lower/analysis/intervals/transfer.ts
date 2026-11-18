// Mutations of src/lower/analysis/intervals/transfer.rs, the MIR intervals'
// transfer that asks the effects query too (#2112 B step 3a).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/intervals/transfer.rs";
const proofs = ["test/intervals-shadow.test.ts", "-t", "keeps what the rules prove"];

export const mutations: Mutation[] = [
  {
    name: "transfer-grown-forgets-all",
    breaks:
      "a `Vec` the query says grows, a `push`'s, is forgotten whole, so `n < len` past `let n = v.len(); v.push(7)` is lost and `v[n]` checked again",
    file,
    find: "                self.lens.retain(|_, &mut of| of != local);\n",
    replace: "                self.forget(local);\n",
    tests: proofs,
  },
  {
    name: "transfer-items-forget-all",
    breaks:
      "a table whose items the query says are written is forgotten whole, so `board[r][c] = x` loses the table's length and each later index is checked again",
    file,
    find: "            Written::Items => self.rows.retain(|&table, _| table != local),\n",
    replace: "            Written::Items => self.forget(local),\n",
    tests: proofs,
  },
];
