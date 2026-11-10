// The neighbour shuffle (#853, `test/neighbours.ts`): a seed's groups of
// corpus cases, each group one crate, run natively and as JS in both
// builds. A group that differs is narrowed to its victim and the fewest
// neighbours beside which it differs, which is what's filed and listed in
// test/neighbours-known.txt; each listed pair is compiled alone too, so the
// list only shrinks.
//
//   bun test test/neighbours.test.ts                                     # the listed pairs, seed 0's first two groups
//   NEIGHBOURS_SEED=7 NEIGHBOURS=10 bun test test/neighbours.test.ts     # seed 7's first ten groups
//   NEIGHBOURS=all RUST_JS_CASES=1/6 bun test test/neighbours.test.ts    # all of seed 0's, a sixth
//   bun test/neighbours.ts crate a.rs b.rs                               # a pair's crate, by hand

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { crateOf, groups, listed, narrowed, neighbourly, readKnown, victim, writeCrate, type Known } from "./neighbours";
import { reducedRepro } from "./reduced";
import { agree, compileJs, runJs, runNative, runtimes, show, type Run } from "./programs";
import { buildCompiler, fixture } from "./support";
import { shard } from "../scripts/shard";
import { partSetting } from "./part-settings";

beforeAll(buildCompiler);

const seed = Number(process.env.NEIGHBOURS_SEED ?? "0");
const many = partSetting("NEIGHBOURS") ?? "2";
if (!Number.isInteger(seed) || seed < 0 || !/^(all|\d+)$/.test(many)) {
  throw new Error(`NEIGHBOURS=${many} NEIGHBOURS_SEED=${process.env.NEIGHBOURS_SEED}: \`all\` or how many groups, of a seed from 0`);
}
const all = groups(neighbourly(), seed);
// A push's run: seed 0's first two groups; the nightly's, all of the day's.
const count = many === "all" ? all.length : Number(many);
const [part, partsOf] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);
const chosen = shard(
  all.slice(0, count).map((_, i) => String(i).padStart(4, "0")),
  part,
  partsOf,
).map(Number);
const known = readKnown();

/** The two builds: the default, and std's iterator structs (ADR 0908). */
const builds: [string, string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];

/** `group` as one crate in a fixture of its own, run natively, or why it
 * couldn't be. */
function native(name: string, group: string[]): { dir: string; file: string; run: Run } | string {
  const dir = fixture(name);
  const file = writeCrate(dir, group);
  const run = runNative(file, dir);
  return typeof run === "string" ? run : { dir, file, run };
}

/** Its JS's run in `build`, or how its compile failed: in a folder of the
 * build's own, as each module of the crate is a file of its own, `n0.js`,
 * which another build's compile would write over. */
function js(dir: string, file: string, flags: string[], build: string): Run | string {
  const own = join(dir, build);
  mkdirSync(own, { recursive: true });
  const compiled = compileJs(file, own, "2024", false, {}, flags);
  if (!("js" in compiled)) return `${compiled.kind}: ${compiled.error.split("\n").slice(0, 6).join("\n")}`;
  const [runtime, cmd] = runtimes[0];
  return runJs(cmd, compiled.js, own, runtime);
}

/** What a compile's failure is, whichever crate it's of: its first error,
 * or where rust-js panicked and with what, each module's name, `n12`, each
 * `DefId` and each number the run gave made alike. */
function failure(error: string): string {
  const lines = error.split("\n");
  const at = lines.findIndex((line) => line.startsWith("error") || / panicked at /.test(line));
  const said = / panicked at /.test(lines[at] ?? "") ? `${lines[at]} ${lines[at + 1] ?? ""}` : (lines[at] ?? error);
  return said
    .replaceAll(/\bn\d+::/g, "n::")
    .replaceAll(/DefId\([^)]*\)/g, "DefId")
    .replaceAll(/\(\d+\)|\[\d+\]/g, "");
}

/** The fewest of `group` that still fail: each dropped in turn where the
 * rest fail without it. */
async function fewest(group: string[], fails: (group: string[]) => Promise<boolean>): Promise<string[]> {
  let kept = group;
  for (const one of group) {
    const without = kept.filter((name) => name !== one);
    if (without.length > 0 && (await fails(without))) kept = without;
  }
  return kept;
}

/** `group`'s crate as one file, each case's text in its module, in a
 * fixture of its own: what a failure's reduced from (#1088). */
function inlinedCrate(group: string[]): string {
  const dir = fixture(`neighbours-reduced-${Bun.hash(group.join(" ")).toString(36)}`);
  const file = join(dir, "neighbours.rs");
  writeFileSync(file, crateOf(group, true));
  return file;
}

