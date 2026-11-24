// Mutations of the push's gate (scripts/judged.ts, #2485): the tree a run
// judged, what it judged on it, and those of the change it didn't.
import type { Mutation } from "../mutations";

const file = "scripts/judged.ts";
const tree = ["test/judged.test.ts", "-t", "a working tree's tree is its files as they are"];
const record = ["test/judged.test.ts", "-t", "what a run judged is recorded by its tree"];

export const mutations: Mutation[] = [
  {
    name: "judged-tree-untracked-left-out",
    breaks: "a tree is git's tracked files alone, so a run that judged a test the change hasn't committed is said to have judged the tree without it, and the tree the push commits it to is taken as judged",
    file,
    find: '    git(root, ["add", "--pathspec-from-file=-", "--pathspec-file-nul"], env, sourceFilesOf(root).join("\\0"));\n',
    replace: "",
    tests: tree,
  },
  {
    name: "judged-tree-deleted-kept",
    breaks: "a tracked file the working tree deleted is kept in its tree, so a run with it gone is said to have judged the tree that has it",
    file,
    find: '    git(root, ["add", "--update"], env);\n',
    replace: "",
    tests: tree,
  },
  {
    name: "judged-of-any-tree",
    breaks: "a mutation judged on any tree is taken as judged on every one, so a push whose rule changed after its mutations were judged lands unjudged, as #2512 did",
    file,
    find: "  return new Set(read(file).find((entry) => entry.tree === tree)?.judged ?? []);\n",
    replace: "  return new Set(read(file).flatMap((entry) => entry.judged));\n",
    tests: record,
  },
  {
    name: "judged-record-replaced",
    breaks: "a run's record of a tree replaces another run's of it, so what check:fast's job judged is lost to the push's next and judged again",
    file,
    find: "  const own = new Set([...(was.find((entry) => entry.tree === tree)?.judged ?? []), ...names]);\n",
    replace: "  const own = new Set(names);\n",
    tests: record,
  },
  {
    name: "judged-record-unbounded",
    breaks: "the record keeps every tree a run judged, growing a line each run",
    file,
    find: ".slice(0, judgedTrees);\n",
    replace: ";\n",
    tests: record,
  },
  {
    name: "unjudged-record-unread",
    breaks: "the change's mutations judged on its tree are judged again, every round of a push and every check, its record unread",
    file,
    find: "  return { names: run.map((m) => m.name).filter((name) => !judged.has(name)), wasm: elsewhere.map((m) => m.name) };\n",
    replace: "  return { names: run.map((m) => m.name), wasm: elsewhere.map((m) => m.name) };\n",
    tests: ["test/judged.test.ts", "-t", "the change's mutations not judged on its tree"],
  },
];
