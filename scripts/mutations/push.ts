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
];
