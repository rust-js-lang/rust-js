// The one way to run generated JS by hand: a case's, as the tests run it,
// each run stopped with everything it started past its time (test/child.ts's
// `runSync`, which kills the process group). A bare `node -e
// 'import("./case.js")…'` that never ends outlives the shell that ran it: one
// held a core for 28 hours, reparented to launchd, unseen (2026-11-09).
//
//   bun scripts/run-case.ts <case.rs> [--std-iterators] [--tidy-until <pass>] [--prepare-without <rewrite>] [--edition <year>]
//       compiled as the corpus's cases are, and run natively (on wasm32 where
//       it says `//@ native: wasm32`) and as JS, each printed, and whether
//       they agree
//   bun scripts/run-case.ts <case.js | dir>   [--timeout <s>]
//       a case's compiled JS, `case.js` in a directory, run as the tests run
//       it: what `entry` prints, and how it ended
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";

import { directives } from "../test/corpus-cases";
import { agree, compileJs, jsLimit, runJs, runNative, runtimes, show, type Run } from "../test/programs";
import { target } from "../test/support";

const args = process.argv.slice(2);
const flag = (name: string) => {
  const at = args.indexOf(name);
  return at >= 0 ? args[at + 1] : undefined;
};
// Its one argument that's neither a flag nor a flag's value.
const valued = ["--tidy-until", "--prepare-without", "--edition", "--timeout"];
const given = args.find((arg, i) => !arg.startsWith("--") && !valued.includes(args[i - 1]));
if (given === undefined) {
  console.error("bun scripts/run-case.ts <case.rs | case.js | dir> [--std-iterators] [--tidy-until <pass>] [--prepare-without <rewrite>] [--edition <year>] [--timeout <s>]");
  process.exit(64);
}

/** What a run printed, and how it ended. */
function report(name: string, run: Run) {
  console.log(`── ${name}: ${show(run.outcome)}${run.ms === undefined ? "" : `, ${Math.round(run.ms)} ms`}`);
  if (run.stdout) process.stdout.write(run.stdout.endsWith("\n") ? run.stdout : `${run.stdout}\n`);
  if (run.stderr) process.stdout.write(`(stderr)\n${run.stderr.endsWith("\n") ? run.stderr : `${run.stderr}\n`}`);
}

const path = resolve(given);
const [runtime, cmd] = runtimes[0];
if (path.endsWith(".rs")) {
  const edition = flag("--edition") ?? "2024";
  const flags = [
    ...(args.includes("--std-iterators") ? ["--std-iterators"] : []),
    ...(["--tidy-until", "--prepare-without"] as const).flatMap((name) => (flag(name) === undefined ? [] : [name, flag(name)!])),
  ];
  const dir = join(target, "run-case", basename(path, ".rs"));
  mkdirSync(join(dir, "native"), { recursive: true });
  mkdirSync(join(dir, "js"), { recursive: true });
  // Run natively as the corpus runs it: `//@ native: wasm32`'s on wasm32.
  const cased = directives(readFileSync(path, "utf8"));
  const native = runNative(path, join(dir, "native"), edition, typeof cased === "string" ? "host" : cased.native);
  if (typeof native === "string") {
    console.error(native);
    process.exit(1);
  }
  report("native Rust", native);
  const compiled = compileJs(path, join(dir, "js"), edition, false, {}, flags);
  if (!("js" in compiled)) {
    console.error(`rust-js ${compiled.kind}:\n${compiled.error}`);
    process.exit(1);
  }
  const js = runJs(cmd, compiled.js, join(dir, "js"), runtime, jsLimit(native));
  report(`its JS (${compiled.js})`, js);
  const same = agree(native, js);
  console.log(same ? "── the same" : "── they differ");
  process.exit(same ? 0 : 1);
} else {
  const js = existsSync(path) && statSync(path).isDirectory() ? join(path, "case.js") : path;
  if (!existsSync(js)) {
    const others = js.endsWith("/case.js") ? readdirSync(dirname(js)).filter((name) => name.endsWith(".js")) : [];
    console.error(others.length ? `no ${js}; name one of its JS: ${others.join(", ")}` : `no ${js}`);
    process.exit(64);
  }
  const timeout = Number(flag("--timeout") ?? 10) * 1000;
  mkdirSync(join(target, "run-case"), { recursive: true });
  const ran = runJs(cmd, js, join(target, "run-case"), `${basename(js, ".js")}-by-hand`, timeout);
  report(js, ran);
  process.exit(typeof ran.outcome === "string" ? 1 : 0);
}
