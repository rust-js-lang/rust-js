// Mutations of src/runtime/alloc.js (ADR 0417).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "alloc-bytes-opaque",
    breaks: "an allocation's bytes written through its `*mut u8` throw, `alloc` giving a view of no bytes, as semver's identifiers did (#1099)",
    file: "src/runtime/alloc.js",
    find: "  return $byteView({ of: slots, at: 0, stride: 1, typed: $ptr(slots, 0), allocated: true });\n",
    replace: "  return $ptrBytes($ptr(slots, 0));\n",
    tests: ["test/corpus.test.ts", "-t", "^alloc_bytes_written_and_read.rs$"],
  },
  {
    name: "alloc-vec-of-wider-bytes",
    breaks: "`Vec::from_raw_parts` of an allocation's bytes cast to `*mut u32` is a `Vec` of its bytes (#1099)",
    file: "src/runtime/alloc.js",
    find: "  if (pointer?.bytes !== undefined && (pointer.width !== 1 || pointer.signed !== false || pointer.stride !== 1)) {\n",
    replace: "  if (false) {\n",
    tests: ["test/runtime-pointers.test.ts", "-t", "allocation's bytes and its items"],
  },
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
  {
    name: "layout-size-unformatted",
    breaks: "a `Layout`'s `size` ignores the options its derived `Debug` gives it, `{:5?}`, `{:x?}` (#1728)",
    file: "src/runtime/alloc.js",
    find: "  const shown = $formatted($debugInt(String(size), size.toString(16), options), options, true);\n",
    replace: "  const shown = String(size);\n",
    tests: ["test/corpus.test.ts", "-t", "^layout_debug_pretty.rs$"],
  },
  {
    name: "write-layout-size-unformatted",
    breaks: "a `Layout`'s `size` written into a writer ignores the options its derived `Debug` gives it, `{:5?}` (#1728)",
    file: "src/runtime/alloc.js",
    find: "  const integer = (g, n) => $writeDebugInteger(g, n, n.toString(16), n.toString(16).toUpperCase(), options);\n",
    replace: "  const integer = (g, n) => g.write(String(n));\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_rest.rs$"],
  },
];
