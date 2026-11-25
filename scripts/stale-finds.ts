// Every mutation whose `find` no longer applies to its file, once, with
// the file's nearest text now, all at once (#2586): after a code move, a
// split or a relay, its finds are rewritten in one pass, not one run of
// test/mutations.test.ts' "every mutation applies" for each.
//
//   bun scripts/stale-finds.ts            # every list's
//   bun scripts/stale-finds.ts <names..>  # those mutations' alone

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { lists, mutate, type Mutation } from "./mutations";

const root = join(import.meta.dir, "..");

/** A stale find: its mutation, its list, why it doesn't apply, and where
 * its file's text now is nearest it, those lines and their first line's
 * number; none where no line of the find is there any more. */
export type Stale = { name: string; list: string; file: string; problem: string; near?: { line: number; text: string[] } };

/** Of `source`, the lines nearest `find`'s: the window of its line count
 * holding the most of its lines, by their text past indentation, each
 * where the find has it counting twice, the earliest of the best; none
 * where none of them is in `source`. */
export function nearest(source: string, find: string): { line: number; text: string[] } | undefined {
  const lines = source.split("\n");
  const wanted = find.replace(/\n$/, "").split("\n").map((line) => line.trim());
  const count = wanted.length;
  let best: { line: number; score: number } | undefined;
  for (let at = 0; at + count <= lines.length; at++) {
    const here = lines.slice(at, at + count).map((line) => line.trim());
    const window = new Set(here);
    const score = wanted.filter((line) => line !== "" && window.has(line)).length + wanted.filter((line, k) => line !== "" && here[k] === line).length;
    if (score > 0 && (best === undefined || score > best.score)) best = { line: at + 1, score };
  }
  // A single line rewritten: of one line, the line sharing most words.
  if (best === undefined && count === 1) {
    const tokens = new Set(wanted[0]!.match(/\w+/g) ?? []);
    if (tokens.size === 0) return undefined;
    for (let at = 0; at < lines.length; at++) {
      const score = (lines[at]!.match(/\w+/g) ?? []).filter((token) => tokens.has(token)).length;
      if (score > 0 && score * 2 >= tokens.size && (best === undefined || score > best.score)) best = { line: at + 1, score };
    }
  }
  return best === undefined ? undefined : { line: best.line, text: lines.slice(best.line - 1, best.line - 1 + count) };
}

/** Each mutation of `of` whose `find` doesn't apply to its file once, as
 * `read` gives the files. */
export function staleFinds(of: readonly { path: string; mutations: readonly Mutation[] }[], read: (file: string) => string | undefined): Stale[] {
  const stale: Stale[] = [];
  for (const list of of) {
    for (const m of list.mutations) {
      const source = read(m.file);
      if (source === undefined) {
        stale.push({ name: m.name, list: `scripts/mutations/${list.path}`, file: m.file, problem: `${m.file} isn't there` });
        continue;
      }
      const mutated = mutate(source, m);
      if (typeof mutated === "string") continue;
      // Where it applies more than once, it's there: no text is nearer.
      const near = source.includes(m.find) ? undefined : nearest(source, m.find);
      stale.push({ name: m.name, list: `scripts/mutations/${list.path}`, file: m.file, problem: mutated.problem, near });
    }
  }
  return stale;
}

/** What's said of each stale find: its name and list, why, and the find
 * beside the file's nearest text, `-` the find's lines, `+` the file's. */
export function said(stale: Stale, find: string): string {
  const out = [`${stale.name} (${stale.list}): ${stale.problem}`];
  if (stale.near === undefined) return [...out, "  no line of its find is in the file now"].join("\n");
  out.push(`  its find, and the nearest text now, at ${stale.file}:${stale.near.line}:`);
  return [...out, ...find.replace(/\n$/, "").split("\n").map((line) => `  - ${line}`), ...stale.near.text.map((line) => `  + ${line}`)].join("\n");
}

if (import.meta.main) {
  const names = new Set(process.argv.slice(2));
  const of = names.size === 0 ? lists : lists.map((list) => ({ ...list, mutations: list.mutations.filter((m) => names.has(m.name)) }));
  const unknown = [...names].filter((name) => !lists.some((list) => list.mutations.some((m) => m.name === name)));
  if (unknown.length > 0) {
    console.error(`no mutation named ${unknown.join(", ")}`);
    process.exit(2);
  }
  const stale = staleFinds(of, (file) => (existsSync(join(root, file)) ? readFileSync(join(root, file), "utf8") : undefined));
  const finds = new Map(lists.flatMap((list) => list.mutations.map((m) => [m.name, m.find] as const)));
  for (const each of stale) console.log(`${said(each, finds.get(each.name)!)}\n`);
  console.log(stale.length === 0 ? "every find applies, once" : `${stale.length} stale find${stale.length === 1 ? "" : "s"}`);
  process.exit(stale.length === 0 ? 0 : 1);
}
