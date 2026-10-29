// Mutations of src/runtime/btree.js (ADR 0404, amended; #237).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "btree-map-never-sorted-again",
    breaks: "a B-tree's map added to out of its keys' order is read by JS in the order it was added, `[[3, \"bronze\"], [1, \"gold\"], ..]` (#237)",
    file: "src/runtime/btree.js",
    find: "      if (this.#sorted) return;\n      const entries",
    replace: "      return;\n      const entries",
    tests: ["test/runtime-package.test.ts", "-t", "reads as a Map"],
  },
  {
    name: "btree-map-added-before-its-last-unmarked",
    breaks: "a key added before a B-tree's last is taken to keep it in order, so JS reads it where it was added (#237)",
    file: "src/runtime/btree.js",
    find: "this.size > 0 && cmp(this.#last, key) > 0",
    replace: "false",
    tests: ["test/runtime-package.test.ts", "-t", "reads as a Map"],
  },
  {
    name: "btree-set-never-sorted-again",
    breaks: "a B-tree's set added to out of its keys' order is read by JS in the order it was added (#237)",
    file: "src/runtime/btree.js",
    find: "      if (this.#sorted) return;\n      const items",
    replace: "      return;\n      const items",
    tests: ["test/runtime-package.test.ts", "-t", "reads as a Map"],
  },
  {
    name: "btree-emptied-unsorted-throws",
    breaks: "a B-tree emptied before it's sorted again throws a `TypeError` where JS reads it (#237)",
    file: "src/runtime/btree.js",
    find: "      this.#last = entries.at(-1)?.[0];",
    replace: "      this.#last = entries[entries.length - 1][0];",
    tests: ["test/runtime-package.test.ts", "-t", "reads as a Map"],
  },
  {
    name: "btree-order-private-method",
    breaks: "a B-tree's reordering is a private method, `#order() {..}`, which React's Babel rejects, \"Class private methods are not enabled\", so no React suite loads (ADR 0919, amended)",
    file: "src/runtime/btree.js",
    find: "    #order = () => {\n      if (this.#sorted) return;\n      const entries",
    replace: "    #order() {\n      if (this.#sorted) return;\n      const entries",
    tests: ["test/runtime-transpiled.test.ts", "-t", "no private method"],
  },
];
