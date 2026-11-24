// Mutations of the compile cache (test/compiles.ts): what a kept compile is counted by.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "kept-compile-counted-by-lengths",
    breaks: "a kept compile is counted by its files' lengths, not the blocks it takes, `entry.json`'s left out, so a store `prune` holds to its 2 GiB cap takes 5 on the disk (ADR 1118, amended)",
    file: "test/compiles.ts",
    find: "  const bytes = bytesOf(building);\n",
    replace: "  const bytes = files.reduce((sum, file) => sum + file.length, 0);\n",
    tests: ["test/compiles.test.ts", "-t", "counted by its blocks on the disk"],
  },
  {
    name: "building-uncounted",
    breaks: "what a writer is building, or a writer that ended left, isn't counted in the store's size, so a store at its cap holds more on the disk than its cap (#2107)",
    file: "test/compiles.ts",
    find: "  const size = kept.size + building;\n",
    replace: "  const size = kept.size;\n",
    tests: ["test/compiles.test.ts", "-t", "a compile being built counts in the store's size"],
  },
  {
    name: "building-counted-before-cleanup",
    breaks: "a writer's leftover an hour old is counted in the store's size by the prune that removes it, so the size says more than's on the disk (#2107)",
    file: "test/compiles.ts",
    find: "  const all = entries(store);\n  const building = buildingBytes(store);\n",
    replace: "  const building = buildingBytes(store);\n  const all = entries(store);\n",
    tests: ["test/compiles.test.ts", "-t", "a compile being built counts in the store's size"],
  },
  {
    name: "relinked-compile-kept",
    breaks: "a compile whose compiler Cargo linked again as it ran is kept under the identity read before it, so another build's output is given again where this one's is asked (#2363)",
    file: "test/compiles.ts",
    find: "    if (!sameFile(statOf(program), compiler.file)) return;\n",
    replace: "",
    tests: ["test/compiles.test.ts", "-t", "linked again as it runs"],
  },
  {
    name: "another-builds-entry-given",
    breaks: "an entry another build kept is given again where this build's key finds it (#2363)",
    file: "test/compiles.ts",
    find: "    if (entry.identity !== identity) continue;\n",
    replace: "",
    tests: ["test/compiles.test.ts", "-t", "an entry another build kept is never given again"],
  },
  {
    name: "compile-key-rust-js-flags-dropped",
    breaks: "a kept compile's key has rustc's arguments alone, not rust-js's own before `--`, so the corpus compiled with `--std-iterators` is given the JS made without (#2501)",
    file: "test/compiles.ts",
    find: "  return sha(JSON.stringify([VERSION, identity, cwd, args, environment, searched]));",
    replace: "  return sha(JSON.stringify([VERSION, identity, cwd, args.filter((arg, i) => i >= args.indexOf(\"--\") || !arg.startsWith(\"--\")), environment, searched]));",
    tests: ["test/compiles.test.ts", "-t", "a compile is given again only where all it's made of is the same"],
  },
];
