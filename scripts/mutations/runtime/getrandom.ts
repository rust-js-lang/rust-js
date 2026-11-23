// Mutations of crates/rust-js-program/src/runtime/getrandom.js (ADR 1185).
import type { Mutation } from "../../mutations";

const file = "crates/rust-js-program/src/runtime/getrandom.js";
const tests = ["test/shipped-crates.test.ts", "-t", "getrandom's API runs"];

export const mutations: Mutation[] = [
  {
    name: "getrandom-one-chunk",
    breaks: "a fill of more than 65,536 bytes fills its first 65,536 alone",
    file,
    find: "  for (let at = 0; at < dest.length; at += chunk.length) {\n",
    replace: "  for (let at = 0; at < chunk.length && at === 0; at += chunk.length) {\n",
    tests,
  },
  {
    name: "getrandom-uninit-ok-empty",
    breaks: "`fill_uninit` gives `Ok` of nothing, not of the bytes it filled",
    file,
    find: '  return uninit ? { TAG: "Ok", _0: dest } : { TAG: "Ok", _0: undefined };\n',
    replace: '  return { TAG: "Ok", _0: undefined };\n',
    tests,
  },
  {
    name: "getrandom-os-words-unread",
    breaks: "an `Error` of an OS's number shows `OS Error: 2`, where native unix getrandom shows its `strerror` words",
    file,
    find: "  return named ? $osErrorOf(errno).message : undefined;\n",
    replace: "  return undefined;\n",
    tests,
  },
  {
    name: "getrandom-debug-numbers-unpadded",
    breaks: "an `Error`'s `{:12?}` shows its code unpadded, where `debug_struct` gives its field the `Formatter`'s width",
    file,
    find: "  const number = (g, n) => $writeDebugInteger(g, n, n.toString(16), n.toString(16).toUpperCase(), options);\n",
    replace: "  const number = (g, n) => $writeInteger(g, n);\n",
    tests,
  },
  {
    name: "getrandom-unknown-code-as-internal",
    breaks: "a code past each version's table shows as an internal one, `Error { internal_code, description: undefined }`",
    file,
    find: '  else fields.push([words !== undefined ? "internal_code" : "unknown_code", code, number]);\n',
    replace: '  else fields.push(["internal_code", code, number]);\n',
    tests,
  },
];
