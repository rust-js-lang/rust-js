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
];
