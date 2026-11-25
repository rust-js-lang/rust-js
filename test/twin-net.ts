// The twin net (#1205, ADR 1122): the runtime's hand-written copies of
// std against std's own bodies, compiled by rust-js. The tables'
// generated programs, loud items and panic points, run in the plain
// build, whose recognizers lower to the runtime's helpers, and in the
// build of std's iterator structs alone (`--std-iterators`, ADR 0908),
// which is the oracle: `ITERATOR_ORACLE=std` of
// test/iterator-matrix.test.ts. No native run: each push of the iterators'
// code or the runtime's runs each table's slice (`bun
// scripts/iteratorSlice.ts run twins`, check.yml's `nets` job's `twins` step), the night
// all of it (`run twinsAll`, nightly.yml's `twins`).
//
// A helper is pitted where a case's plain build imports it and its std
// build doesn't: std's body ran in its place. Each other helper the plain
// build imports, the std build imports too, so no case pits it: the one
// catalog of the runtime's models, models.rs, which the compiler prints,
// says why, glue or an unproven model's reason (ADR 1202). A run fails on
// a gap alone, a helper the catalog doesn't name, and of a whole run one
// it says is `Pitted` that no case pits now, coverage lost; a catalog out
// of date with no gap is drift, which `BLESS=1 bun test/twin-net.ts
// helpers` writes into models.rs, and the nightly offers as a patch, `bun
// run ci:bless nightly` (ADR 1122, amended). `bun test/twin-net.ts
// helpers` asks it of the last run's.

import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

import { twinHelpers } from "./iterator-matrix";
import { compiler } from "./packages-of";

/** The runtime's helpers a module of rust-js's imports, by name. */
export function importedHelpers(js: string): string[] {
  return [...js.matchAll(/^import \{([^}]*)\} from "@rust-js\/std";$/gm)].flatMap((m) => m[1].split(",").map((name) => name.trim()).filter(Boolean));
}

/** One helper of the runtime's as the one catalog says it
 * (crates/rust-js-lower/src/recognition/models.rs, ADR 1202): glue, or a
 * model of the std items it stands for, pitted or unproven for a reason. */
export type Entry = { helper: string; kind: "glue" | "model" | "form"; pitted: boolean; why: string | null; standsFor: string[]; note: string; heldBy: "twin" | "boundary" | "std-sites" };

/** The catalog's file, which a whole run's drift is blessed into. */
export const catalogFile = join(import.meta.dir, "..", "crates", "rust-js-lower", "src", "recognition", "models.rs");

let read: { models: Entry[]; why: Record<string, string> } | undefined;

/** The catalog as the compiler prints it, `rust-js --std-models`: no list of
 * its own here (test/architecture-models.test.ts). */
export function catalog(): { models: Entry[]; why: Record<string, string> } {
  if (read !== undefined) return read;
  const printed = spawnSync(compiler, ["--std-models"], { encoding: "utf8" });
  // A spawn that failed itself, a compiler not built, has no stderr: its error says why (#2624).
  if (printed.status !== 0) throw new Error(`\`${compiler} --std-models\` failed: ${printed.error?.message ?? printed.stderr}`);
  read = JSON.parse(printed.stdout) as { models: Entry[]; why: Record<string, string> };
  return read;
}

/** Each helper the catalog says the twin net pits against std's body. */
export function pittedHelpers(): string[] {
  return catalog().models.filter((entry) => entry.pitted).map((entry) => entry.helper);
}

/** Each helper both builds import, by why no case pits it against std's
 * body: glue's, or an unproven model's reason. A form the lowering writes
 * is no helper of the runtime's, and isn't one. */
export function sharedHelpers(): Record<string, string> {
  return Object.fromEntries(catalog().models.filter((entry) => entry.kind !== "form" && !entry.pitted).map((entry) => [entry.helper, entry.why ?? "glue"]));
}

/** Each model that stands for no std item yet: #2513's measure of what's
 * left to bind. */
export function unboundModels(): string[] {
  return catalog().models.filter((entry) => entry.kind === "model" && entry.standsFor.length === 0).map((entry) => entry.helper);
}


/** Of a run's `target/twin-net/*.json`, what each build of each batch
 * imported: the helpers some case pitted, and those the plain build
 * imported and none pitted. */
