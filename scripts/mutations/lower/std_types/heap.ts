// Mutations of src/lower/std_types/heap.rs (ADR 0333).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "heap-pop-unboxed",
    breaks:
      "a `BinaryHeap`'s `pop()` of items that could look like `None` gives its top unboxed: a `None` it holds is an empty heap's `None` (#318)",
    file: "src/lower/std_types/heap.rs",
    find: "        if op == HeapOp::Pop && boxed {\n",
    replace: "        if op == HeapOp::Pop && boxed && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^heap_pop_of_a_param.rs$"],
  },
  {
    name: "heap-drain-kept",
    breaks: "`drain()` leaves its heap full",
    file: "src/lower/std_types/heap.rs",
    find: '            return Ok(Expr::call(Expr::member(heap, "splice"), vec![Expr::int(0)]));',
    replace:
      '            return Ok(Expr::call(Expr::member(heap, "slice"), vec![Expr::int(0)]));',
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
