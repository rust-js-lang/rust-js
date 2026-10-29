// Mutations of src/runtime/fold_of.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "collect-dropping-keeps-collected",
    breaks: "a `collect` a panic stops leaves what it had collected undropped (#422)",
    file: "src/runtime/fold_of.js",
    find: "    if (drop !== undefined) $dropItems(items, drop);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_iteration_drops.rs$"],
  },
  {
    name: "collect-rest-first-ignored",
    breaks: "a `collect` std does otherwise than in place, a panic stopping it, drops what it collected before what's left, where std drops what's left first (#423)",
    file: "src/runtime/fold_of.js",
    find: "  const [first, then] = restFirst ? [rest, collected] : [collected, rest];",
    replace: "  const [first, then] = [collected, rest];",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "fold-to-array-unwinds-nothing",
    breaks: "a compiled std struct's collect, a `zip`'s, a panic stopping it, drops neither what it collected nor what's left (#423)",
    file: "src/runtime/fold_of.js",
    find: "  } catch (error) {\n    $collectUnwound(it, items, drop, restFirst);\n    throw error;\n  }\n}",
    replace: "  } catch (error) {\n    throw error;\n  }\n}",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
];
