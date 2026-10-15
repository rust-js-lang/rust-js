// Mutations of src/runtime/path.js (ADR 0401).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "path-parts-keep-dots",
    breaks: "a path's `.` in its middle is a component: `a/./b` isn't `a/b`",
    file: "src/runtime/path.js",
    find: '  for (const piece of pieces) if (piece !== "" && piece !== ".") parts.push(piece);\n',
    replace: '  for (const piece of pieces) if (piece !== "") parts.push(piece);\n',
    tests: ["test/corpus.test.ts", "-t", "^path_compare.rs$"],
  },
  {
    name: "path-parts-no-cur-dir",
    breaks: "a path's leading `.` is no component: `./a` is `a`",
    file: "src/runtime/path.js",
    find: '  const parts = path.startsWith("/") ? ["/"] : pieces[0] === "." ? ["."] : [];\n',
    replace: '  const parts = path.startsWith("/") ? ["/"] : [];\n',
    tests: ["test/corpus.test.ts", "-t", "^path_compare.rs$"],
  },
  {
    name: "path-parts-no-root",
    breaks: "an absolute path's root is no component: `/a` is `a`",
    file: "src/runtime/path.js",
    find: '  const parts = path.startsWith("/") ? ["/"] : pieces[0] === "." ? ["."] : [];\n',
    replace: '  const parts = pieces[0] === "." ? ["."] : [];\n',
    tests: ["test/corpus.test.ts", "-t", "^path_compare.rs$"],
  },
  {
    name: "path-cmp-as-text",
    breaks: "paths order their components as text, `./here` before `/z`, where Rust puts the root first",
    file: "src/runtime/path.js",
    find: "    const order = rank(x[i]) - rank(y[i]) || $cmp(x[i], y[i]);\n",
    replace: "    const order = $cmp(x[i], y[i]);\n",
    tests: ["test/corpus.test.ts", "-t", "^path_keys.rs$"],
  },
  {
    name: "path-cmp-shorter-after",
    breaks: "a path is after one that goes on from it, `a/b` before `a`",
    file: "src/runtime/path.js",
    find: "  return Math.sign(x.length - y.length);\n",
    replace: "  return Math.sign(y.length - x.length);\n",
    tests: ["test/corpus.test.ts", "-t", "^path_compare.rs$"],
  },
  {
    name: "path-map-keyed-by-text",
    breaks: "a map of paths finds one by its text: `a//b` doesn't find `a/b`",
    file: "src/runtime/path.js",
    find: "class $PathMap extends $KeyMap {\n  keyOf(path) {\n    return $pathKey(path);\n",
    replace: "class $PathMap extends $KeyMap {\n  keyOf(path) {\n    return path;\n",
    tests: ["test/corpus.test.ts", "-t", "^path_keys.rs$"],
  },
  {
    name: "path-set-keyed-by-text",
    breaks: "a set of paths holds `a/b` and `a//b` both",
    file: "src/runtime/path.js",
    find: "class $PathSet extends $KeySet {\n  keyOf(path) {\n    return $pathKey(path);\n",
    replace: "class $PathSet extends $KeySet {\n  keyOf(path) {\n    return path;\n",
    tests: ["test/corpus.test.ts", "-t", "^path_keys.rs$"],
  },
  {
    name: "path-tree-keeps-first",
    breaks: "a B-tree of paths made of items keeps the first of equal ones, where std keeps the last",
    file: "src/runtime/path.js",
    find: "    tree.delete(set ? item : item[0]);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^path_keys.rs$"],
  },
];