/** The first lines where two outputs part. */
function parting(a: string, b: string): string {
  const [x, y] = [a.split("\n"), b.split("\n")];
  const at = x.findIndex((line, i) => line !== y[i]);
  return `native: ${JSON.stringify(x.slice(at, at + 3))}\njs:     ${JSON.stringify(y.slice(at, at + 3))}`;
}

for (const index of chosen) {
  const group = all[index];
  test(`seed ${seed} group ${index}: ${group.length} cases`, async () => {
    const whole = native(`neighbours-${seed}-${index}`, group);
    // A case the shuffle let in that can't share a crate: the shuffle's
    // own fault, `apart`'s to say.
    if (typeof whole === "string") throw new Error(`the group doesn't build natively: ${whole.slice(0, 600)}`);
    const problems: string[] = [];
    for (const [build, flags] of builds) {
      const ran = js(whole.dir, whole.file, flags, build);
      if (typeof ran === "string") {
        // Refused together, though each compiles alone: the case its first
        // error points at, and the fewest neighbours beside which it's
        // refused or crashes so, its signature the same; of one that points
        // at none, the fewest of the group that still do.
        const named = /--> test\/corpus\/([\w-]+\.rs)/.exec(ran)?.[1];
        const signature = failure(ran);
        const refused = async (subset: string[]) => {
          const ordered = group.filter((name) => subset.includes(name));
          const key = Bun.hash(ordered.join(" ")).toString(36);
          const dir = fixture(`neighbours-${seed}-${index}-${key}`);
          const it = js(dir, writeCrate(dir, ordered), flags, build);
          return typeof it === "string" && failure(it) === signature;
        };
        const beside =
          named !== undefined && group.includes(named)
            ? await narrowed(named, group.filter((name) => name !== named), refused)
            : await fewest(group, refused);
        const victimOf = named !== undefined && group.includes(named) ? named : undefined;
        if (!listed(known, build, victimOf, beside)) {
          const repro = reducedRepro(inlinedCrate([...(victimOf === undefined ? [] : [victimOf]), ...beside]), build);
          problems.push(`${build}: ${victimOf ?? "-"} refused beside ${beside.join(" ")}, though each compiles alone: ${ran}${repro}`);
        }
        continue;
      }
      if (agree(ran, whole.run)) continue;
      const hurt = victim(group, whole.run.stdout, ran.stdout);
      // Narrowed in the group's order, each subset a crate of its own.
      const fails = async (subset: string[]) => {
        const ordered = group.filter((name) => subset.includes(name));
        const key = Bun.hash(ordered.join(" ")).toString(36);
        const alone = native(`neighbours-${seed}-${index}-${key}`, ordered);
        if (typeof alone === "string") return false;
        const it = js(alone.dir, alone.file, flags, build);
        if (typeof it === "string") return false;
        return hurt === undefined ? !agree(it, alone.run) : victim(ordered, alone.run.stdout, it.stdout) === hurt;
      };
      const others = group.filter((name) => name !== hurt);
      const beside = hurt === undefined ? group : await narrowed(hurt, others, fails);
      if (hurt !== undefined && listed(known, build, hurt, beside)) continue;
      const repro = reducedRepro(inlinedCrate(hurt === undefined ? group : [hurt, ...beside]), build);
      problems.push(
        (hurt === undefined
          ? `${build}: ${group.join(" ")} ended ${show(ran.outcome)} where native ended ${show(whole.run.outcome)}${ran.bytes.stderr.equals(whole.run.bytes.stderr) ? "" : `, its stderr ${JSON.stringify(ran.stderr.slice(0, 300))} where native's is ${JSON.stringify(whole.run.stderr.slice(0, 300))}`}`
          : `${build}: ${hurt} beside ${beside.join(" ")} runs otherwise than native, ending ${show(ran.outcome)}:\n${parting(whole.run.stdout, ran.stdout)}`)
          .concat(repro),
      );
    }
    // What isn't listed: an issue to file, and its pair to list under it,
    // `<builds> <case> beside <cases>`, or a line more under its issue.
    expect(problems.join("\n\n")).toBe("");
  }, 1_800_000);
}

/** Whether `pair`'s crate still goes wrong in `build`: refused or crashing,
 * running otherwise than native, or, of a slow one, running past a second,
 * where native's takes a few milliseconds. */
function stillWrong(pair: Known, build: string, flags: string[]): boolean {
  const cases = [...(pair.victim === undefined ? [] : [pair.victim]), ...pair.beside].sort();
  const crate = native(`neighbours-known-${Bun.hash(cases.join(" ")).toString(36)}`, cases);
  if (typeof crate === "string") throw new Error(`${cases.join(" ")} doesn't build natively: ${crate.slice(0, 400)}`);
  const started = Date.now();
  const ran = js(crate.dir, crate.file, flags, build);
  if (typeof ran === "string") return !pair.slow;
  return pair.slow ? Date.now() - started > 1000 || !agree(ran, crate.run) : !agree(ran, crate.run);
}

