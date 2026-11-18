// What the js crate doesn't bind of TypeScript's ESNext library, each with
// its issue: builtins/gaps.txt, the one list. A line is a member, or
// `Root.*`, every member of that root and of the roots in it, `Temporal.*`
// (#1903, ADR 0283).

import { readFileSync } from "node:fs";
import { join } from "node:path";

export type Gap = { pattern: string; issue: number };

/** Each line of builtins/gaps.txt. */
export function gaps(): Gap[] {
  const text = readFileSync(join(import.meta.dir, "gaps.txt"), "utf8");
  return text
    .split("\n")
    .filter((line) => line && !line.startsWith("#"))
    .map((line) => {
      const [pattern, issue] = line.split("\t");
      const n = Number(issue?.match(/^#(\d+)$/)?.[1]);
      if (!pattern || !Number.isInteger(n)) throw new Error(`builtins/gaps.txt: \`${line}\` isn't \`member<TAB>#issue\``);
      return { pattern, issue: n };
    });
}

/** Whether `gap` names `member`: itself, or of a root `Root.*` covers. */
export function names(gap: Gap, member: string): boolean {
  if (!gap.pattern.endsWith(".*")) return gap.pattern === member;
  return member.startsWith(gap.pattern.slice(0, -1));
}

/** What's wrong with the gaps of a measure, `all` and `covered`: each
 * unbound member no gap names, each gap that names none, and each bound
 * member a gap names. */
export function problems({ all, covered }: { all: string[]; covered: string[] }, list = gaps()): string[] {
  const bound = new Set(covered);
  const unbound = all.filter((m) => !bound.has(m));
  return [
    ...unbound.filter((m) => !list.some((g) => names(g, m))).map((m) => `${m}: unbound, and no line of builtins/gaps.txt names its issue`),
    ...list.filter((g) => !unbound.some((m) => names(g, m))).map((g) => `${g.pattern}: builtins/gaps.txt names it, but nothing it names is unbound`),
    ...covered.flatMap((m) => list.filter((g) => !g.pattern.endsWith(".*") && g.pattern === m).map(() => `${m}: bound, and builtins/gaps.txt still names it`)),
  ];
}
