// A test file of cases in parts on one machine, as the nightly runs its
// nets (#1306): `RUST_JS_CASES=i/n` each, at most `at once` of them
// running together, the rest waiting for one to end. Each part's lines are
// printed as they come, its part's name before each, and what each part
// holds, it and every process it started, is read each second: its peak is
// said as it ends, with what held it, and the machine's memory each
// minute, so a job the machine stops still shows where it was. A part that
// ends by a signal, or that's running as the job is stopped, is said
// `killed (memory?)`, with what it was running, each command's fixture
// naming its case, and the last case it said ended: `bun test` holds a
// file's lines till it ends where its output is a pipe, so a part killed
// may have said none. Nightly 42884008739's metamorphic net ended in a bare
// exit 143, and its neighbour shuffle in the runner's shutdown, and neither
// said where.
//
//   bun scripts/parts.ts auto test/neighbours.test.ts --timeout=1800000
//   bun scripts/parts.ts 4 test/neighbours.test.ts
//   bun scripts/parts.ts 8 --at-once=4 test/neighbours.test.ts
//   bun scripts/parts.ts 5-8/24 test/iterator-matrix.test.ts
//   RUST_JS_CASES=1/1:<file> bun scripts/parts.ts auto test/metamorphic.test.ts
//
// runs the file in as many parts as the machine's cores and memory hold
// at once (scripts/shard.ts' `atOnce`); in 4; in 8, 4 at once; or the
// 5th to the 8th of 24, a machine's share of a job split across
// machines; with `bun test`'s flags given after the file; or the corpus
// cases `<file>` names, a line each, in parts, each part `i/n:<file>`, at
// most a part a case. Parts run as many at once as the machine holds, of
// those named, unless `--at-once` says fewer.
//
// A list given so is each part's: a list it dropped, each part given
// `i/n` alone, ran a file's default set, the metamorphic net's push
// sample, and passed as if the list had (#2618). And as the spec names
// the parts, a `RUST_JS_CASES` of another part than `1/1` is refused.
//
// One file, of cases each part takes its share of (`RUST_JS_CASES`), or
// it's refused (`unparted`): a file of no cases, or a second file among
// the flags, ran whole in every part, and a run of the suite's files in
// 10 parts wrote 20 GiB of fixtures and filled the disk. The suite in
// parts is `bun run check:fast`'s, which runs each file once.
import { readFileSync } from "node:fs";
import { freemem, totalmem } from "node:os";
import { join, resolve } from "node:path";

import { footprints, processTree, stopGroup, treeBytes } from "../test/child";
import { atOnce } from "./shard";
import { passed } from "./test-summary";

const root = join(import.meta.dir, "..");

/** A part as it runs: when it started and ended, its lines' last case,
 * what it held at its most and what held it then, what its processes run,
 * read last, and how it ended. */
type Part = {
  index: number;
  started: number;
  ended?: number;
  lastCase?: string;
  peak: number;
  heldBy: string[];
  running: string[];
  output: string;
  verdict?: Verdict;
};

type Verdict = "passed" | "failed" | "killed (memory?)";

/** The parts `spec` names, by their index from 1, of how many: `n`, each
 * of `n`, or `a-b/n`, the `a`th to the `b`th of `n`. */
export function partsOf(spec: string): { parts: number[]; count: number } {
  const [, from, to, of] = /^(?:(\d+)-(\d+)\/)?(\d+)$/.exec(spec) ?? [];
  const count = Number(of);
  const [first, last] = from === undefined ? [1, count] : [Number(from), Number(to)];
  if (of === undefined || count < 1 || first < 1 || first > last || last > count) throw new Error(`parts are \`n\` or \`a-b/n\`, not ${spec}`);
  return { parts: Array.from({ length: last - first + 1 }, (_, i) => first + i), count };
}

/** The case list `RUST_JS_CASES` gives parts.ts, `1/1:<file>`, `<file>`
 * made absolute from `cwd`, as each part runs in the checkout: each part
 * keeps it (`partCases`). Nothing where it's unset or `1/1`; another part,
 * `i/n` or `i/n:<file>`, is refused, as the spec names the parts (#2618). */
export function givenList(given: string | undefined, cwd = process.cwd()): string | undefined {
  if (given === undefined || given === "" || given === "1/1") return undefined;
  const split = given.indexOf(":");
  if (split < 0 || given.slice(0, split) !== "1/1") {
    throw new Error(`RUST_JS_CASES=${given}: parts.ts's spec names its parts, and \`RUST_JS_CASES=1/1:<file>\` the cases they run, a line each; give no other part`);
  }
  return resolve(cwd, given.slice(split + 1));
}

