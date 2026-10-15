// Mutations of src/runtime/pointers.js (ADR 0407).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "ptr-handle-uncached",
    breaks: "a pointer to a place is a new handle each time: two to it differ",
    file: "src/runtime/pointers.js",
    find: "  let handle = handles.get(key);\n  if (handle === undefined) {\n    handle = {\n      of: object,",
    replace: "  let handle;\n  if (handle === undefined) {\n    handle = {\n      of: object,",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "ptr-to-uncached",
    breaks: "a pointer to an object is a new handle each time: two to it differ",
    file: "src/runtime/pointers.js",
    find: "  let handle = $objects.get(object);\n",
    replace: "  let handle;\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "ptr-to-written-whole",
    breaks: "`*p = v` of an object's pointer gives the handle another, leaving the object",
    file: "src/runtime/pointers.js",
    find: "        $assign(object, value);\n",
    replace: "        object = value;\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "ptr-add-from-start",
    breaks: "`p.add(n)` steps from the array's start",
    file: "src/runtime/pointers.js",
    find: "  return $ptr(pointer.of, pointer.at + count);\n",
    replace: "  return $ptr(pointer.of, count);\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "ptr-replace-new",
    breaks: "`ptr::replace` gives the new value",
    file: "src/runtime/pointers.js",
    find: "  const old = pointer.value;\n  pointer.value = value;\n  return old;\n",
    replace: "  pointer.value = value;\n  return value;\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "ptr-swap-one-way",
    breaks: "`ptr::swap` copies one way",
    file: "src/runtime/pointers.js",
    find: "  b.value = held;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "ptr-shown-null",
    breaks: "`{:?}` of a null pointer is rust-js's error, where it's `0x0`",
    file: "src/runtime/pointers.js",
    find: "  if (pointer === null) return \"0x0\";\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "offset-from-backwards",
    breaks: "`p.offset_from(q)` is how far `q` is past `p`",
    file: "src/runtime/pointers.js",
    find: "  return p.at - q.at;\n",
    replace: "  return q.at - p.at;\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_offset_from.rs$"],
  },
];
