// The MIR intervals' rule ablation (ADR 1137's audit): each mutation of the
// intervals and what they recognize applied in turn, the compiler built,
// and the shadow (`--intervals-shadow`) run again over the corpus's cases
// that have a MIR-only line and over the bounds matrix's programs. A site
// whose MIR-only line a mutation takes away is one its rule proves; a
// matrix program whose shadow lines a mutation changes at all is one of its
// rows: of a `Vec`'s index THIR proves too, the rule's loss is a THIR-only
// line added, no MIR-only one taken. Merged, the shards say each rule's sites and rows: a rule
// with corpus sites and no matrix row is a gap the matrix must fill, and a
// site no rule's mutation takes away rests on the walk alone.
//
//   bun scripts/ablate.ts --shard=1/16 --out=ablation-1.json   # a shard's mutations
//   bun scripts/ablate.ts --list --shard=1/16                  # which, none run
//   bun scripts/ablate.ts --out=one.json intervals-unequal-unnarrowed   # one, by name
//   bun scripts/ablate.ts --merge ablation-*.json --out=ablation.md
//
// It writes each mutation into the checkout and builds `target/debug`, so
// run it on CI (rustc-tests.yml's `ablation`) or in a worktree of its own.

import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { mutate, mutations, type Mutation } from "./mutations";
import { shard, shardArg } from "./shard";
import { directives } from "../test/corpus-cases";
import { programs } from "../test/bounds-matrix";

const root = join(import.meta.dir, "..");
const compiler = join(root, "target/debug/rust-js");

/** The mutations of the intervals' rules: the analysis's own files, what
 * they recognize of a loop's range and a key, and each a shadow test names. */
export const ablated = (m: Mutation) =>
  /^src\/lower\/analysis\/intervals/.test(m.file) ||
  /^src\/lower\/recognition\/(loop_ranges|stable_keys)\.rs$/.test(m.file) ||
  (m.tests ?? []).some((t) => t.includes("intervals-shadow"));

/** A shard's result: the corpus's MIR-only sites as the compiler is and
 * the matrix programs with any shadow line, and of each mutation the sites
 * it takes away and the programs whose lines it changes. */
type Shard = {
  baseline: { sites: string[]; rows: string[] };
  results: Record<string, { sites: string[]; rows: string[]; note?: string }>;
};

