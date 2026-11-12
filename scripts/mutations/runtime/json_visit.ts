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
    find: "      const visit = kind === \"borrowed\" ? (borrowedStr ?? str) : kind === \"owned\" ? (string ?? str) : str;\n",
    replace: "      const visit = borrowedStr ?? str;\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
  {
    name: "json-expected-made-once-up-front",
    breaks:
      "what a visitor expects is its `expecting()` called once as the read starts, so a read that succeeds runs it, a panic among what it does (#1544)",
    file: "src/runtime/json_visit.js",
    find: "  return { toString: expecting };\n",
    replace: "  return expecting();\n",
    tests: ["test/serde-protocol.test.ts", "-t", "expecting is asked only where an error says it"],
  },
  {
    name: "json-visit-owned-as-str",
    breaks: "a string given whole, a `Value`'s, goes to the visitor's `visit_str`, though it writes `visit_string` (#1546)",
    file: "src/runtime/json_visit.js",
    find: "kind === \"owned\" ? (string ?? str) : str;\n",
    replace: "kind === \"owned\" ? str : str;\n",
    tests: ["test/serde-protocol.test.ts", "-t", "a string is given to a visitor as serde_json gives it"],
  },
  {
    name: "json-visit-borrowed-as-str",
    breaks: "a string borrowed from the text goes to the visitor's `visit_str`, though it writes `visit_borrowed_str` (#1546)",
    file: "src/runtime/json_visit.js",
    find: "const visit = kind === \"borrowed\" ? (borrowedStr ?? str) : ",
    replace: "const visit = kind === \"borrowed\" ? str : ",
    tests: ["test/serde-protocol.test.ts", "-t", "a string is given to a visitor as serde_json gives it"],
  },
  {
    name: "json-visit-transient-as-owned",
    breaks: "a string the text unescaped into a buffer goes to the visitor's `visit_string`, where serde_json gives it by reference, `visit_str` (#1546)",
    file: "src/runtime/json_visit.js",
    find: "kind === \"owned\" ? (string ?? str) : str;\n",
    replace: "kind === \"owned\" ? (string ?? str) : (string ?? str);\n",
    tests: ["test/serde-protocol.test.ts", "-t", "a string is given to a visitor as serde_json gives it"],
  },
];
