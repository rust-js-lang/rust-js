// A survivor's run at the base, in a worktree of its own, never the checkout
// under test (scripts/latent-survivors.ts, #2578): it leaves the checkout,
// and what the run wrote of its own, as they are throughout.
import type { Mutation } from "../mutations";

const file = "scripts/latent-survivors.ts";
const apart = ["test/survivors.test.ts", "-t", "in a worktree of its own"];

export const mutations: Mutation[] = [
  {
    name: "base-run-checks-out-the-checkout",
    breaks: "a survivor's run at the base checks the base out in the checkout under test, so check:fast's other jobs test the base's files with the change's compiler, a check failing or passing of a tree that isn't the change (#2578)",
    file,
    find: '    const made = await git(["worktree", "add", "--quiet", "--detach", tree, base]);\n',
    replace: '    await git(["checkout", "--quiet", "--detach", base]);\n    const made = await git(["worktree", "add", "--quiet", "--detach", tree, base]);\n',
    tests: apart,
  },
  {
    name: "base-run-by-the-checkouts-runner",
    breaks: "a survivor's run at the base is the checkout's runner's, the push's own, not the base's, in the checkout under test, which writes its results where the run's are",
    file,
    find: '    const ran = await run(["bun", "scripts/mutations.ts", ...known], tree, baseLimit,',
    replace: '    const ran = await run(["bun", "scripts/mutations.ts", ...known], root, baseLimit,',
    tests: apart,
  },
  {
    name: "base-run-leaves-its-results",
    breaks: "a survivor's run at the base is read of the run's own results.json, not the base's runner's, so each survivor is said latent or the push's by what the run itself found (#2222)",
    file,
    find: '    const at = join(tree, "target", "mutants", "results.json");\n',
    replace: '    const at = join(work, "results.json");\n',
    tests: apart,
  },
  {
    name: "base-run-leaves-its-worktree",
    breaks: "a survivor's run at the base leaves its worktree, a whole checkout and its build, under the mutants' store, and listed by git, after each run",
    file,
    find: "  } finally {\n    await removeTree();\n  }\n",
    replace: "  } finally {\n  }\n",
    tests: apart,
  },
  {
    name: "base-run-unnamed-its-map",
    breaks: "a survivor's run at the base isn't named the test map the run read, so in CI, which keeps a map of the push's alone, no ancestor of the base, its snapshot judge finds none, and each readability owner's survivor isn't judged there as it was here (#2341)",
    file,
    find: "tree, baseLimit, map === undefined ? {} : { [namedMapEnv]: map });\n",
    replace: "tree, baseLimit);\n",
    tests: ["test/survivors.test.ts", "-t", "judged by the test map the run read"],
  },
];