/** A shadow line's site, without the checkout's path. */
const site = (line: string) => line.split("\t").slice(0, 3).join("\t").replace(/[^\t]*\/test\/corpus\//, "");

async function shadow(
  files: { path: string; row?: string; edition: string }[],
  work: string,
): Promise<{ sites: Set<string>; rows: Map<string, string>; lined: Set<string> }> {
  const sites = new Set<string>();
  // Each matrix program's shadow lines, all of them, as one text.
  const rows = new Map<string, string>();
  // The files with a MIR-only line: a site in std's body is named by std's
  // path, not its case's.
  const lined = new Set<string>();
  let next = 0;
  await Promise.all(
    Array.from({ length: navigator.hardwareConcurrency }, async () => {
      while (next < files.length) {
        const file = files[next++];
        const log = join(work, `${next}.shadow`);
        writeFileSync(log, "");
        const proc = Bun.spawn([compiler, "--intervals-shadow", log, file.path, "-o", join(work, `${next}.js`), "--", `--edition=${file.edition}`], {
          stdout: "ignore",
          stderr: "ignore",
        });
        const timer = setTimeout(() => proc.kill(), 120_000);
        await proc.exited;
        clearTimeout(timer);
        const lines = readFileSync(log, "utf8").split("\n").filter(Boolean);
        if (file.row) {
          rows.set(file.row, lines.map(site).sort().join("\n"));
          continue;
        }
        for (const line of lines.filter((l) => l.startsWith("mir-only"))) {
          lined.add(file.path);
          sites.add(site(line));
        }
      }
    }),
  );
  return { sites, rows, lined };
}

function build(): boolean {
  return Bun.spawnSync(["cargo", "build", "-q"], { cwd: root, stdout: "ignore", stderr: "ignore" }).exitCode === 0;
}

/** The corpus's cases, each by its directives' edition, and the matrix's
 * programs, written to `work`. */
function inputs(work: string): { path: string; row?: string; edition: string }[] {
  const corpus = join(root, "test/corpus");
  const cases = [...new Bun.Glob("*.rs").scanSync(corpus)].sort().flatMap((name) => {
    const want = directives(readFileSync(join(corpus, name), "utf8"));
    return typeof want === "string" || want.kind === "compile-fail" ? [] : [{ path: join(corpus, name), edition: want.edition }];
  });
  const dir = join(work, "matrix");
  mkdirSync(dir, { recursive: true });
  const rows = Object.entries(programs()).map(([name, source]) => {
    const path = join(dir, `${name}.rs`);
    writeFileSync(path, source);
    return { path, row: name, edition: "2024" };
  });
  return [...cases, ...rows];
}

async function run(args: string[]) {
  const out = args.find((a) => a.startsWith("--out="))?.slice("--out=".length);
  const sharded = shardArg(args);
  const all = mutations.filter(ablated).map((m) => m.name);
  const named = args.filter((a) => !a.startsWith("--"));
  const unknown = named.filter((n) => !all.includes(n));
  if (unknown.length) throw new Error(`no ablated mutation ${unknown.join(", ")}`);
  const chosen = named.length ? named : all;
  const names = sharded ? shard(chosen, sharded.index, sharded.count) : chosen;
  if (args.includes("--list")) {
    for (const name of names) console.log(name);
    console.log(`${names.length} of ${all.length} mutations`);
    return;
  }
  if (!out) throw new Error("--out=<file>: where the shard's result goes");
  const work = mkdtempSync(join(tmpdir(), "ablate-"));
  try {
    await ablate(names, out, work);
  } finally {
    // The programs' JS and logs, removed however the run ends.
    rmSync(work, { recursive: true, force: true });
  }
}

async function ablate(names: string[], out: string, work: string) {
  if (!build()) throw new Error("the compiler as it is doesn't build");
  // The baseline over every case, then each mutation over those with a
  // MIR-only line and every matrix program: a mutation only takes lines away.
  const everything = inputs(work);
  const started = Date.now();
  const first = await shadow(everything, work);
  const files = everything.filter((f) => f.row || first.lined.has(f.path));
  const matrix = everything.filter((f) => f.row).length;
  const shown = [...first.rows].filter(([, lines]) => lines).length;
  console.log(`baseline: ${first.sites.size} sites, ${shown} of ${matrix} matrix programs with a line, ${Math.round((Date.now() - started) / 1000)} s`);
  const result: Shard = { baseline: { sites: [...first.sites].sort(), rows: [...first.rows.keys()].sort() }, results: {} };
  for (const name of names) {
    const m = mutations.find((x) => x.name === name)!;
    const path = join(root, m.file);
    const source = readFileSync(path, "utf8");
    const mutated = mutate(source, m);
    if (typeof mutated !== "string") {
      result.results[name] = { sites: [], rows: [], note: mutated.problem };
      console.log(`${name}\t${mutated.problem}`);
      continue;
    }
    writeFileSync(path, mutated);
    try {
      if (!build()) {
        result.results[name] = { sites: [], rows: [], note: "doesn't build" };
        console.log(`${name}\tdoesn't build`);
        continue;
      }
      const now = await shadow(files, work);
      const sites = result.baseline.sites.filter((s) => !now.sites.has(s));
      const rows = result.baseline.rows.filter((r) => now.rows.get(r) !== first.rows.get(r));
      result.results[name] = { sites, rows };
      console.log(`${name}\t-${sites.length} sites\t-${rows.length} rows`);
    } finally {
      writeFileSync(path, source);
    }
    writeFileSync(out, JSON.stringify(result, null, 1));
  }
  build();
  writeFileSync(out, JSON.stringify(result, null, 1));
}

/** The shards as one table: each rule's sites and rows by the mutation that
 * takes the fewest away, the finest; a whole kind's switch, which takes
 * every site of its kind, is no rule. */
function merge(files: string[], out: string) {
  const shards = files.map((f) => JSON.parse(readFileSync(f, "utf8")) as Shard);
  const results = Object.assign({}, ...shards.map((s) => s.results)) as Shard["results"];
  const baseline = shards[0].baseline;
  const kinds = new Map<string, number>();
  for (const s of baseline.sites) kinds.set(s.split("\t")[0], (kinds.get(s.split("\t")[0]) ?? 0) + 1);
  const isSwitch = (r: { sites: string[] }) =>
    [...kinds].some(([kind, n]) => n > 0 && r.sites.filter((s) => s.startsWith(`${kind}\t`)).length === n);
  const rules = Object.entries(results).filter(([, r]) => !r.note && !isSwitch(r));
  const finest = new Map<string, string>();
  for (const s of baseline.sites) {
    const proving = rules.filter(([, r]) => r.sites.includes(s)).sort((a, b) => a[1].sites.length - b[1].sites.length);
    if (proving.length) finest.set(s, proving[0][0]);
  }
  const lines = [
    `# The MIR intervals' rules, by ablation`,
    "",
    `${baseline.sites.length} MIR-only sites in the corpus, ${baseline.rows.length} matrix programs; ${Object.keys(results).length} mutations. A rule's matrix programs are those whose shadow lines its mutation changes.`,
    "",
    "| Rule (mutation) | Corpus sites, its finest | Corpus sites | Matrix programs |",
    "|---|---|---|---|",
  ];
  for (const [name, r] of rules.filter(([, r]) => r.sites.length || r.rows.length).sort((a, b) => b[1].sites.length - a[1].sites.length)) {
    const own = [...finest.values()].filter((n) => n === name).length;
    lines.push(`| ${name} | ${own} | ${r.sites.length} | ${r.rows.length} |`);
  }
  const gaps = rules.filter(([, r]) => r.sites.length > 0 && r.rows.length === 0).map(([n]) => n);
  const walk = baseline.sites.filter((s) => !finest.has(s));
  lines.push("", `## Gaps: corpus sites, no matrix row (${gaps.length})`, "", ...gaps.map((g) => `- ${g}`));
  lines.push("", `## Sites no rule's mutation takes away: the walk's own (${walk.length})`, "", ...walk.map((s) => `- ${s.replaceAll("\t", " ")}`));
  const noted = Object.entries(results).filter(([, r]) => r.note);
  if (noted.length) lines.push("", "## Mutations not measured", "", ...noted.map(([n, r]) => `- ${n}: ${r.note}`));
  writeFileSync(out, lines.join("\n") + "\n");
  writeFileSync(out.replace(/\.md$/, ".json"), JSON.stringify({ baseline, results, finest: Object.fromEntries(finest), gaps, walk }, null, 1));
  console.log(`${rules.length} rules; ${gaps.length} gaps; ${walk.length} sites the walk's own: ${out}`);
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  if (args[0] === "--merge") {
    const out = args.find((a) => a.startsWith("--out="))?.slice("--out=".length) ?? "ablation.md";
    merge(args.slice(1).filter((a) => !a.startsWith("--") && existsSync(a)), out);
  } else {
    await run(args);
  }
}