/** The parts `spec` names of a run of `list`, if one's given: at most a
 * part a case it names, of a spec of `auto`'s count or `n`; `a-b/n`, a
 * machine's share, as it says. */
export function partsOfList(spec: string, list: string | undefined): { parts: number[]; count: number; list?: string } {
  const named = partsOf(spec);
  if (list === undefined) return named;
  const cases = new Set(readFileSync(list, "utf8").split("\n").filter(Boolean)).size;
  const capped = /^\d+$/.test(spec) && cases < named.count ? partsOf(String(Math.max(1, cases))) : named;
  return { ...capped, list };
}

/** Part `index` of `count`'s `RUST_JS_CASES`: `i/n`, or of a list,
 * `i/n:<list>`. */
export function partCases(index: number, count: number, list?: string): string {
  return `${index}/${count}${list === undefined ? "" : `:${list}`}`;
}

/** Why `file`, of source `source`, given `flags` after it, can't run in
 * parts: a test file among the flags, or a file that doesn't take its
 * part's share of its cases, `RUST_JS_CASES` read or `caseShard`'s, so
 * each part would run all of it; or nothing. */
export function unparted(file: string, source: string, flags: string[]): string | undefined {
  const runSuite = "run the suite with `bun run check:fast`, which runs each file once and the corpus in parts";
  const others = flags.filter((flag) => !flag.startsWith("-") && /\.test\.[jt]sx?$/.test(flag));
  if (others.length > 0) return `${others.join(", ")}: one test file runs in parts, and every other given would run whole in each part: ${runSuite}`;
  if (!/process\.env\.RUST_JS_CASES\b|\bcaseShard\(/.test(source)) {
    return `${file} doesn't take its part's share of its cases (\`RUST_JS_CASES\`, \`caseShard\`), so each part would run it whole: ${runSuite}`;
  }
  return undefined;
}

/** The test a `bun test` line says ended, or nothing. */
export function caseOf(line: string): string | undefined {
  return /^\((?:pass|fail|skip|todo)\) (.*?)(?: \[[\d.]+m?s\])?$/.exec(line)?.[1];
}

/** How a part ended: killed, where a signal ended it, or its shell said
 * one did, `128 +` its number; passed, of code 0, its summary's tests
 * passing and none failing; or failed. */
export function verdict(code: number | null, signal: string | null, output: string): Verdict {
  if (signal !== null || (code !== null && code > 128)) return "killed (memory?)";
  return passed(code, true, output) ? "passed" : "failed";
}

const gb = (bytes: number) => `${(bytes / 2 ** 30).toFixed(1)} GB`;
const minutes = (from: number, to: number) => `${((to - from) / 60_000).toFixed(0)} min`;

/** What a part's processes run, the largest first, with what each holds,
 * the test runner's own left out, the checkout's path cut from each, and
 * each cut to 240 characters. */
function held(tree: { pid: number; bytes: number; command: string }[], runner: number): string[] {
  return tree
    .filter((p) => p.pid !== runner && !p.command.startsWith("ps "))
    .sort((a, b) => b.bytes - a.bytes)
    .map((p) => `${gb(p.bytes)} ${p.command.replaceAll(`${root}/`, "").slice(0, 240)}`);
}

/** A part's line: how it ended, how long it took, what it held at its
 * most and what held it; of one killed, where it was. */
export function said(part: Part, count: number, now: number): string {
  const how = part.verdict ?? "killed (memory?), running as the job was stopped";
  const peak = `at most ${gb(part.peak)}${part.heldBy.length ? `, ${part.heldBy.slice(0, 3).join("; ")}` : ""}`;
  const killed = part.verdict !== "passed" && part.verdict !== "failed";
  const running = part.running.length ? `; then running ${part.running.slice(0, 3).join("; ")}` : "";
  const where = killed ? `${running}; the last case it said ended ${part.lastCase ?? "none"}` : "";
  return `── part ${part.index}/${count} ${how} in ${minutes(part.started, part.ended ?? now)}, ${peak}${where}`;
}

/** Runs `file`'s `parts` of `count`, of the cases `list` names if it's
 * given, `atOnce` together, with `flags`, in `cwd`: whether every part
 * passed. */
export async function runParts(file: string, { parts: indices, count, list }: { parts: number[]; count: number; list?: string }, atOnce: number, flags: string[], cwd = root): Promise<boolean> {
  const parts: Part[] = [];
  const runners = new Map<number, number>();
  const start = Date.now();
  let lowest = freemem();
  const end = () => {
    for (const part of parts) console.log(said(part, count, Date.now()));
    console.log(`── ${indices.length} parts of ${count}, ${Math.min(atOnce, indices.length)} at once, in ${minutes(start, Date.now())}; of the machine's ${gb(totalmem())}, ${gb(lowest)} free at the least`);
  };
  // The job stopped: each part still running said, then stopped.
  const signals = ["SIGTERM", "SIGINT", "SIGHUP"] as const;
  const stopped = (signal: NodeJS.Signals) => {
    console.log(`── stopped by ${signal}`);
    end();
    for (const pid of runners.values()) stopGroup(pid);
    process.exit(143);
  };
  for (const signal of signals) process.on(signal, stopped);
  const sampler = setInterval(() => {
    lowest = Math.min(lowest, freemem());
    // Each running part's tree, then what each holds, of one `footprint`
    // of them all (`treeBytes`, #1499).
    const trees = parts.flatMap((part) => {
      const runner = runners.get(part.index);
      return runner === undefined ? [] : [{ part, runner, tree: processTree(runner) }];
    });
    const footprinted = footprints(trees.flatMap(({ tree }) => tree.map(({ pid }) => pid)));
    for (const { part, runner, tree } of trees) {
      const bytes = treeBytes(tree, footprinted);
      part.running = held(tree, runner);
      if (bytes > part.peak) [part.peak, part.heldBy] = [bytes, part.running];
    }
  }, 1000);
  const minute = setInterval(() => {
    const running = parts.filter((part) => runners.has(part.index)).map((part) => `part ${part.index} at most ${gb(part.peak)}`);
    console.log(`── memory: ${gb(freemem())} free of ${gb(totalmem())}; ${running.join(", ")}`);
  }, 60_000);
  const waiting = [...indices];
  let passed = true;
  const worker = async () => {
    for (let index = waiting.shift(); index !== undefined; index = waiting.shift()) {
      const part: Part = { index, started: Date.now(), peak: 0, heldBy: [], running: [], output: "" };
      parts.push(part);
      const p = Bun.spawn(["bun", "test", ...flags, file], {
        cwd,
        env: { ...process.env, RUST_JS_CASES: partCases(part.index, count, list) },
        stdout: "pipe",
        stderr: "pipe",
        detached: true,
      });
      runners.set(part.index, p.pid);
      const each = (line: string) => {
        part.output += `${line}\n`;
        part.lastCase = caseOf(line) ?? part.lastCase;
        console.log(`[part ${part.index}/${count}] ${line}`);
      };
      await Promise.all([lines(p.stdout, each), lines(p.stderr, each)]);
      const code = await p.exited;
      runners.delete(part.index);
      part.ended = Date.now();
      part.verdict = verdict(code, p.signalCode == null ? null : String(p.signalCode), part.output);
      console.log(said(part, count, part.ended));
      passed &&= part.verdict === "passed";
    }
  };
  await Promise.all(Array.from({ length: Math.min(atOnce, indices.length) }, worker));
  clearInterval(sampler);
  clearInterval(minute);
  for (const signal of signals) process.off(signal, stopped);
  end();
  return passed;
}

/** Each line of `stream`, given `each` as it ends. */
export async function lines(stream: ReadableStream<Uint8Array>, each: (line: string) => void): Promise<void> {
  const decoder = new TextDecoder();
  let rest = "";
  for await (const chunk of stream) {
    rest += decoder.decode(chunk, { stream: true });
    const got = rest.split("\n");
    rest = got.pop() ?? "";
    for (const line of got) each(line);
  }
  if (rest !== "") each(rest);
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const said = args.find((arg) => arg.startsWith("--at-once="));
  const [spec, file, ...flags] = args.filter((arg) => arg !== said);
  if (spec === undefined || file === undefined) throw new Error("usage: bun scripts/parts.ts <auto | n | a-b/n> [--at-once=<k>] <test file> [bun test's flags]");
  const refused = unparted(file, readFileSync(resolve(root, file), "utf8"), flags);
  if (refused !== undefined) throw new Error(refused);
  const held = atOnce("test");
  const parts = partsOfList(spec === "auto" ? String(held) : spec, givenList(process.env.RUST_JS_CASES));
  const together = said === undefined ? Math.min(parts.parts.length, held) : Number(said.slice("--at-once=".length));
  if (!Number.isInteger(together) || together < 1) throw new Error(`${said}: how many parts run at once, from 1`);
  process.exit((await runParts(file, parts, together, flags)) ? 0 : 1);
}
