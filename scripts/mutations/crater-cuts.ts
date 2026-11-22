// Mutations of what the small Crater reads and cuts of a crate's source
// (scripts/crater-cuts.ts): a macro invoked where an item goes, the tests an
// expansion holds, and a cut's reason followed to the function it lost (#1420).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "crater-invocation-not-cut",
    breaks: "the Crater finds no cut for an error at a macro invocation where an item goes, and refuses the whole test crate",
    file: "scripts/crater-cuts.ts",
    find: "  if (!outer && invocation && (testOnly || mods(invocation).some((m) => m.cfgTest))) return { from: invocation.from, invocation: true };\n",
    replace: "",
    tests: ["test/crater.test.ts", "-t", "a macro's invocation where an item goes is an item"],
  },
  {
    name: "crater-invocation-in-a-body",
    breaks: "the Crater reads a macro invoked in a function's body as an item of the file",
    file: "scripts/crater-cuts.ts",
    find: "      open.every((o) => o.item?.kind === \"mod\") &&\n",
    replace: "",
    tests: ["test/crater.test.ts", "-t", "a macro's invocation where an item goes is an item"],
  },
  {
    name: "crater-not-followed",
    breaks: "a test refused as its helper was cut is ranked under rustc's \"cannot find\", not the helper's refusal",
    file: "scripts/crater-cuts.ts",
    find: "  return cut === undefined ? why : `${items[cut].split(leftByCut)[0]}${leftByCut}${why}`;\n",
    replace: "  return why;\n",
    tests: ["test/crater.test.ts", "-t", "a cut for a function an earlier round cut is refused for that function's reason"],
  },
  {
    name: "crater-followed-chain-nests",
    breaks: "a chain of cut helpers nests each round's note in the reason, not the root's",
    file: "scripts/crater-cuts.ts",
    find: "  return cut === undefined ? why : `${items[cut].split(leftByCut)[0]}${leftByCut}${why}`;\n",
    replace: "  return cut === undefined ? why : `${items[cut]}${leftByCut}${why}`;\n",
    tests: ["test/crater.test.ts", "-t", "a cut for a function an earlier round cut is refused for that function's reason"],
  },
  {
    name: "crater-marker-of-one-line",
    breaks: "the Crater reads no test whose marker rustc printed on two lines, and takes it for one a cut took",
    file: "scripts/crater-cuts.ts",
    find: "  return new Set([...expanded.matchAll(/^\\s*#\\[rustc_test_marker\\s*=\\s*(\"(?:[^\"\\\\]|\\\\.)*\")\\s*\\]/gm)]",
    replace: "  return new Set([...expanded.matchAll(/^\\s*#\\[rustc_test_marker = (\"(?:[^\"\\\\]|\\\\.)*\")\\s*\\]/gm)]",
    tests: ["test/crater.test.ts", "-t", "the tests a cut macro invocation made"],
  },
];
