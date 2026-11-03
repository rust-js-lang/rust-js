// Mutations of src/node_session.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "session-stdin-decoded-by-chunk",
    breaks:
      "a Node session decodes each of its stdin's chunks alone, and a character split across two is two `\\u{FFFD}`, so Prettier's layout of React's `⚛` changes the program and isn't taken (rd716)",
    file: "src/node_session.rs",
    find: '  process.stdin.setEncoding("utf8");\n',
    replace: "",
    tests: ["test/settings.test.ts", "-t", "across a Node session's stdin chunks"],
  },
];
