// Mutations of src/runtime/scan.js (ADR 0517).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "scan-past-none",
    breaks: "`scan` goes on past its first `None`",
    file: "src/runtime/scan.js",
    find: "    if (next === undefined) return;\n",
    replace: "    if (next === undefined) continue;\n",
    tests: ["test/corpus.test.ts", "-t", "^scan_lazy_drops.rs$"],
  },
];
