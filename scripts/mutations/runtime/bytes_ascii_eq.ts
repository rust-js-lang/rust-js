// Mutations of src/runtime/bytes_ascii_eq.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "bytes-ascii-eq-case-kept",
    breaks: "`a.eq_ignore_ascii_case(b)` of bytes tells `C` from `c`",
    file: "src/runtime/bytes_ascii_eq.js",
    find: "  const lower = (byte) => (byte >= 65 && byte <= 90 ? byte + 32 : byte);",
    replace: "  const lower = (byte) => byte;",
    tests: ["test/corpus.test.ts", "-t", "slice_prefixes"],
  },
  {
    name: "bytes-ascii-eq-beyond-ascii",
    breaks: "`a.eq_ignore_ascii_case(b)` of bytes lowers bytes past ASCII, `0xC0` as `0xE0`",
    file: "src/runtime/bytes_ascii_eq.js",
    find: "  const lower = (byte) => (byte >= 65 && byte <= 90 ? byte + 32 : byte);",
    replace: "  const lower = (byte) => ((byte >= 65 && byte <= 90) || (byte >= 0xc0 && byte <= 0xde) ? byte + 32 : byte);",
    tests: ["test/corpus.test.ts", "-t", "slice_prefixes"],
  },
];
