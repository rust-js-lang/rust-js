// The push guard (scripts/push-guard.ts): a push it lets through that
// undoes another's commit, or one it refuses that undoes nothing.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [  {
    name: "push-guard-undone-already-counted",
    breaks: "a file as it was before a commit a later one already undid is read as undoing it, so a push that takes off a section another appended must name a commit it doesn't revert (#2009)",
    file: "scripts/push-guard.ts",
    find: "      if (before === pushed && after !== pushed && !later.has(pushed) && !named(commit)) {\n",
    replace: "      if (before === pushed && after !== pushed && !named(commit)) {\n",
    tests: ["test/push-guard.test.ts", "-t", "a later commit already undid"],
  },
  {
    name: "push-guard-undone-remote-unasked",
    breaks: "a commit whose change a later one took off the remote again is read as undone where its patch applies to what's pushed, as 24e269c11's section did once #2007's followed it (#2009)",
    file: "scripts/push-guard.ts",
    find: "      if (has && appliesTo(",
    replace: "      if (appliesTo(",
    tests: ["test/push-guard.test.ts", "-t", "a later commit already undid"],
  },
];
