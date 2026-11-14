// Which net or method found each wrong answer (#1206): every P0 carries
// one `found-by: …` label, as docs/bug-kinds.md defines them, so where
// wrong answers come from is a count, not a reading.
//
//   bun scripts/found-by.ts                 the P0s found by each net, by week
//   bun scripts/found-by.ts --since=2026-11-01
//   bun scripts/found-by.ts --backfill      each unlabelled P0's net, read from its body
//   bun scripts/found-by.ts --backfill --apply   and labels each one read
//
// `--backfill` reads the sentence that says how a P0 was found, "Found by
// the metamorphic net's `library` rewrite…", and lists those it can't
// classify, for a person to label.

import { spawnSync } from "node:child_process";

/** The nets, each a `found-by:` label, in docs/bug-kinds.md's order. */
export const nets = [
  "corpus",
  "metamorphic",
  "consumer-net",
  "neighbour",
  "object-model",
  "tables",
  "fuzz",
  "crater",
  "crate-corpus",
  "port",
  "audit",
  "review",
  "mutations",
  "rustc-tests",
  "validator",
  "unknown",
] as const;
export type Net = (typeof nets)[number];

type Issue = { number: number; title: string; createdAt: string; body: string; labels: { name: string }[] };

const P0 = "P0: wrong answer";
const prefix = "found-by: ";

/** How a body says each net found it, the first that matches its sentence
 * of how it was found: the nets named by what they're called, before the
 * methods, an audit's "while fixing #N", whose sentences may name a net
 * they were probing beside. */
