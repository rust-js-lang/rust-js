// Mutations of src/runtime/print.js (ADR 0392).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "print-bytes-decoded-in-node",
    breaks: "bytes written to Node's stream are decoded and printed as text, a character split between writes lost",
    file: "src/runtime/print.js",
    find: "  if (stream?.write) stream.write(Uint8Array.from(bytes));\n  else ",
    replace: "  ",
    tests: ["test/corpus.test.ts", "-t", "^io_writers.rs$"],
  },
  {
    name: "vec-writer-text-unencoded",
    breaks: "a `Vec<u8>`'s `write!` adds the text's UTF-16 code units, not its UTF-8 bytes",
    file: "src/runtime/print.js",
    find: "  $extend(v, new TextEncoder().encode(text));\n",
    replace: "  $extend(v, Array.from(text, (c) => c.charCodeAt(0)));\n",
    tests: ["test/corpus.test.ts", "-t", "^io_writers.rs$"],
  },
  {
    name: "io-write-all-error-refused",
    breaks: "std's `write_all` of a writer whose `write` fails panics, not giving its error",
    file: "src/runtime/print.js",
    find: "        throw written._0;\n",
    replace: "        throw new Error(\"a write that fails\");\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "vec-writer-spread-arguments",
    breaks: "a `Vec<u8>`'s `write_all` as a `dyn Write` spreads its bytes into one `push`, more than a JS call takes: `RangeError` (ADR 0919)",
    file: "src/runtime/print.js",
    find: "writeAll: (w, bytes) => $extend(w.value, bytes),",
    replace: "writeAll: (w, bytes) => void w.value.push(...bytes),",
    tests: ["test/corpus.test.ts", "-t", "^runtime_long_writes.rs$"],
  },
  {
    name: "slice-writer-unadvanced",
    breaks: "std's `Write for &mut [u8]` writes its bytes but doesn't re-point the slice past them, the next write over them (#1282)",
    file: "src/runtime/print.js",
    find: "  w.value = $view(room, n);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_of_a_mut_slice.rs$"],
  },
  {
    name: "line-held-past-its-newline",
    breaks: "stdout's line is never written at its newline, only as `$printing` ends, so standard error written meanwhile comes first, and a print inside a later line follows it (#1490)",
    file: "src/runtime/print.js",
    find: "  const end = $lineHeld.lastIndexOf(\"\\n\");\n  if (end < 0) return;\n",
    replace: "  const end = -1;\n  if (end < 0) return;\n",
    tests: ["test/runtime-print.test.ts", "-t", "stdout's line is written at its newline"],
  },
  {
    name: "line-left-unended-lost",
    breaks: "what's left unended of a line, a `print!`'s or a panic's part-way, is never written (#1490)",
    file: "src/runtime/print.js",
    find: "      $print(rest);\n    }\n  }\n}\n",
    replace: "    }\n  }\n}\n",
    tests: ["test/corpus.test.ts", "-t", "^printed_while_a_line_is_formatted.rs$"],
  },
  {
    name: "sink-never-the-line",
    breaks: "a writer that prints writes its text to a string of its own even as it formats into stdout's line, so its text comes after what it prints (#1490)",
    file: "src/runtime/print.js",
    find: "  if ($printFrames.at(-1) === $out) return { write: $out, done: () => \"\" };\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^printed_while_a_line_is_formatted.rs$"],
  },
];
