// Mutations of src/lower/std_types/alloc.rs (ADR 0417).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "alloc-unrecognized",
    breaks: "`alloc::alloc(layout)` isn't the allocator's, and is refused",
    file: "src/lower/std_types/alloc.rs",
    find: "        \"std::alloc::alloc\" => AllocOp::Alloc,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^allocator.rs$"],
  },
  {
    name: "realloc-unrecognized",
    breaks: "`alloc::realloc` isn't the allocator's, and is refused",
    file: "src/lower/std_types/alloc.rs",
    find: "        \"std::alloc::realloc\" => AllocOp::Realloc,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^allocator.rs$"],
  },
  {
    name: "layout-size-unread",
    breaks: "`layout.size()` is the layout itself",
    file: "src/lower/std_types/alloc.rs",
    find: "            AllocOp::Size => Expr::member(values.next().expect(\"a layout\"), \"size\"),\n",
    replace: "            AllocOp::Size => values.next().expect(\"a layout\"),\n",
    tests: ["test/corpus.test.ts", "-t", "^allocator.rs$"],
  },
  {
    name: "set-len-unrecognized",
    breaks: "`Vec::set_len` isn't the allocator's, and is refused",
    file: "src/lower/std_types/alloc.rs",
    find: "        \"std::vec::Vec::<T, A>::set_len\" => AllocOp::SetLen,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_set_len.rs$"],
  },
];
