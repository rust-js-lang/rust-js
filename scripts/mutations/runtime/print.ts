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
    find: "  v.push(...new TextEncoder().encode(text));\n",
    replace: "  v.push(...[...text].map((c) => c.charCodeAt(0)));\n",
    tests: ["test/corpus.test.ts", "-t", "^io_writers.rs$"],
  },
];
