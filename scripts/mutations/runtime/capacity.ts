// Mutations of src/runtime/capacity.js (ADR 0419).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "shrink-to-unshrunk",
    breaks: "`shrink_to_fit()` leaves its room",
    file: "src/runtime/capacity.js",
    find: "  $capacities.set(v, Math.max(v.length, Math.min($capacity(v), min)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_capacity.rs$"],
  },
  {
    name: "room-past-usize-max-items",
    breaks: "room for more than `usize::MAX` items of no size is no panic",
    file: "src/runtime/capacity.js",
    find: "  if (count > 2 ** 32 - 1 || count * size > 2 ** 31 - 1) throw new Error(\"capacity overflow\");\n",
    replace: "  if (count * size > 2 ** 31 - 1) throw new Error(\"capacity overflow\");\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_overflow.rs$"],
  },
  {
    name: "room-past-usize-max-bytes",
    breaks: "room is checked against `usize::MAX` bytes, not `isize::MAX`",
    file: "src/runtime/capacity.js",
    find: "  if (count > 2 ** 32 - 1 || count * size > 2 ** 31 - 1) throw new Error(\"capacity overflow\");\n",
    replace: "  if (count > 2 ** 32 - 1 || count * size > 2 ** 32 - 1) throw new Error(\"capacity overflow\");\n",
    tests: ["test/runtime-package.test.ts", "-t", "room past"],
  },
  {
    name: "text-room-of-units",
    breaks: "a `String`'s room counts its UTF-16 units as a byte each, where it may not fit",
    file: "src/runtime/capacity.js",
    find: "  if (additional > 2 ** 31 - 1 - 3 * s.length) $roomFor($byteLen(s) + additional, 1);\n",
    replace: "  if (additional > 2 ** 31 - 1 - s.length) $roomFor($byteLen(s) + additional, 1);\n",
    tests: ["test/runtime-package.test.ts", "-t", "room past"],
  },
  {
    name: "text-room-unchecked",
    breaks: "`s.reserve(n)` of a `String` is no panic",
    file: "src/runtime/capacity.js",
    find: "  if (additional > 2 ** 31 - 1 - 3 * s.length) $roomFor($byteLen(s) + additional, 1);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^capacity_overflow.rs$"],
  },
];
