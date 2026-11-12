// Mutations of src/runtime/array_map.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "array-map-left-undropped",
    breaks: "an array's `map` that panics part-way drops what was made but not the items it hasn't mapped yet (#1586)",
    file: "src/runtime/array_map.js",
    find: "      for (let i = at + 1; i < items.length; i++) dropLeft(items[i]);\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","^array_map_panicking_part_way_drops_both_sides.rs$"],
  },
  {
    name: "array-map-made-undropped",
    breaks: "an array's `map` that panics part-way drops the items it hasn't mapped but not what it made (#1586)",
    file: "src/runtime/array_map.js",
    find: "      for (const item of made) dropMade(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","^array_map_panicking_part_way_drops_both_sides.rs$"],
  },
  {
    name: "array-map-mapped-item-dropped-again",
    breaks: "an array's `map` that panics drops the item being mapped too, which the closure has dropped already (#1586)",
    file: "src/runtime/array_map.js",
    find: "for (let i = at + 1; i < items.length; i++)",
    replace: "for (let i = at; i < items.length; i++)",
    tests: ["test/corpus.test.ts","-t","^array_map_panicking_part_way_drops_both_sides.rs$"],
  },
];
