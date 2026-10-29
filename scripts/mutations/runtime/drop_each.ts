// Mutations of src/runtime/drop_each.js (ADR 0098, #416).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "drop-items-stop-at-a-throw",
    breaks: "the items after one whose destructor panics are never dropped, `$dropItems` stopping at its throw (#416)",
    file: "src/runtime/drop_each.js",
    find: "    } catch (error) {\n      threw = true;\n      thrown = error;\n    }\n",
    replace: "    } catch (error) {\n      throw error;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_parts_past_a_panic.rs$"],
  },
];