export function helpersOf(dir = twinHelpers): { pitted: Set<string>; unpitted: Set<string>; judged: number; of: number } {
  const pitted = new Set<string>();
  const used = new Set<string>();
  let judged = 0;
  let of = 0;
  // In its batches' order, not the directory's, which Linux's ext4 gives
  // by its hash, so the helpers come in one order everywhere.
  for (const file of existsSync(dir) ? readdirSync(dir).sort() : []) {
    const batch: { judged: number; of: number; plain: string[]; std: string[] } = JSON.parse(readFileSync(join(dir, file), "utf8"));
    judged += batch.judged;
    of += batch.of;
    const std = new Set(batch.std);
    for (const helper of batch.plain.filter((name) => name.startsWith("$"))) {
      used.add(helper);
      if (!std.has(helper)) pitted.add(helper);
    }
  }
  return { pitted, unpitted: new Set([...used].filter((helper) => !pitted.has(helper))), judged, of };
}

/** What a run says of the catalog's pitted and unproven helpers (ADR
 * 1122, amended; ADR 1202).
 * - `gaps`, which fail it: a helper both builds import that neither list
 *   names; and of a whole run, `whole`, one it says is pitted that no case
 *   pits now, coverage lost, which a slice can't say.
 * - `drift`, of a whole run alone: a list out of date that hides no gap,
 *   which fails nothing and `blessedCatalog` writes. Each helper a case pits
 *   that `pitted` doesn't list goes on it, `pit`; each `shared` lists that
 *   a case pits now, or that nothing imports and the twin net holds
 *   (`heldBy`), comes off it, `unshare`. A row another net holds, the
 *   boundary's tests or `bun run std`'s sites, is that net's to answer for.
 *
 * The nightly's twin net was red four nights running to 2026-11-24 of
 * drift alone, each fixed by hand the next morning. */
export function twinVerdict(run: { pitted: Set<string>; unpitted: Set<string> }, whole: boolean): { gaps: string[]; drift: string[]; pit: string[]; unshare: string[] } {
  const pitted = pittedHelpers();
  const shared = sharedHelpers();
  const listed = new Set(pitted);
  const gaps = [...run.unpitted]
    .filter((helper) => shared[helper] === undefined && !listed.has(helper))
    .map((helper) => `${helper}: both builds import it and models.rs doesn't say why std's build has no body of its own in its place`);
  if (!whole) return { gaps: gaps.sort(), drift: [], pit: [], unshare: [] };
  for (const helper of pitted.filter((helper) => !run.pitted.has(helper))) {
    gaps.push(
      run.unpitted.has(helper)
        ? `${helper}: models.rs says it's \`Pitted\` and both builds import it now, so no case pits it against std's body: coverage lost: pit it again, or make it \`Unproven\` saying why`
        : `${helper}: models.rs says it's \`Pitted\` and no case imports it now: coverage lost: give a table a case that reaches it, or take it off models.rs where nothing makes it`,
    );
  }
  const pit = [...run.pitted].filter((helper) => !listed.has(helper)).sort();
  // A row another net holds is no drift where no case imports it.
  const held = Object.fromEntries(catalog().models.map((entry) => [entry.helper, entry.heldBy]));
  const unshare = Object.keys(shared)
    .filter((helper) => run.pitted.has(helper) || (!run.unpitted.has(helper) && held[helper] === "twin"))
    .sort();
  const drift = [
    ...pit.map((helper) => (shared[helper] === undefined ? `${helper}: a case pits it against std's body: onto models.rs, \`Pitted\`` : `${helper}: a case pits it against std's body now: \`Pitted\` in models.rs`)),
    ...unshare.filter((helper) => !run.pitted.has(helper)).map((helper) => `${helper}: no case imports it now: off models.rs`),
  ];
  return { gaps: gaps.sort(), drift: drift.sort(), pit, unshare };
}

/** The catalog's text, `source`, models.rs', with a whole run's drift
 * written into it: each of `pit` `Pitted`, its std items kept, the twin
 * net's to hold then, a new one's line put in its place in order; each of
 * `unshare` that no case pits taken off; every other line as it was. */
