// `bun run push` (scripts/push.ts, ADR 1101, amended): a push whose sanity
// pass past a rebase isn't the change's own costs more than its check.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "push-sanity-whole",
    breaks: "a push's sanity pass past a rebase selects by the map's diff to the new tree, not the interaction of the change and the rebase's commits: it ran all and outlasted the check",
    file: "scripts/push.ts",
    find: '...(name === "sanity" ? [`--sanity=${checked}`] : [])',
    replace: "...([] as string[])",
    tests: ["test/select.test.ts", "-t", "bun run push says its selection"],
  },
  {
    name: "push-files-of-a-moved-main",
    breaks: "a fetch while the push was checked moved origin/main, and its new commits' files are said to be stray, refusing a push the next step would rebase",
    file: "scripts/push.ts",
    find: 'lines(git("diff", "--name-only", base).out)',
    replace: 'lines(git("diff", "--name-only", "origin/main").out)',
    tests: ["test/select.test.ts", "-t", "bun run push says its selection on its line"],
  },
  {
    name: "push-react-gate-reused-blind",
    breaks: "a push reuses the round before's React gate past a rebase whose commits touch what compiles the port, so a port regression another's commit and this one make together lands unseen",
    file: "scripts/push.ts",
    find: "  return incoming !== undefined && !touchesPort(incoming) ? \"reused\" : \"run\";\n",
    replace: "  return incoming !== undefined ? \"reused\" : \"run\";\n",
    tests: ["test/ci.test.ts", "-t", "a push's React gate runs again"],
  },
  {
    name: "push-react-gate-never-reused",
    breaks: "a push runs the React gate again at every round past a rebase, 54 s each, where the commits it brought touch nothing that compiles the port",
    file: "scripts/push.ts",
    find: "  return incoming !== undefined && !touchesPort(incoming) ? \"reused\" : \"run\";\n",
    replace: "  return \"run\";\n",
    tests: ["test/ci.test.ts", "-t", "a push's React gate runs again"],
  },
  {
    name: "push-remakes-any-conflict",
    breaks: "a rebase's conflict in hand-written code, `src/lower/analysis.rs`'s re-exports say, is resolved as main's, and the change's own side is lost",
    file: "scripts/push.ts",
    find: "  return conflicts.every((file) => file in regenerated);\n",
    replace: "  return true;\n",
    tests: ["test/ci.test.ts", "-t", "a rebase's conflict in a blessed baseline alone is made again"],
  },
  {
    name: "push-remakes-no-conflict",
    breaks: "a rebase's conflict in a blessed baseline alone stops the push, as two pushes that each blessed one did (2 of m3-max's 5 failed pushes)",
    file: "scripts/push.ts",
    find: "  return conflicts.every((file) => file in regenerated);\n",
    replace: "  return false;\n",
    tests: ["test/ci.test.ts", "-t", "a rebase's conflict in a blessed baseline alone is made again"],
  },
];
