// Mutations of src/runtime/char_indices.js (ADR 0093, ADR 1117).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "char-indices-units",
    breaks: "`char_indices()` counts UTF-16 units, not UTF-8 bytes",
    file: "src/runtime/char_indices.js",
    find: "  this.byte += $byteLen(c);",
    replace: "  this.byte += c.length;",
    tests: ["test/corpus.test.ts", "-t", "^chars_given_to_helpers.rs$"],
  },
  {
    name: "chars-back-splits-a-pair",
    breaks: "`next_back()` of a `char` outside the BMP gives its pair's second half alone",
    file: "src/runtime/char_indices.js",
    find: "  if (unit >= 0xdc00 && unit <= 0xdfff && from > this.at) from--;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^chars_given_to_helpers.rs$"],
  },
  {
    name: "chars-back-bytes-units",
    breaks: "what's stepped from its back is counted in UTF-16 units, so its hint and its back's byte are another",
    file: "src/runtime/char_indices.js",
    find: "  this.backBytes += $byteLen(c);",
    replace: "  this.backBytes += c.length;",
    tests: ["test/corpus.test.ts", "-t", "^chars_size_hint.rs$"],
  },
  {
    name: "chars-hint-lower-floored",
    breaks: "the hint's lower bound is the bytes left over 4 rounded down, not std's `div_ceil(4)`",
    file: "src/runtime/char_indices.js",
    find: "  return [Math.ceil(bytes / 4), bytes];",
    replace: "  return [Math.floor(bytes / 4), bytes];",
    tests: ["test/corpus.test.ts", "-t", "^chars_size_hint.rs$"],
  },
  {
    name: "char-indices-offset-in-units",
    breaks: "`offset()` is where its front is in UTF-16 units, not UTF-8 bytes",
    file: "src/runtime/char_indices.js",
    find: "  return it.byte;",
    replace: "  return it.at;",
    tests: ["test/corpus.test.ts", "-t", "^chars_size_hint.rs$"],
  },
  {
    name: "chars-rest-past-its-back",
    breaks: "`as_str()` gives what was stepped from its back too",
    file: "src/runtime/char_indices.js",
    find: "  const part = this.s.slice(this.at, this.end);",
    replace: "  const part = this.s.slice(this.at);",
    tests: ["test/corpus.test.ts", "-t", "^chars_size_hint.rs$"],
  },
  {
    name: "chars-clone-shared",
    breaks: "a clone of a `chars()` is the same iterator, so stepping one steps the other",
    file: "src/runtime/char_indices.js",
    find: "  return Object.assign(Object.create(Iterator.prototype), this);",
    replace: "  return this;",
    tests: ["test/corpus.test.ts", "-t", "^std_iterator_clones.rs$"],
  },
];
