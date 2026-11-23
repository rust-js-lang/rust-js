// Mutations of crates/rust-js-program/src/runtime/json_path.js (ADR 1188).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-path-index-dotted",
    breaks: "an index is shown after a dot, `items.[1].qty`, where the crate writes `items[1].qty`",
    file: "crates/rust-js-program/src/runtime/json_path.js",
    find: "    if (typeof segment === \"number\") {\n      f.write(\"[\");",
    replace: "    if (typeof segment === \"number\") {\n      f.write(separator);\n      f.write(\"[\");",
    tests: ["test/serde-path.test.ts", "-t", "says where"],
  },
  {
    name: "json-path-root-blank",
    breaks: "the root's path, none, is shown as nothing, where the crate's `Path` shows `.`",
    file: "crates/rust-js-program/src/runtime/json_path.js",
    find: "  if (path.length === 0) return f.write(\".\");\n",
    replace: "  if (path.length === 0) return;\n",
    tests: ["test/serde-path.test.ts", "-t", "says where"],
  },
  {
    name: "json-path-unknown-shown",
    breaks: "an error at the root, or at a key that isn't text, shows its path, `.: expected` or `?: EOF`, where the crate shows serde_json's error alone",
    file: "crates/rust-js-program/src/runtime/json_path.js",
    find: "  if (!e.path.every((segment) => segment === null)) {\n",
    replace: "  if (true) {\n",
    tests: ["test/serde-path.test.ts", "-t", "says where"],
  },
  {
    name: "json-path-variant-debugged-as-key",
    breaks: "a variant's segment is shown `Map { key: \"circle\" }`, where the crate's derived `Debug` shows `Enum { variant: \"circle\" }`",
    file: "crates/rust-js-program/src/runtime/json_path.js",
    find: "  const segment = (g, s) => {\n    if (s === null) return g.write(\"Unknown\");\n",
    replace: "  const segment = (g, s) => {\n    if (s === null) return g.write(\"Unknown\");\n    if (s.variant !== undefined) s = s.variant;\n",
    tests: ["test/serde-path.test.ts", "-t", "says where"],
  },
  {
    name: "json-path-index-debug-unoptioned",
    breaks: "`{:x?}` of a path shows its index in decimal, where the derived `Debug` gives its field the `Formatter`'s options",
    file: "crates/rust-js-program/src/runtime/json_path.js",
    find: "  const index = (g, n) => $writeDebugInteger(g, n, n.toString(16), n.toString(16).toUpperCase(), options);\n",
    replace: "  const index = (g, n) => $writeInteger(g, n);\n",
    tests: ["test/serde-path.test.ts", "-t", "shown as the crate"],
  },
];
