// What JS made is read as often as Rust reads it, in Rust's order, however
// the JS is made readable (ADRs 1022, 1087): each read case's calls, each
// given counting `Proxy`s of the objects JS made, every read, write and
// call they see logged (`getter-reads-run.ts`), log the same after each of
// tidy's stages, without each of preparation's rewrites that changed the
// case, and with neither at its least. Native Rust has no getters, so the
// corpus can't see a getter read twice (#430, #485, #564): these cases can.
//
//   case.rs ── rust-js <each variant> ──► node, given proxies ──► each variant's log the same?
//
//   bun test test/getter-reads.test.ts
//
// A case, `test/getter-reads/<name>.rs`, is a library of the js crate's
// bindings; each `//@ call: <export> <JSON array of arguments>` is a call
// of what it exports, in order, an object argument a counting proxy, a
// `{ "$fn": value }` in one a function that gives `value`.

import { beforeAll, expect, test } from "bun:test";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const dir = join(root, "test", "getter-reads");
const readCases = readdirSync(dir)
  .filter((file) => file.endsWith(".rs"))
  .sort();

/** A case's calls, from its `//@ call:` lines. */
function calls(source: string): { fn: string; args: unknown[] }[] {
  return [...source.matchAll(/^\/\/@ call: (\w+) (.*)$/gm)].map(([, fn, args]) => ({ fn, args: JSON.parse(args) }));
}

/** The last pass of each of tidy's stages, `--tidy-until`'s. */
function stageEnds(): string[] {
  const ends = new Map<string, string>();
  for (const pass of run([compiler, "--tidy-passes"]).trim().split("\n")) ends.set(pass.slice(0, pass.lastIndexOf("/")), pass);
  return [...ends.values()];
}

/** `file` compiled with `flags` into `out`, its JS, or why not. */
function compiled(file: string, out: string, flags: string[]): string | { error: string } {
  const js = join(out, `${flags.join("-").replaceAll(/[^A-Za-z0-9-]+/g, "_").slice(0, 80) || "case"}-${Bun.hash(flags.join(" ")).toString(36)}.js`);
  const r = runSync([compiler, file, "-o", js, ...flags, "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target, "--edition=2024", "-Awarnings"], root, 120_000);
  return r.code === 0 ? js : { error: r.stderr.split("\n").slice(0, 6).join("\n") };
}

/** What `js`'s calls logged, and what each gave. */
function logged(js: string, out: string, callsFile: string): string {
  const logFile = `${js}.log.json`;
  run([node ?? "node", join(root, "test", "getter-reads-run.ts"), js, callsFile, logFile]);
  return readFileSync(logFile, "utf8");
}

test("each read case calls what it exports", () => {
  expect(readCases.length).toBeGreaterThan(1);
  for (const name of readCases) expect(calls(readFileSync(join(dir, name), "utf8")).length).toBeGreaterThan(0);
});

for (const name of readCases) {
  test(`${basename(name, ".rs")}'s reads are the same after each of tidy's stages and without each of preparation's rewrites`, () => {
    const file = join(dir, name);
    const out = fixture(`getter-reads-${basename(name, ".rs")}`);
    const callsFile = join(out, "calls.json");
    writeFileSync(callsFile, JSON.stringify(calls(readFileSync(file, "utf8"))));
    const firedFile = join(out, "fired.txt");
    const full = compiled(file, out, ["--prepare-fired", firedFile]);
    if (typeof full !== "string") throw new Error(full.error);
    const fired = readFileSync(firedFile, "utf8").split("\n").filter(Boolean);
    const ends = stageEnds();
    const everyRewrite = run([compiler, "--prepare-passes"]).trim().split("\n").flatMap((pass) => ["--prepare-without", pass]);
    // The least: tidy stopped after its first stage, and none of
    // preparation's rewrites, the reads closest to MIR's, which are Rust's.
    const least = compiled(file, out, ["--tidy-until", ends[0], ...everyRewrite]);
    if (typeof least !== "string") throw new Error(least.error);
    const want = logged(least, out, callsFile);
    // Each call does what Rust does, gives or panics: an error is JS's, a
    // call of what the case doesn't export, or the JS gone wrong.
    const { results } = JSON.parse(want) as { results: object[] };
    expect(results.filter((result) => "error" in result)).toEqual([]);
    const variants: [string, string[]][] = [
      ["as built", []],
      ...ends.slice(1).map((end): [string, string[]] => [`after ${end}`, ["--tidy-until", end]]),
      ...fired.map((pass): [string, string[]] => [`without preparation's ${pass}`, ["--prepare-without", pass]]),
    ];
    const problems: string[] = [];
    for (const [variant, flags] of variants) {
      const js = flags.length === 0 ? full : compiled(file, out, flags);
      // A stage refused, before tidy finishes, is a later phase's refusal
      // of a body tidy hasn't finished (ADR 0659, amended): no JS to read.
      if (typeof js !== "string") {
        if (flags[0] === "--tidy-until" && flags[1] !== ends.at(-1) && !js.error.includes("rust-js can't read")) continue;
        problems.push(`${variant}: rust-js failed: ${js.error}`);
        continue;
      }
      const got = logged(js, out, callsFile);
      if (got !== want) problems.push(`${variant}: logged\n${got}\nwhere the least JS logged\n${want}`);
    }
    expect(problems.join("\n\n")).toBe("");
  }, 600_000);
}
