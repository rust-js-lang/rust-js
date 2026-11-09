// Mutations of src/runtime/time_debug.js (ADR 0447).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "time-debug-before-epoch-unborrowed",
    breaks:
      "a time before the epoch shows its nanoseconds below zero, `tv_sec: -1, tv_nsec: -500000000`, not unix's `tv_sec: -2, tv_nsec: 500000000`",
    file: "src/runtime/time_debug.js",
    find: "  if (past < 0n) {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^time_debug_as_unix\\.rs$"],
  },
  {
    name: "time-debug-hex-unread",
    breaks: "`{:x?}` of a time shows its fields in decimal, where std's `debug_struct` gives each integer the hex flag",
    file: "src/runtime/time_debug.js",
    find: "    $formatted($debugInt(String(n), BigInt.asUintN(bits, n).toString(16), options), options, true);\n",
    replace: "    $formatted(String(n), options, true);\n",
    tests: ["test/corpus.test.ts", "-t", "^time_debug_as_unix\\.rs$"],
  },
  {
    name: "duration-debug-zero-padded",
    breaks: "`{:010?}` of a `Duration` is padded by zeros, which std's never is",
    file: "src/runtime/time_debug.js",
    find: "  return $pad(text, options.width, align, options.fill);\n",
    replace: "  return options.zero ? $zeroPad(text, options.width) : $pad(text, options.width, align, options.fill);\n",
    tests: ["test/corpus.test.ts", "-t", "^duration_debug_options\\.rs$"],
  },
  {
    name: "duration-debug-right-aligned",
    breaks: "`{:10?}` of a `Duration` is padded to the right, as a number's is, where std's pads it to the left",
    file: "src/runtime/time_debug.js",
    find: '  const align = { Left: "<", Right: ">", Center: "^" }[options.align] ?? "<";\n',
    replace: '  const align = { Left: "<", Right: ">", Center: "^" }[options.align] ?? ">";\n',
    tests: ["test/corpus.test.ts", "-t", "^duration_debug_options\\.rs$"],
  },
];
