// Mutations of src/runtime/memchr.js (ADR 0529).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "memchr-needles-past-first",
    breaks: "`memchr2` and `memchr3` look for their first needle alone",
    file: "src/runtime/memchr.js",
    find: "    if (needles.includes(haystack[i])) return i;\n  }\n  return undefined;\n}\n\n// memchr's `memrchr`",
    replace: "    if (haystack[i] === needles[0]) return i;\n  }\n  return undefined;\n}\n\n// memchr's `memrchr`",
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches run"],
  },
  {
    name: "memrchr-skips-first-byte",
    breaks: "`memrchr2` never looks at the slice's first byte",
    file: "src/runtime/memchr.js",
    find: "  for (let i = haystack.length - 1; i >= 0; i--) {\n",
    replace: "  for (let i = haystack.length - 1; i > 0; i--) {\n",
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches run"],
  },
  {
    name: "memchr-iter-past-end",
    breaks: "a `Memchr` stepped from the front finds what its back already gave",
    file: "src/runtime/memchr.js",
    find: "        if (i < 0 || i >= it.end) {\n",
    replace: "        if (i < 0) {\n",
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches run"],
  },
  {
    name: "memchr-clone-restarts",
    breaks: "a clone of a `Memchr` starts again from the slice's ends, not where it is",
    file: "src/runtime/memchr.js",
    find: "  it[$cloneKey] = () => $memchrIter(haystack, needles, it.at, it.end);\n",
    replace: "  it[$cloneKey] = () => $memchrIter(haystack, needles);\n",
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches run"],
  },
];
