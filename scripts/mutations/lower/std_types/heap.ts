// Mutations of src/lower/std_types/heap.rs (ADR 0333).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "heap-drain-kept",
    breaks: "`drain()` leaves its heap full",
    file: "src/lower/std_types/heap.rs",
    find: "            return Ok(Expr::call(Expr::member(heap, \"splice\"), vec![Expr::int(0)]));",
    replace: "            return Ok(Expr::call(Expr::member(heap, \"slice\"), vec![Expr::int(0)]));",
    tests: ["test/corpus.test.ts", "-t", "heap_methods"],
  },
  {
    name: "heap-retain-given-no-drop",
    breaks: "a heap's `retain` of items with a destructor is given no drop",
    file: "src/lower/std_types/heap.rs",
    find: "            values.extend(self.std_drop_function(item, span)?);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^heap_destructors.rs$"],
  },
];