export function blessedCatalog(source: string, { pit, unshare }: { pit: string[]; unshare: string[] }): string {
  // Each line's helper, a bare name there, the export `$` and it.
  const line = /^ {4}entry\("(\w+)", (Glue|Model\((?:Pitted|Unproven\(Why::\w+\)), (&\[[^\]]*\])\)), ("(?:[^"\\]|\\.)*")\)(?:\.held\(Net::\w+\))?,$/;
  const lines = source.split("\n");
  const at = new Map<string, number>();
  lines.forEach((text, i) => {
    const m = line.exec(text);
    if (m) at.set(`$${m[1]}`, i);
  });
  if (at.size === 0) throw new Error(`${catalogFile} has no catalog's lines to bless`);
  const pitted = new Set(pit);
  for (const helper of pit) {
    const i = at.get(helper);
    if (i === undefined) continue;
    const m = line.exec(lines[i])!;
    lines[i] = `    entry("${helper.slice(1)}", Model(Pitted, ${m[3] ?? "&[]"}), ${m[4]}),`;
  }
  const off = new Set(unshare.filter((helper) => !pitted.has(helper)));
  let kept = lines.filter((text) => {
    const m = line.exec(text);
    return !(m && off.has(`$${m[1]}`));
  });
  for (const helper of pit.filter((helper) => !at.has(helper))) {
    const made = `    entry("${helper.slice(1)}", Model(Pitted, &[]), ""),`;
    const after = kept.findIndex((text) => {
      const m = line.exec(text);
      return m !== null && `$${m[1]}` > helper;
    });
    const last = kept.findLastIndex((text) => line.test(text));
    kept = [...kept.slice(0, after < 0 ? last + 1 : after), made, ...kept.slice(after < 0 ? last + 1 : after)];
  }
  return kept.join("\n");
}

/** Where a run of the twin net says what it ran (`scripts/iteratorSlice.ts`):
 * whether it was the whole net, and passed, which alone its lists are blessed of. */
export const twinRun = join(dirname(twinHelpers), "twin-net-run.json");

/** Says a run's verdict: its gaps, and its drift with how to bless it.
 * Whether it has no gap. */
export function sayVerdict(verdict: { gaps: string[]; drift: string[] }): boolean {
  for (const gap of verdict.gaps) console.log(gap);
  if (verdict.drift.length > 0) {
    console.log(`the catalog drifted, no coverage lost: \`BLESS=1 bun test/twin-net.ts helpers\` writes it, models.rs; the nightly's \`bless-twin-net\`, \`bun run ci:bless nightly\``);
    for (const drift of verdict.drift) console.log(`  ${drift}`);
  }
  return verdict.gaps.length === 0;
}

/** `bun test/twin-net.ts helpers`: the last run's verdict, said, and with
 * `bless`, `BLESS=1`, its drift written into the catalog, `file`, of a
 * whole run that passed alone. Whether it has no gap. */
export function helpersCommand({ dir = twinHelpers, ran = twinRun, file = catalogFile, bless = false } = {}): boolean {
  const helpers = helpersOf(dir);
  if (helpers.of === 0) throw new Error(`no run's helpers in ${dir}: run \`bun scripts/iteratorSlice.ts run twins\``);
  const { whole, passed }: { whole: boolean; passed: boolean } = existsSync(ran) ? JSON.parse(readFileSync(ran, "utf8")) : { whole: false, passed: false };
  console.log(`${helpers.judged} of ${helpers.of} cases judged against std's bodies, ${whole ? "the whole net" : "a slice"}; ${helpers.pitted.size} helpers pitted: ${[...helpers.pitted].sort().join(" ")}`);
  const verdict = twinVerdict(helpers, whole);
  const ok = sayVerdict(verdict);
  if (bless && verdict.drift.length > 0) {
    // A slice's or a failed run's helpers aren't every case's: blessed,
    // they'd take off what the rest pit or import.
    if (!whole || !passed) throw new Error("only a whole run that passed is blessed: `bun scripts/iteratorSlice.ts run twinsAll`");
    writeFileSync(file, blessedCatalog(readFileSync(file, "utf8"), verdict));
    console.log(`blessed: ${verdict.drift.length} of the catalog's lines, in ${file}: rebuild the compiler, which prints it`);
  }
  return ok;
}

if (import.meta.main) {
  if (process.argv[2] !== "helpers") throw new Error("usage: [BLESS=1] bun test/twin-net.ts helpers");
  process.exit(helpersCommand({ bless: Boolean(process.env.BLESS) }) ? 0 : 1);
}
