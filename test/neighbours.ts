// The neighbour shuffle (#853): corpus cases compiled together, each one's
// `main` in a module of its own, called in turn by one driver, the crate
// run natively and as JS in both builds and compared. Each case runs as
// native Rust does alone (the corpus); a group that doesn't is one where a
// crate-wide or per-type fact one case makes leaks into another: #823,
// #834 and #850 were wrong only beside another function using the same
// type. Groups are a seed's hash of the cases that can share a crate, by
// name, so a run is reproduced from its seed and a case added changes one.
//
//   seed ─► hash ─► 128 groups ─┬─ rustc ──► native ─────┐
//                               └─ rust-js ─► node ──────┴─► the same?
//
// A group that differs is narrowed to the case whose part of the output
// differs, the victim, and the fewest neighbours that make it differ
// (`narrowed`), so what's filed is a pair.

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { cases, corpus, directives } from "./corpus-cases";

/** Why a case can't share a crate with others, or `undefined`: what its
 * text names that resolves at the crate's root, or that two cases would
 * both define there, or what it expects that a driver calling the next
 * case can't carry, a panic out of `main` or an abort. */
export function apart(source: string): string | undefined {
  const want = directives(source);
  if (typeof want === "string") return want;
  if (want.kind !== "run-pass") return want.kind;
  if (want.edition !== "2024") return `edition ${want.edition}`;
  if (want.native !== "host") return "native wasm32";
  const rooted: [string, RegExp][] = [
    ["an inner attribute", /^\s*#!\[/m],
    ["a path from the crate's root", /\bcrate::/],
    ["a macro's `$crate`", /\$crate\b/],
    ["a path from its parent", /\bsuper::/],
    ["an exported macro", /#\[macro_export\]/],
    ["an `extern crate`", /\bextern crate\b/],
    ["a module in another file", /^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*;/m],
    ["a symbol's own name", /\b(?:no_mangle|export_name)\b/],
  ];
  return rooted.find(([, pattern]) => pattern.test(source))?.[0];
}

/** The cases that can share a crate, in name order. */
export function neighbourly(): string[] {
  return cases.filter((name) => apart(readFileSync(join(corpus, name), "utf8")) === undefined);
}

/** A seeded generator of numbers in [0, 1): mulberry32. */
function random(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** How many groups a seed's shuffle makes, whatever the corpus's size: a
 * case added joins one of them, by its name, and leaves the others as they
 * were, so the groups a push runs change only where a case is added to them
 * (#853). Of 1,652 cases, about 13 a group. */
export const GROUPS = 128;

/** Seed `seed`'s groups of `names`: each case in the group its name and the
 * seed hash it to, `GROUPS` of them, each in name order. */
export function groups(names: string[], seed: number): string[][] {
  const out: string[][] = Array.from({ length: GROUPS }, () => []);
  for (const name of names) {
    const at = random(seed ^ Number(BigInt.asUintN(32, Bun.hash(name) as bigint)))() * GROUPS;
    out[Math.floor(at)].push(name);
  }
  return out.filter((group) => group.length > 0);
}

/** What marks the start of a case's part of the output. */
export const marker = (name: string) => `=== neighbour ${name}`;

/** The crate of `group`: each case a module, `n0`, `n1`.., its `main`
 * called by `neighbour`; and a driver, the crate's `main`, that prints
 * each case's marker and calls it, in turn. */
export function crateOf(group: string[], inlined = false): string {
  // Each case's text in its module where it's `inlined`, one file a
  // failure is reduced from (#1088), else `include!`d where it is.
  const of = (name: string) => (inlined ? readFileSync(join(corpus, name), "utf8") : `include!(${JSON.stringify(join(corpus, name))});`);
  const modules = group.map(
    (name, i) => `mod n${i} {\n    ${of(name)}\n    pub fn neighbour() {\n        main()\n    }\n}\n`,
  );
  const calls = group.map((name, i) => `    println!(${JSON.stringify(marker(name))});\n    n${i}::neighbour();\n`);
  return `// The neighbour shuffle's crate (#853): ${group.join(", ")}.\n${modules.join("")}\nfn main() {\n${calls.join("")}}\n`;
}

/** `stdout` cut at each case's marker: each case's part, by its name. */
export function parts(stdout: string, group: string[]): Map<string, string> {
  const out = new Map<string, string>();
  let rest = stdout;
  for (let i = 0; i < group.length; i++) {
    const start = rest.indexOf(`${marker(group[i])}\n`);
    if (start < 0) break;
    const after = rest.slice(start + marker(group[i]).length + 1);
    const end = i + 1 < group.length ? after.indexOf(`${marker(group[i + 1])}\n`) : -1;
    out.set(group[i], end < 0 ? after : after.slice(0, end));
    rest = end < 0 ? "" : after.slice(end);
  }
  return out;
}

/** The first case whose part of the output differs, native's against the
 * JS's, or `undefined` where each part is the same. */
export function victim(group: string[], native: string, js: string): string | undefined {
  const [a, b] = [parts(native, group), parts(js, group)];
  return group.find((name) => a.get(name) !== b.get(name));
}

/** The fewest of `others` beside which `fails` holds of `victim`: halved
 * while one half alone keeps it failing, then one by one, each dropped
 * where the rest still fail without it. */
export async function narrowed(
  victim: string,
  others: string[],
  fails: (group: string[]) => Promise<boolean>,
): Promise<string[]> {
  let kept = others;
  while (kept.length > 1) {
    const half = Math.ceil(kept.length / 2);
    const [first, second] = [kept.slice(0, half), kept.slice(half)];
    if (await fails([...first, victim])) kept = first;
    else if (await fails([...second, victim])) kept = second;
    else break;
  }
  for (const one of [...kept]) {
    if (kept.length === 1) break;
    const without = kept.filter((name) => name !== one);
    if (await fails([...without, victim])) kept = without;
  }
  return kept;
}

/** The crate of `group` written into `dir`, its path. */
export function writeCrate(dir: string, group: string[]): string {
  const file = join(dir, "neighbours.rs");
  writeFileSync(file, crateOf(group));
  return file;
}

/** A pair the shuffle found going wrong, listed under its issue: its
 * builds, its victim, `undefined` of a crash that points at none, its
 * neighbours, and whether it's slow rather than wrong. */
export type Known = { issue: string; builds: string[]; victim: string | undefined; beside: string[]; slow: boolean };

export const knownFile = join(import.meta.dir, "neighbours-known.txt");

/** test/neighbours-known.txt's pairs, or why a line isn't one. */
export function readKnown(text = readFileSync(knownFile, "utf8")): Known[] {
  const known: Known[] = [];
  let issue: string | undefined;
  for (const line of text.split("\n")) {
    if (line === "" || line.startsWith("//")) continue;
    const heading = /^#(\d+) /.exec(line);
    if (heading) {
      issue = heading[1];
      continue;
    }
    const words = line.split(" ");
    const at = words.indexOf("beside");
    const builds = words.slice(0, at - 1);
    const slow = words.at(-1) === "slow";
    const beside = words.slice(at + 1, slow ? -1 : undefined);
    if (issue === undefined || at < 2 || builds.length === 0 || builds.some((b) => b !== "plain" && b !== "std") || beside.length === 0) {
      throw new Error(`test/neighbours-known.txt: \`${line}\` isn't \`<builds> <case>|- beside <cases> [slow]\` under an \`#<issue>\``);
    }
    known.push({ issue, builds, victim: words[at - 1] === "-" ? undefined : words[at - 1], beside, slow });
  }
  return known;
}

/** The listed pair, of `known`, that a problem of `build`, `victim` beside
 * `beside` is, if any: the same victim and the same neighbours. */
export function listed(known: Known[], build: string, victim: string | undefined, beside: string[]): Known | undefined {
  const same = (a: string[], b: string[]) => a.length === b.length && a.every((name) => b.includes(name));
  return known.find((k) => k.builds.includes(build) && k.victim === victim && same(k.beside, beside));
}

// `bun test/neighbours.ts crate a.rs b.rs ..`: the crate of these cases, to
// run natively and as JS by hand.
if (import.meta.main) {
  const [command, ...names] = process.argv.slice(2);
  if (command !== "crate" || names.length === 0) throw new Error("`crate <case.rs> ..`: the crate of these corpus cases");
  process.stdout.write(crateOf(names));
}