const said: [Net, RegExp][] = [
  // Found while fixing a sibling, before any net its sentence names: what
  // was being fixed, or the probe beside it, found it.
  ["audit", /^\W*(\d\.\s*)?(\**how (it was )?found\**:?\**\s*)?(found\s+)?(while|when|during|beside|fixing|writing)\b/i],
  // A probe by hand, or an agent at work on another issue, whatever net
  // it then names.
  ["audit", /^\W*(\d\.\s*)?\**how (it was )?found\**:?\**\s*(by hand|a (hand )?probe|probe)\b/i],
  ["audit", /^\W*found (on main at \w+ )?by [\w-]+'s #\d+/i],
  ["consumer-net", /consumer(s'?)? net|ITERATOR_TABLE=consumers/i],
  ["object-model", /object[- ]model net|#891\b/i],
  ["neighbour", /neighbou?r(s)? (net|shuffle)|#853\b|test\/neighbours/i],
  ["metamorphic", /metamorphic|#939\b|#1008\b|EMI net|#1086\b/i],
  // A tree the check of every compile refused, whatever suite compiled it
  // (ADR 1143).
  ["validator", /js::validate|\bvalidator\b|scope check of every tree|ADR 1143/i],
  ["crate-corpus", /crate[- ]corpus/i],
  ["mutations", /surviv(ing|ed|or)s? (of )?(a |the )?mutation|mutation (that )?survived|mutation survivors?/i],
  ["rustc-tests", /rustc's (own )?(run-pass )?tests|rustc tests?\b|rustc \/ report|test:rustc|rustc-known-failures/i],
  ["crater", /\bcrater/i],
  ["fuzz", /fuzz|generated program|program-stages|\bseed \d{5,}/i],
  [
    "tables",
    /iterator[- ]matrix|conformance (table|matrix|rows)|points table|unwind table|ITERATOR_(MATRIX|TABLE)|`&mut` matrix|mut-ref-matrix|None matrix|numbers grid|matrix's|#817\b|#818\b|#854\b|#730's|#731\b/i,
  ],
  ["port", /\bport\b|porter|react-in-rust|alien-signals|names run/i],
  ["corpus", /by the corpus|ITERATOR_FORMS=corpus|by `?test\/[\w-]+\.test\.ts|CI Check|corpus test/i],
  [
    "audit",
    /\b(while|when|during) (fixing|finishing|working|writing|doing|probing|mapping|porting|lifting|closing|checking|refactoring|migrating|restoring|extending|landing|studying|testing|lowering|analysing|analyzing|splitting|chasing|investigating|building|auditing)|found (fixing|writing|while)|audit|probe|grid|bindings run|by run \d+|'s #\d+ agent|std-protocol agent|leftovers agent|iterator-drop agent|^\W*(split from|left (by|after)) #\d+/i,
  ],
  ["review", /review|reported by|the user's report/i],
];

/** The sentence of `body` that says how its issue was found: one that
 * says "found", "How it was found" or "How found". */
export function foundSentence(body: string): string | undefined {
  const lines = body.split("\n").map((line) => line.trim()).filter(Boolean);
  return (
    lines.find((line) => /how (it was )?found/i.test(line)) ??
    lines.find((line) => /^\W*found\b/i.test(line)) ??
    lines.find((line) => /\bfound (by|while|with|in|from|on|fixing|writing|beside|during)\b/i.test(line)) ??
    lines.find((line) => /^\W*(split from|left (by|after)|seen (by|while|with|in)|reported by|from the nightly)\b/i.test(line))
  );
}

/** The net that found the issue whose body is `body`, as it says it, or
 * `undefined` where it doesn't say. */
export function classify(body: string): Net | undefined {
  const sentence = foundSentence(body);
  if (sentence === undefined) return undefined;
  return said.find(([, re]) => re.test(sentence))?.[0];
}

function gh(args: string[]): string {
  const p = spawnSync("gh", args, { encoding: "utf8", maxBuffer: 256 * 1024 * 1024, timeout: 120_000 });
  if (p.status !== 0) throw new Error(`gh ${args.join(" ")}: ${p.stderr}`);
  return p.stdout;
}

function p0s(since: string, withBodies: boolean): Issue[] {
  const fields = withBodies ? "number,title,createdAt,labels,body" : "number,title,createdAt,labels";
  const out = gh(["issue", "list", "--state", "all", "--label", P0, "--search", `created:>=${since}`, "--limit", "2000", "--json", fields]);
  return (JSON.parse(out) as Issue[]).map((issue) => ({ ...issue, body: issue.body ?? "" }));
}

/** The issue's `found-by:` net, from its labels. */
function labelled(issue: Issue): Net | "none" {
  const label = issue.labels.map((l) => l.name).find((name) => name.startsWith(prefix));
  return (label?.slice(prefix.length) as Net | undefined) ?? "none";
}

/** The Monday of the week `iso` falls in, `2026-11-02`. */
export function weekOf(iso: string): string {
  const day = new Date(iso.slice(0, 10) + "T00:00:00Z");
  day.setUTCDate(day.getUTCDate() - ((day.getUTCDay() + 6) % 7));
  return day.toISOString().slice(0, 10);
}

function report(issues: Issue[]) {
  const weeks = [...new Set(issues.map((issue) => weekOf(issue.createdAt)))].sort();
  const rows: (Net | "none")[] = [...nets, "none"];
  const count = (net: Net | "none", week?: string) =>
    issues.filter((issue) => labelled(issue) === net && (week === undefined || weekOf(issue.createdAt) === week)).length;
  const header = ["net", ...weeks.map((week) => `w/c ${week}`), "total"];
  const table = rows
    .map((net) => [net, ...weeks.map((week) => String(count(net, week))), String(count(net))])
    .filter((row) => row[row.length - 1] !== "0");
  table.push(["all", ...weeks.map((week) => String(issues.filter((i) => weekOf(i.createdAt) === week).length)), String(issues.length)]);
  const widths = header.map((_, i) => Math.max(header[i].length, ...table.map((row) => row[i].length)));
  const line = (row: string[]) => row.map((cell, i) => (i === 0 ? cell.padEnd(widths[i]) : cell.padStart(widths[i]))).join("  ");
  console.log(line(header));
  for (const row of table) console.log(line(row));
}

function backfill(issues: Issue[], apply: boolean) {
  const unlabelled = issues.filter((issue) => labelled(issue) === "none");
  const unread: Issue[] = [];
  for (const issue of unlabelled.sort((a, b) => a.number - b.number)) {
    const net = classify(issue.body);
    if (net === undefined) {
      unread.push(issue);
      continue;
    }
    console.log(`#${issue.number}\t${net}\t${(foundSentence(issue.body) ?? "").slice(0, 120)}`);
    if (apply) gh(["issue", "edit", String(issue.number), "--add-label", prefix + net]);
  }
  console.log(`\n${unlabelled.length - unread.length} read, ${unread.length} not:`);
  for (const issue of unread) console.log(`#${issue.number}\t${issue.title.slice(0, 110)}`);
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const since = args.find((arg) => arg.startsWith("--since="))?.slice("--since=".length) ?? "2026-11-01";
  if (args.includes("--backfill")) backfill(p0s(since, true), args.includes("--apply"));
  else report(p0s(since, false));
}
