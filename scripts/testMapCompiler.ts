#!/usr/bin/env -S bun --no-env-file
// The compiler the tests are given while the test map records what they
// reach (scripts/testMap.ts, #877): the build of it that counts each of its
// functions' runs (`-C instrument-coverage`), run as it was asked, then the
// functions it ran kept for the test it ran for, as a bit each of the map's
// table. Its exit, output and input are the compiler's own; and its
// environment, as Bun loads no `.env` file for it: `.env.test`'s
// `RUSTC_BOOTSTRAP`, which a test of the refusal unsets, set again, let a
// stable release's crate use a feature.
//
// What it ran for: the test file `RUST_JS_TEST_MAP_UNIT` names, or, of a
// file of the corpus's cases, the case its compile includes, the line
// `compileJs`'s wrapper is (test/programs.ts), so each case is a test of its
// own (`caseUnit`).
import { appendFileSync, existsSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";

import { caseUnit } from "./testMap";

const env = (name: string) => {
  const value = process.env[name];
  if (value === undefined) throw new Error(`the test map's compiler is run by scripts/testMap.ts, which sets ${name}`);
  return value;
};
const real = env("RUST_JS_TEST_MAP_COMPILER");
const dir = env("RUST_JS_TEST_MAP_DIR");
const profdata = env("RUST_JS_TEST_MAP_PROFDATA");
const root = env("RUST_JS_TEST_MAP_ROOT");
const args = process.argv.slice(2);

let unit = env("RUST_JS_TEST_MAP_UNIT");
if (args[0]?.endsWith(".rs") && existsSync(args[0])) {
  const included = /^include!\("((?:[^"\\]|\\.)*)"\);/.exec(readFileSync(args[0], "utf8"))?.[1];
  const file = included === undefined ? undefined : relative(root, JSON.parse(`"${included}"`));
  const name = file !== undefined && /^test\/corpus\/[^/]+\.rs$/.test(file) ? file.slice("test/corpus/".length) : undefined;
  if (name !== undefined) unit = caseUnit(unit, name) ?? unit;
}

const id = `${process.pid}-${Date.now()}`;
// A file of counts each process: the compiler runs itself again for each
// crate of crates.io it tries, and those, given one file, wrote over it.
const raws = () => readdirSync(join(dir, "raw")).filter((name) => name.startsWith(`${id}-`)).map((name) => join(dir, "raw", name));
const slug = unit.replaceAll("/", "__");
// Its start marked, and unmarked once its bits are kept: one this process
// never finished, stopped by a test's timeout say, left marked, and what it
// ran is unknown, so the test runs on every change (`always`).
const started = join(dir, "units", `${slug}.${id}.start`);
writeFileSync(started, "");
// And the jobserver's descriptors Cargo gives it, at their numbers: without
// them the compiler warned it couldn't reach Cargo's jobserver, which a
// guide page's output then said.
const stdio: (string | number | null)[] = ["inherit", "inherit", "inherit"];
for (const flags of [process.env.CARGO_MAKEFLAGS, process.env.MAKEFLAGS]) {
  for (const [, read, write] of (flags ?? "").matchAll(/--jobserver-(?:auth|fds)=(\d+),(\d+)/g)) {
    for (const fd of [Number(read), Number(write)]) {
      while (stdio.length <= fd) stdio.push(null);
      stdio[fd] = fd;
    }
  }
}
const ran = Bun.spawnSync([real, ...args], { stdio: stdio as never, env: { ...process.env, LLVM_PROFILE_FILE: join(dir, "raw", `${id}-%p.profraw`) } });
const code = ran.exitCode ?? 128 + (ran.signalCode ? 15 : 0);
// Killed, or ended without writing its counts, an abort's: unknown too.
const written = raws();
if (written.length === 0) {
  appendFileSync(join(dir, "unknown"), `${unit}\n`);
  process.exit(code);
}
const index: Map<string, number> = new Map((JSON.parse(readFileSync(join(dir, "names.json"), "utf8")) as string[]).map((name, i) => [name, i]));
// Of each function's counts, those not all zero: the functions it ran.
const text = Bun.spawnSync([profdata, "merge", "-sparse", "--text", ...written, "-o", "-"], { stdout: "pipe", stderr: "pipe" });
for (const raw of written) rmSync(raw, { force: true });
if (text.exitCode !== 0) {
  appendFileSync(join(dir, "unknown"), `${unit}\n`);
  process.exit(code);
}
const lines = text.stdout.toString().split("\n");
const bits = new Uint8Array(Math.ceil(index.size / 8));
for (let i = 0; i + 1 < lines.length; i++) {
  if (lines[i + 1] !== "# Func Hash:") continue;
  const at = index.get(lines[i]);
  // A function the table doesn't have: the map can't say what it is.
  if (at === undefined) appendFileSync(join(dir, "unknown"), `${unit}\t${lines[i]}\n`);
  else bits[at >> 3] |= 1 << (at & 7);
}
writeFileSync(join(dir, "units", `${slug}.${id}.bits`), bits);
rmSync(started);
process.exit(code);
