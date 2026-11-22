// Mutations of scripts/own-compiler.ts (#761, #2317, ADR 0093): a run's
// own link of the compiler, and the folders of runs gone.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "own-compiler-dead-runs-kept",
    breaks: "a run killed before it removed its folder leaves its compiler's link in target/ for good",
    file: "scripts/own-compiler.ts",
    find: "      rmSync(join(runs, pid), { recursive: true, force: true });\n",
    replace: "",
    tests: ["test/push.test.ts", "-t", "a dead run's folder goes"],
  },
  {
    name: "own-compiler-checkout-build",
    breaks: "a run's own compiler is the checkout's build itself, which a relink takes from under what runs it",
    file: "scripts/own-compiler.ts",
    find: '  const compiler = join(ownFolder(root, kind), name);\n  rmSync(compiler, { force: true });\n  linkSync(join(root, "target", "debug", "rust-js"), compiler);\n  return compiler;\n',
    replace: '  void name;\n  void ownFolder(root, kind);\n  void rmSync;\n  void linkSync;\n  return join(root, "target", "debug", "rust-js");\n',
    tests: ["test/push.test.ts", "-t", "the round's own compiler"],
  },
];
