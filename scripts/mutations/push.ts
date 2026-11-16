// `bun run push` (scripts/push.ts, ADR 1101, amended): a push whose sanity
// pass past a rebase isn't the change's own costs more than its check.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "push-sanity-whole",
    breaks: "a push's sanity pass past a rebase selects by the map's diff to the new tree, others' commits too, not by the change's own: it ran all and outlasted the check",
    file: "scripts/push.ts",
    find: '...(name === "sanity" ? ["--sanity"] : [])',
    replace: "...([] as string[])",
    tests: ["test/select.test.ts", "-t", "bun run push says its selection"],
  },
];
