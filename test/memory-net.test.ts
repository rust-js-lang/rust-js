// The memory net (#1172): each seed's program of unsafe Rust over plain
// data (test/memory-net.ts writes them), run under Miri, which must find
// it defined, natively on `wasm32-wasip1`, whose `usize` and layout are
// rust-js's (ADR 0090), and as JS in both builds, the default and std's
// iterator structs (ADR 0908), which must print and end as native Rust
// does. One that doesn't is a wrong answer: it's written to
// target/memory-net/ with what each run printed, and reduced (#1088). What
// rust-js refuses is counted by its reason, each part's in
// target/memory-net/refused-<part>.json, so the model's next pieces are
// chosen by need. What isn't yet as native Rust's is in
// test/memory-net-known.txt by what it is, not by seed: a refusal by its
// reason, a wrong answer by the form of the operation that first prints
// otherwise, so a known one passes in any seed while a new one fails. The
// slice and the nightly read it alike. Each push runs the slice where the
// pointers' code changes; the nightly 400 seeds from the day's
// (`MEMORY_NET=all`).
//
//   bun test test/memory-net.test.ts                                                        # the slice
//   MEMORY_NET=all MEMORY_NET_START=5000 RUST_JS_CASES=1/4 bun test test/memory-net.test.ts  # 400 seeds from 5000, a quarter

import { afterAll, beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { memoryProgram } from "./memory-net";
import { miriInstalled, undefinedBehaviour, installMiri } from "./miri";
import { partSetting } from "./part-settings";
import { agree, compileJs, runJs, runNative, runtimes, show, type Run } from "./programs";
import { reducedRepro } from "./reduced";
import { buildCompiler, contentDirectory, fixture, root, target, writeWhole } from "./support";

beforeAll(buildCompiler);

const which = partSetting("MEMORY_NET");
if (which !== undefined && which !== "all") throw new Error(`MEMORY_NET=${which}: \`all\`, or unset for the slice`);
const startSetting = process.env.MEMORY_NET_START;
if (startSetting !== undefined && !/^\d+$/.test(startSetting)) throw new Error(`MEMORY_NET_START=${startSetting}: a whole number`);

/** The seeds each push runs where the pointers' code changes. */
const slice = Array.from({ length: 24 }, (_, i) => i + 1);
/** How many seeds the nightly runs, from the day's. */
const nightly = 400;
const start = Number(startSetting ?? 1);
const seeds = which === "all" ? Array.from({ length: nightly }, (_, i) => start + i) : slice;
const [part, parts] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);
const chosen = seeds.filter((_, i) => i % parts === part - 1);

/** What rust-js doesn't yet do as native Rust does, filed, a line each of
 * test/memory-net-known.txt: a refusal whose reason holds `signature`, or a
 * wrong answer first printed by an operation of form `signature`. */
type Known = { kind: "refused" | "wrong"; signature: string; line: string };
export function knownOf(text: string): Known[] {
  return text
    .split("\n")
    .filter((line) => line.trim() !== "" && !line.startsWith("#"))
    .map((line) => {
      const parsed = /^(refused|wrong) (.+): #\d+ .+$/.exec(line);
      if (parsed === null) throw new Error(`test/memory-net-known.txt: \`${line}\` isn't \`refused <its reason's words>: #<issue> <why>\` nor \`wrong <family>/<form>: #<issue> <why>\``);
      return { kind: parsed[1] as Known["kind"], signature: parsed[2], line };
    });
}
const known = knownOf(readFileSync(join(root, "test", "memory-net-known.txt"), "utf8"));
/** How many programs met each known line, of this part. */
const met = new Map<string, number>();

/** The known line a refusal's reason or a wrong answer's form is, if any:
 * counted as met. */
function knownAs(kind: Known["kind"], what: string): Known | undefined {
  const found = known.find((k) => k.kind === kind && (kind === "refused" ? what.includes(k.signature) : what === k.signature));
  if (found !== undefined) met.set(found.line, (met.get(found.line) ?? 0) + 1);
  return found;
}

/** The form of the operation whose line a run first prints otherwise than
 * native Rust, or first leaves unprinted: each line starts with its
 * operation's number. None where every line is native's, and only how it
 * ended differs. */
export function firstOtherwise(native: string, ran: string, forms: string[]): string | undefined {
  const theirs = native.trimEnd().split("\n");
  const ours = ran.trimEnd().split("\n");
  const at = Array.from({ length: Math.max(theirs.length, ours.length) }, (_, i) => i).find((i) => theirs[i] !== ours[i]);
  const line = at === undefined ? undefined : (theirs[at] ?? ours[at]);
  const op = line === undefined ? null : /^(\d+) /.exec(line);
  return op === null ? undefined : forms[Number(op[1]) - 1];
}

/** The two builds, as the corpus's. */
const builds: [string, string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];

// Miri judges each program where it's installed; the nightly's job installs
// it, and a run that asks for it, `RUST_JS_REQUIRE_MIRI=1`, fails without.
const miri = miriInstalled();
if (!miri && partSetting("RUST_JS_REQUIRE_MIRI") === "1") throw new Error(`Miri isn't installed: ${installMiri}`);

/** Each refusal's reason, its spans and numbers taken out, and how many
 * programs it refused, of each build. */
const refusals = new Map<string, { count: number; seeds: number[] }>();

/** A refusal's reason, as one of its kind: its first line, rust-js's own
 * words, without where or how many. */
export function reasonOf(error: string): string {
  const line = error.split("\n").find((l) => /rust-js/.test(l)) ?? error.split("\n")[0];
  return line.replace(/^.*?rust-js( does not support|:)?\s*/, "").replace(/\b\d+\b/g, "N").trim();
}

