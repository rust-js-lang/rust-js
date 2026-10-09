// Mutations of src/runtime/char_escape.js (ADR 0327).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "escape-debug-str-marks",
    breaks: "a `str`'s `escape_debug` escapes a combining mark after the first",
    file: "src/runtime/char_escape.js",
    find: "    out += $debugChar(c, \"'\\\"\", !str || first);\n",
    replace: "    out += $debugChar(c, \"'\\\"\");\n",
    tests: ["test/corpus.test.ts", "-t", "cell_deque_char_methods"],
  },
  {
    name: "escape-default-unicode-raw",
    breaks: "`escape_default` leaves a non-ASCII `char` as it is",
    file: "src/runtime/char_escape.js",
    find: "    else if (c >= \" \" && c <= \"~\") out += c;\n",
    replace: "    else if (c >= \" \") out += c;\n",
    tests: ["test/corpus.test.ts", "-t", "cell_deque_char_methods"],
  },
  {
    name: "encode-utf8-unwritten",
    breaks: "`encode_utf8` writes nothing to its buffer",
    file: "src/runtime/char_escape.js",
    find: "  bytes.forEach((b, i) => (buf[i] = b));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "cell_deque_char_methods"],
  },
  {
    name: "decode-utf16-unpaired-pairs",
    breaks: "`decode_utf16` takes a surrogate pair as two errors",
    file: "src/runtime/char_escape.js",
    find: "    else if (unit <= 0xdbff && i + 1 < list.length && list[i + 1] >= 0xdc00 && list[i + 1] <= 0xdfff) {\n",
    replace: "    else if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "cell_deque_char_methods"],
  },
];
