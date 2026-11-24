// Mutations of tooling/bounded.js: a store held to its cap and the disk's
// floor, the least recently used first, each thing removed as its store says.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "evict-removal-refused-ignored",
    breaks: "a store's removal that refuses a thing, a kept app a process claimed, is counted removed and its size taken off, so the prune stops short of the floor (#2446)",
    file: "tooling/bounded.js",
    find: "    else if (!remove(thing)) continue;\n",
    replace: "    else remove(thing);\n",
    tests: ["test/support.test.ts", "-t", "a kept app claimed after the prune listed it"],
  },];
