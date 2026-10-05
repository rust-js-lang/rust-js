// Mutations of src/runtime/utf8.js (ADR 0172).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "utf8-surrogates-accepted",
    breaks: "`ED A0 80`, a surrogate's bytes, is taken for UTF-8",
    file: "src/runtime/utf8.js",
    find: "    const high = first === 0xed ? 0x9f : first === 0xf4 ? 0x8f : 0xbf;",
    replace: "    const high = first === 0xf4 ? 0x8f : 0xbf;",
    tests: ["test/corpus.test.ts", "-t", "utf8_decoding"],
  },
  {
    name: "utf8-past-max-accepted",
    breaks: "`F4 90 80 80`, past U+10FFFF, is taken for UTF-8",
    file: "src/runtime/utf8.js",
    find: "    const high = first === 0xed ? 0x9f : first === 0xf4 ? 0x8f : 0xbf;",
    replace: "    const high = first === 0xed ? 0x9f : 0xbf;",
    tests: ["test/corpus.test.ts", "-t", "utf8_decoding"],
  },
  {
    name: "utf8-incomplete-as-invalid",
    breaks: "bytes ending inside a sequence are an invalid sequence, not an incomplete one",
    file: "src/runtime/utf8.js",
    find: "      if (i + k >= bytes.length) return [i, undefined];",
    replace: "      if (i + k >= bytes.length) return [i, k];",
    tests: ["test/corpus.test.ts", "-t", "utf8_decoding"],
  },
  {
    name: "utf8-error-length-whole",
    breaks: "a bad sequence's length is its first byte's width, not where it went bad",
    file: "src/runtime/utf8.js",
    find: "      if (k === 1 ? byte < low || byte > high : (byte & 0xc0) !== 0x80) return [i, k];",
    replace: "      if (k === 1 ? byte < low || byte > high : (byte & 0xc0) !== 0x80) return [i, width];",
    tests: ["test/corpus.test.ts", "-t", "utf8_decoding"],
  },
  {
    name: "utf8-lossy-always-owned",
    breaks: "`from_utf8_lossy` of valid UTF-8 is owned, not borrowed",
    file: "src/runtime/utf8.js",
    find: '  return { TAG: $utf8Check(bytes) === undefined ? "Borrowed" : "Owned", _0: $utf8Decode(bytes) };',
    replace: '  return { TAG: "Owned", _0: $utf8Decode(bytes) };',
    tests: ["test/corpus.test.ts", "-t", "utf8_decoding"],
  },
  {
    name: "utf8-drops-bom",
    breaks: "`String::from_utf8_lossy` of text beginning with U+FEFF drops it, as a byte order mark",
    file: "src/runtime/utf8.js",
    find: 'new TextDecoder("utf-8", { ignoreBOM: true })',
    replace: "new TextDecoder()",
    tests: ["test/corpus.test.ts", "-t", "matrix_strings"],
  },
];
