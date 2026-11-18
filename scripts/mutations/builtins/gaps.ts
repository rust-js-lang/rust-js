// Mutations of builtins/gaps.ts, what the js crate doesn't bind, each named
// with its issue (#1903).
import type { Mutation } from "../../mutations";

const file = "builtins/gaps.ts";
const tests = ["test/builtins-coverage.test.ts", "-t", "a gap list names each"];

export const mutations: Mutation[] = [
  {
    name: "gaps-unlinked-unseen",
    breaks: "an unbound member no line of builtins/gaps.txt names passes, a gap with no issue",
    file,
    find: "    ...unbound.filter((m) => !list.some((g) => names(g, m)))",
    replace: "    ...unbound.filter((m) => false && !list.some((g) => names(g, m)))",
    tests,
  },
  {
    name: "gaps-stale-unseen",
    breaks: "a line of builtins/gaps.txt that names nothing unbound stays, its gap gone",
    file,
    find: "    ...list.filter((g) => !unbound.some((m) => names(g, m)))",
    replace: "    ...list.filter((g) => false && !unbound.some((m) => names(g, m)))",
    tests,
  },
  {
    name: "gaps-bound-still-named",
    breaks: "a member the crate binds now stays named a gap in builtins/gaps.txt",
    file,
    find: "list.filter((g) => !g.pattern.endsWith(\".*\") && g.pattern === m)",
    replace: "list.filter((g) => false)",
    tests,
  },
  {
    name: "gaps-root-pattern-exact",
    breaks: "`Temporal.*` names no member of `Temporal.Instant`, only one named `Temporal.*` itself",
    file,
    find: "  return member.startsWith(gap.pattern.slice(0, -1));",
    replace: "  return gap.pattern === member;",
    tests,
  },
];
