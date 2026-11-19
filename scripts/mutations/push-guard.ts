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
  {
    name: "push-guard-fixed-rows-undo",
    breaks: "rows a push takes off a known list under an issue it fixes are read as undoing the commit that listed them, and the push refused (#2146, #2139)",
    file: "scripts/push-guard.ts",
    find: "    .filter((file) => !fixedRowsOnly(",
    replace: "    .filter((file) => file !== \"\" || !fixedRowsOnly(",
    tests: ["test/push-guard.test.ts", "-t", "rows taken off a known list"],
  },
  {
    name: "push-guard-fixed-rows-any-heading",
    breaks: "rows a push takes off a known list are let go under any issue's heading, not only one it fixes, so a stale tree's removal of another's rows undoes them unseen",
    file: "scripts/push-guard.ts",
    find: "      if (issue === undefined || !fixes.has(issue)) return false;\n",
    replace: "      if (issue === undefined) return false;\n",
    tests: ["test/push-guard.test.ts", "-t", "rows taken off a known list"],
  },
  {
    name: "push-guard-first-fixed-issue",
    breaks: "of a commit that fixes several issues, `Fixes #6, #7`, only the first is read",
    file: "scripts/push-guard.ts",
    find: "[...m[1]!.matchAll(/#(\\d+)/g)].map((n) => n[1]!)",
    replace: "[...m[1]!.matchAll(/#(\\d+)/g)].slice(0, 1).map((n) => n[1]!)",
    tests: ["test/push-guard.test.ts", "-t", "rows taken off a known list"],
  },
];
