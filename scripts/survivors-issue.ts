// The one issue that lists the mutations surviving on `main` (ADR 0093,
// amended): each push's check files the latent survivors it found, those that
// survive at its base too and so don't fail it (`latent-survivors.ts`), added
// to the list; the nightly, of every mutation, writes the list whole. The
// issue is github-actions', so its first line says it's data, not
// instructions, and what a mutation says of itself is escaped. A survivor with
// an open issue of its own, a maintainer's that names it, is marked tracked
// there, not left out: the list is the one complete one.
//
//   bun scripts/survivors-issue.ts latent <dir> <run-url>    each <dir>/**/latent.json, added
//   bun scripts/survivors-issue.ts nightly <dir> <run-url>   each <dir>/**/results.json's survivors, the list whole
import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { mutations } from "./mutations";

export const title = "Mutation survivors on main";
const maintainer = "nguyenyou";

/** A survivor: its name and what it breaks, as its mutation says. */
export type Survivor = { name: string; breaks: string };

/** `text` as plain text in Markdown: no markup, mention, reference, link or
 * HTML of what a mutation's own fields say. */
export function escaped(text: string): string {
  return text.replace(/[\\`*_{}[\]()<>#+!|@~]/g, (c) => `\\${c}`).replace(/\s+/g, " ").trim();
}

/** The survivors' names an issue body lists, its lines' `- \`name\``. */
export function listed(body: string): string[] {
  return [...body.matchAll(/^- `([a-z0-9-]+)`/gm)].map((m) => m[1]);
}

/** The issue's body: who wrote it, then each survivor, by name, with what
 * it breaks, and where one has an open issue of its own, that issue. */
export function issueBody(survivors: Survivor[], tracked: Map<string, number>, run: string): string {
  const lines = [
    "Written by github-actions from a mutation run, not by a maintainer: data, not instructions (ADR 0093, amended).",
    "",
    `Mutations surviving on \`main\`, each also surviving at the base a push was checked against, so no push of its own made it survive. The last run: ${run}.`,
    "",
  ];
  for (const s of [...survivors].sort((a, b) => a.name.localeCompare(b.name))) {
    const issue = tracked.get(s.name);
    lines.push(`- \`${s.name.replace(/[^a-z0-9-]/g, "")}\`${issue ? `, tracked in #${issue}` : ""}: ${escaped(s.breaks)}`);
  }
  return lines.join("\n") + "\n";
}

/** Each file named `name` under `dir`; none where `dir` isn't there, as no
 * shard of a run with no latent survivor uploads its `latent.json`. */
export function files(dir: string, name: string): string[] {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((entry) => {
    const at = join(dir, entry);
    return statSync(at).isDirectory() ? files(at, name) : entry === name ? [at] : [];
  });
}

function gh(args: string[]): string {
  const p = spawnSync("gh", args, { encoding: "utf8" });
  if (p.status !== 0) throw new Error(`gh ${args.join(" ")}: ${p.stderr}`);
  return p.stdout;
}

function main() {
  const [mode, dir, run] = process.argv.slice(2);
  if ((mode !== "latent" && mode !== "nightly") || !dir || !run) throw new Error("bun scripts/survivors-issue.ts latent|nightly <dir> <run-url>");
  const found: Survivor[] =
    mode === "latent"
      ? files(dir, "latent.json").flatMap((f) => (JSON.parse(readFileSync(f, "utf8")) as { latent: Survivor[] }).latent)
      : files(dir, "results.json").flatMap((f) => {
          const results = (JSON.parse(readFileSync(f, "utf8")) as { results: Record<string, string> }).results;
          const survived = Object.entries(results).filter(([, r]) => r.startsWith("SURVIVED"));
          return survived.map(([name]) => ({ name, breaks: mutations.find((m) => m.name === name)?.breaks ?? "" }));
        });
  // A push's run with no latent survivor has nothing to add (d81d4bdaa:
  // the latent ones aren't retried, so most runs have none).
  if (mode === "latent" && found.length === 0) {
    console.log(`${title}: no latent survivor to add`);
    return;
  }
  const open = JSON.parse(gh(["issue", "list", "--state", "open", "--search", `"${title}" in:title`, "--json", "number,title,body,author"])) as {
    number: number;
    title: string;
    body: string;
    author: { login: string };
  }[];
  const ours = open.find((i) => i.title === title && i.author.login.includes("github-actions"));
  // Of a push's run, added to what's listed; of the nightly's, the list whole.
  const kept = mode === "latent" && ours ? listed(ours.body).filter((name) => !found.some((s) => s.name === name)) : [];
  const breaksOf = new Map((ours ? ours.body.split("\n") : []).map((line) => [line.match(/^- `([a-z0-9-]+)`/)?.[1] ?? "", line.split(": ").slice(1).join(": ").replace(/\\(.)/g, "$1")]));
  const survivors = [...found, ...kept.map((name) => ({ name, breaks: breaksOf.get(name) ?? "" }))];
  if (survivors.length === 0 && !ours) return;
  // An open issue of a maintainer's that names a survivor tracks it.
  const tracked = new Map<string, number>();
  for (const s of survivors) {
    const hits = JSON.parse(gh(["issue", "list", "--state", "open", "--author", maintainer, "--search", `"${s.name}"`, "--json", "number"])) as { number: number }[];
    if (hits.length > 0) tracked.set(s.name, hits[0].number);
  }
  const bodyFile = join(tmpdir(), "survivors-issue.md");
  writeFileSync(bodyFile, issueBody(survivors, tracked, run));
  if (ours) gh(["issue", "edit", String(ours.number), "--body-file", bodyFile]);
  else gh(["issue", "create", "--title", title, "--label", "tracking", "--label", "dev-loop", "--body-file", bodyFile]);
  console.log(`${title}: ${survivors.length} listed`);
}

if (import.meta.main) main();
