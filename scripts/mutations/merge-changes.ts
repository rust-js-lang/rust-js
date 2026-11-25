// TESTS.md's `Changed on` lines merged, by git's driver and by a push's
// rebase alike (scripts/merge-changes.ts, #2587).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "changes-merged-oldest-first",
    breaks: "two pushes' TESTS.md lines are merged main's first, so the replayed change's, the newest, is below an older one",
    file: "scripts/merge-changes.ts",
    find: '    out.push(...[...added, ...kept].flatMap((line, at) => (at === 0 ? [line] : ["", line])));\n',
    replace: '    out.push(...[...kept, ...added].flatMap((line, at) => (at === 0 ? [line] : ["", line])));\n',
    tests: ["test/push.test.ts", "-t", "TESTS.md"],
  },
  {
    name: "changes-merged-with-the-old-line",
    breaks: "a TESTS.md line main rewrote is kept beside its rewrite, as the replayed change still had it, so the log says both counts",
    file: "scripts/merge-changes.ts",
    find: '    const added = theirs.filter((line) => line !== "" && !base.includes(line));\n',
    replace: '    const added = theirs.filter((line) => line !== "");\n',
    tests: ["test/push.test.ts", "-t", "TESTS.md"],
  },
  {
    name: "changes-log-driver-merges-nothing",
    breaks: "git's TESTS.md driver leaves every conflict of its `Changed on` lines to a hand, so a relay's cherry-pick stops on them as nearly every one did (#2587)",
    file: "scripts/merge-changes.ts",
    find: "  return merged === undefined ? 1 : 0;\n",
    replace: "  return 1;\n",
    tests: ["test/push.test.ts", "-t", "merged by git's driver"],
  },
];
