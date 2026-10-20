// Mutations of src/runtime/fuse.js (ADR 0473).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fuse-keeps-every-source",
    breaks: "a `Fuse` of what isn't a `FusedIterator` keeps it as it ends, dropping it only with the `Fuse`",
    file: "src/runtime/fuse.js",
    find: "    if (at.done && !fused) {\n",
    replace: "    if (at.done && false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterators.rs$"],
  },
  {
    name: "fuse-backless",
    breaks: "a `fuse` of what knows its back doesn't know its own",
    file: "src/runtime/fuse.js",
    find: "  if (iter.back !== undefined) it.back = stepped(() => it.iter.back());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fuse_enumerate_ends.rs$"],
  },
  {
    name: "fuse-lengthless",
    breaks: "a `fuse` of what knows its length doesn't know its own",
    file: "src/runtime/fuse.js",
    find: "  if (iter.len !== undefined) it.len = () => (it.iter === undefined ? 0 : it.iter.len());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fuse_enumerate_ends.rs$"],
  },
];
