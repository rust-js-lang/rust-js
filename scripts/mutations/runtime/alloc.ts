// Mutations of src/runtime/alloc.js (ADR 0417).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "vec-from-raw-untruncated",
    breaks: "`Vec::from_raw_parts(p, len, cap)` keeps the allocation's slots past `len`",
    file: "src/runtime/alloc.js",
    find: "  $setLen(pointer.of, length);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^allocator.rs$"],
  },
  {
    name: "layout-align-unchecked",
    breaks: "`Layout::from_size_align` takes an alignment that isn't a power of two",
    file: "src/runtime/alloc.js",
    find: "  const ok = align > 0 && (align & (align - 1)) === 0 && size <= 2 ** 31 - align;\n",
    replace: "  const ok = size <= 2 ** 31 - align;\n",
    tests: ["test/corpus.test.ts", "-t", "^allocator.rs$"],
  },
];