test("each known wrong answer is of a form the generator makes", () => {
  const made = new Set(Array.from({ length: 1000 }, (_, i) => memoryProgram(i + 1).forms).flat());
  for (const k of known) if (k.kind === "wrong") expect(made, k.line).toContain(k.signature);
});

test("a wrong answer is the form of the operation that first prints otherwise", () => {
  const forms = ["arith/write", "views/bytes", "copies/copy"];
  const native = "1 a0 [1, 2]\n2 x2 [3]\n3 a1 [4]\n";
  expect(firstOtherwise(native, "1 a0 [1, 2]\n2 x2 [9]\n3 a1 [4]\n", forms)).toBe("views/bytes");
  expect(firstOtherwise(native, "1 a0 [1, 2]\n", forms)).toBe("views/bytes");
  expect(firstOtherwise(native, native, forms)).toBeUndefined();
  expect(firstOtherwise(native, `${native}3 a1 [5]\n`, forms)).toBe("copies/copy");
});

for (const seed of chosen) {
  test(`seed ${seed}`, () => {
    const { text, swarm, forms } = memoryProgram(seed);
    const dir = fixture(`memory-${seed}`);
    // Where its text says, so the same program is one kept native binary.
    const file = join(contentDirectory(text), "program.rs");
    writeWhole(file, text);
    if (miri) {
      const ub = undefinedBehaviour(file);
      if (ub !== undefined) throw new Error(`seed ${seed}'s program (${swarm.join(", ")}) is undefined behaviour, which the generator must never make:\n${text}\n${ub}`);
    }
    const native = runNative(file, dir, "2024", "wasm32");
    if (typeof native === "string") throw new Error(`seed ${seed} made a program rustc rejects:\n${text}\n${native}`);
    const problems: string[] = [];
    for (const [build, flags] of builds) {
      const key = `${seed} ${build}`;
      const compiled = compileJs(file, dir, "2024", false, {}, flags);
      if ("error" in compiled) {
        if (compiled.kind === "crashed") {
          problems.push(`${key}: rust-js crashed: ${compiled.reason}\n${compiled.error.slice(0, 2000)}`);
          continue;
        }
        const reason = reasonOf(compiled.error);
        refused(build, reason, seed);
        if (knownAs("refused", reason) === undefined) problems.push(`${key}: refused, and its reason isn't in test/memory-net-known.txt:\n${compiled.error.split("\n").slice(0, 6).join("\n")}`);
        continue;
      }
      const [runtime, cmd] = runtimes[0];
      const ran = runJs(cmd, compiled.js, dir, `${runtime}-${build}`);
      // A refusal where it runs, `$unsupported`'s error (#1193), the same
      // as native Rust up to it: counted as a refusal, at run time.
      const error = typeof ran.outcome === "object" && "error" in ran.outcome ? String(ran.outcome.error) : "";
      if (error.includes("rust-js does not support") && native.bytes.stdout.subarray(0, ran.bytes.stdout.length).equals(ran.bytes.stdout)) {
        const reason = `at run time: ${reasonOf(error)}`;
        refused(build, reason, seed);
        if (knownAs("refused", reason) === undefined) problems.push(`${key}: refused where it runs, and its reason isn't in test/memory-net-known.txt:\n${error}`);
        continue;
      }
      if (agree(ran, native)) continue;
      // A wrong answer of a known form passes; another is kept and reduced.
      const form = firstOtherwise(native.stdout, ran.stdout, forms);
      if (form !== undefined && knownAs("wrong", form) !== undefined) continue;
      problems.push(wrong(seed, build, file, text, native, ran, form));
    }
    expect(problems.join("\n\n")).toBe("");
  }, 600_000);
}

/** One more program `build` refused for `reason`. */
function refused(build: string, reason: string, seed: number) {
  const counted = refusals.get(`${build}: ${reason}`) ?? { count: 0, seeds: [] };
  counted.count++;
  counted.seeds.push(seed);
  refusals.set(`${build}: ${reason}`, counted);
}

/** A wrong answer, kept in target/memory-net/ before it's reduced, so a
 * job stopped can't lose it: what each run printed, and the program
 * reduced. */
function wrong(seed: number, build: string, file: string, text: string, native: Run, ran: Run, form: string | undefined): string {
  const out = join(target, "memory-net");
  mkdirSync(out, { recursive: true });
  writeFileSync(join(out, `seed-${seed}.rs`), text);
  const detail = `native: ${show(native.outcome)}\n${native.stdout}\nJS (${build}): ${show(ran.outcome)}\n${ran.stdout}${ran.stderr.slice(0, 2000)}`;
  writeFileSync(join(out, `seed-${seed}.${build}.txt`), detail);
  const first = form === undefined ? "only in how it ended" : `first printed by a \`${form}\``;
  return `seed ${seed}, ${build}: a wrong answer, ${first}\n${detail}${reducedRepro(file, build)}`;
}

afterAll(() => {
  // How many programs each known line met, of this part: what its fix
  // takes off.
  const counted = known.filter((k) => met.has(k.line)).map((k) => `${String(met.get(k.line)).padStart(4)}  ${k.line}`);
  if (counted.length > 0) console.log(`known, by how many programs each met:\n${counted.join("\n")}`);
  if (refusals.size === 0) return;
  const out = join(target, "memory-net");
  mkdirSync(out, { recursive: true });
  const ranked = [...refusals].sort((a, b) => b[1].count - a[1].count).map(([reason, { count, seeds }]) => ({ reason, count, seeds }));
  writeFileSync(join(out, `refused-${part}-of-${parts}.json`), JSON.stringify(ranked, null, 2));
  console.log(`refusals, by how many programs each refused:\n${ranked.map((r) => `${String(r.count).padStart(4)}  ${r.reason}`).join("\n")}`);
});
