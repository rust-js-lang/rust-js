// Mutations of src/runtime/char_boundary.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "char-boundary-of-a-part-its-texts",
    breaks: "where a byte of a part of a long text is, is given in its text's units, not the part's",
    file: "src/runtime/char_boundary.js",
    find: "      return inRoot === undefined ? undefined : inRoot - facts.rootUnit;\n",
    replace: "      return inRoot;\n",
    tests: ["test/runtime-package.test.ts", "-t", "kept facts"],
  },
  {
    name: "char-boundary-step-past",
    breaks: "a byte's unit is found from the step of bytes after it, not before, so it's never found",
    file: "src/runtime/char_boundary.js",
    find: "    const step = Math.floor(at / $textStep);\n",
    replace: "    const step = Math.ceil(at / $textStep);\n",
    tests: ["test/runtime-package.test.ts", "-t", "kept facts"],
  },
  {
    name: "char-boundary-ascii-units-past",
    breaks: "of an ASCII long text, a byte's unit is one past it",
    file: "src/runtime/char_boundary.js",
    find: "    if (facts.ascii) return at;\n",
    replace: "    if (facts.ascii) return at + 1;\n",
    tests: ["test/corpus.test.ts", "-t", "^long_text_read_byte_by_byte.rs$"],
  },
];