for (const pair of known) {
  test(`#${pair.issue}: ${pair.victim ?? "-"} beside ${pair.beside.join(" ")} still goes wrong`, () => {
    const fixed = builds.filter(([build]) => pair.builds.includes(build)).filter(([build, flags]) => !stillWrong(pair, build, flags));
    expect(fixed.map(([build]) => `${build} runs as Rust does now: take it off test/neighbours-known.txt (#${pair.issue})`)).toEqual([]);
  }, 300_000);
}

// The groups the shuffle found going wrong that a fix made run as Rust
// does, each by the cases it narrowed them to, compiled together each push
// in each build, so a seed a commit names is run again: by the seed and the
// group that found it, a group shifting as cases are added (#1111, #1009).
const regressions: { seed: number; group: number; issue: string; cases: string[] }[] = [
  { seed: 102, group: 1, issue: "1111", cases: ["fmt_error_as_a_value.rs", "std_inspect.rs"] },
  { seed: 145, group: 1, issue: "1111", cases: ["failing_io_writer.rs", "std_inspect.rs"] },
  { seed: 57, group: 1, issue: "1009", cases: ["eq_of_identities_inside.rs", "rc_of_param_unnamed.rs"] },
];

for (const { seed, group, issue, cases } of regressions) {
  test(`seed ${seed} group ${group}'s ${cases.join(" and ")} run as Rust does together (#${issue})`, () => {
    const crate = native(`neighbours-regression-${Bun.hash(cases.join(" ")).toString(36)}`, [...cases].sort());
    if (typeof crate === "string") throw new Error(`${cases.join(" ")} doesn't build natively: ${crate.slice(0, 400)}`);
    const wrong = builds.flatMap(([build, flags]) => {
      const ran = js(crate.dir, crate.file, flags, build);
      return typeof ran === "string" ? [`${build}: ${ran.slice(0, 400)}`] : agree(ran, crate.run) ? [] : [`${build}: runs otherwise than native Rust`];
    });
    expect(wrong).toEqual([]);
  }, 300_000);
}

// The harness's own parts, which a wrong answer's report rests on.
test("a seed's groups are the same each time, hold each case once, and a case added changes one", () => {
  const names = Array.from({ length: 1600 }, (_, i) => `c${i}.rs`);
  const [a, b] = [groups(names, 3), groups(names, 3)];
  expect(a).toEqual(b);
  expect(a.flat().sort()).toEqual([...names].sort());
  expect(a.every((group) => group.length >= 3 && group.length <= 30)).toBe(true);
  expect(groups(names, 4)).not.toEqual(a);
  const more = groups([...names, "added.rs"], 3);
  expect(more.filter((group, i) => JSON.stringify(group) !== JSON.stringify(a[i]))).toHaveLength(1);
});

test("the victim is the first case whose part differs, and narrowing finds its neighbour", async () => {
  const group = ["a.rs", "b.rs", "c.rs"];
  const out = (b: string) => `=== neighbour a.rs\n1\n=== neighbour b.rs\n${b}\n=== neighbour c.rs\n3\n`;
  expect(victim(group, out("2"), out("2"))).toBeUndefined();
  expect(victim(group, out("2"), out("two"))).toBe("b.rs");
  const others = Array.from({ length: 13 }, (_, i) => `n${i}.rs`);
  const asked: string[][] = [];
  const fails = async (subset: string[]) => (asked.push(subset), subset.includes("n9.rs"));
  expect(await narrowed("v.rs", others, fails)).toEqual(["n9.rs"]);
  expect(asked.length).toBeLessThan(10);
});

test("a listed pair is read under its issue, and a malformed line is refused", () => {
  const read = readKnown("// x\n#7 P0: a title\nplain std a.rs beside b.rs c.rs\nstd - beside d.rs e.rs slow\n");
  expect(read).toEqual([
    { issue: "7", builds: ["plain", "std"], victim: "a.rs", beside: ["b.rs", "c.rs"], slow: false },
    { issue: "7", builds: ["std"], victim: undefined, beside: ["d.rs", "e.rs"], slow: true },
  ]);
  expect(listed(read, "std", "a.rs", ["c.rs", "b.rs"])?.issue).toBe("7");
  expect(listed(read, "std", "a.rs", ["b.rs"])).toBeUndefined();
  expect(() => readKnown("plain a.rs beside b.rs\n")).toThrow();
  expect(() => readKnown("#7 t\nnative a.rs beside b.rs\n")).toThrow();
});
