// Mutations of src/runtime/json_visit.js (ADR 0459).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-visit-number-unconverted",
    breaks: "a visitor's `visit_u64` is given a JS number, where a `u64` is a BigInt, and JS throws",
    file: "src/runtime/json_visit.js",
    find: "      return $jsonCodec(visit)(n.kind === \"f\" ? Number(n.value) : BigInt(n.value));\n",
    replace: "      return $jsonCodec(visit)(n.value);\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
  {
    name: "json-visit-signed-as-unsigned",
    breaks: "a negative number is given to a visitor's `visit_u64`",
    file: "src/runtime/json_visit.js",
    find: "      const visit = n.kind === \"u\" ? u64 : n.kind === \"i\" ? i64 : f64;\n",
    replace: "      const visit = n.kind === \"f\" ? f64 : u64;\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
  {
    name: "json-visit-copied-as-borrowed",
    breaks: "an escaped string, copied, is given to a visitor's `visit_borrowed_str`",
    file: "src/runtime/json_visit.js",
    find: "      const visit = borrowed ? (borrowedStr ?? str) : str;\n",
    replace: "      const visit = borrowedStr ?? str;\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
];
