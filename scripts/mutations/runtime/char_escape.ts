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
    name: "decode-utf16-unpaired-pairs",
    breaks: "`decode_utf16` takes a surrogate pair as two errors",
    file: "src/runtime/char_escape.js",
    find: "      if (next.value < 0xdc00 || next.value > 0xdfff) {\n",
    replace: "      if (true) {\n",
    tests: ["test/corpus.test.ts", "-t", "cell_deque_char_methods"],
  },
  {
    name: "decode-utf16-unit-past-lost",
    breaks: "`decode_utf16` drops the unit it read past an unpaired leading surrogate, where std's keeps it for its next step (#1151)",
    file: "src/runtime/char_escape.js",
    find: "        buf = next.value;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^decode_utf16_steps_as_asked.rs$"],
  },
  {
    name: "decode-utf16-hint-unhalved",
    breaks: "`decode_utf16`'s `size_hint()` is its source's lower bound, not half of it rounded up, as std's is (#1151)",
    file: "src/runtime/char_escape.js",
    find: "    return [Math.ceil(low / 2) + kept, high === undefined ? undefined : high + kept];\n",
    replace: "    return [low + kept, high === undefined ? undefined : high + kept];\n",
    tests: ["test/corpus.test.ts", "-t", "^decode_utf16_steps_as_asked.rs$"],
  },
  {
    name: "decode-utf16-array-hintless",
    breaks: "`decode_utf16` of an array steps it by JS's own iterator, which knows no hint, so its `size_hint()` is `(0, None)` (#1151)",
    file: "src/runtime/char_escape.js",
    find: "  const it = Array.isArray(units) ? $iter(units) : typeof units.next === \"function\" ? units : units[Symbol.iterator]();\n",
    replace: "  const it = typeof units.next === \"function\" ? units : units[Symbol.iterator]();\n",
    tests: ["test/corpus.test.ts", "-t", "^decode_utf16_steps_as_asked.rs$"],
  },
];
