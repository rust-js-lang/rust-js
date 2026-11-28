// Negative controls for how a program's run is taken (ADR 0088): a run that
// failed must not pass for one that didn't.

import { expect, test } from "bun:test";
import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { agree, compared, compileJsAsync, hookSays, jsLimit, ranOnce, runJs, runProcess, runtimes, unplaced, type Run } from "./programs";
import { buildCompiler, fixture } from "./support";

const clean: Run = { stdout: "", stderr: "", bytes: { stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) }, outcome: { exit: 0 } };
const script = (code: string) => [process.execPath, "-e", code];
const mark = (name: string) => join(fixture(name), "run");

// A program's run is its process's (#2447): its status is how `main` ended.
test("a run's status is how it ended, and one that fails isn't one that didn't", () => {
  const run = runProcess(script("process.exit(7)"), mark("programs-exit"));
  expect(run.outcome).toEqual({ exit: 7 });
  expect(agree(run, clean)).toBe(false);
  const ok = runProcess(script("0"), mark("programs-ok"));
  expect(agree(ok, clean)).toBe(true);
  expect(ok.stopped).toBeUndefined();
});

// A run stopped past its time didn't end by itself: what it printed is a
// cut run's, which a net doesn't compare (#1252).
test("a run stopped past its time is said to be stopped", () => {
  const run = runProcess(script("setInterval(() => {}, 1000)"), mark("programs-endless"), 300);
  expect(run.outcome).toBe("didn't finish in 0.3s");
  expect(run.stopped).toBe(true);
});

// What a run printed is compared byte for byte: a byte that isn't UTF-8
// isn't the U+FFFD it reads as. Found in review: they compared equal.
test("what a run printed is compared byte for byte", () => {
  const printing = (hex: string) => runProcess(script(`process.stdout.write(Buffer.from("${hex}", "hex"))`), mark("programs-bytes"));
  expect(agree(printing("ff"), printing("efbfbd"))).toBe(false);
  expect(agree(printing("efbfbd"), printing("efbfbd"))).toBe(true);
});

// Where a panic was raised is its hook's line, compared with the rest of
// stderr; of a run `unplaced`, a case `located-otherwise`'s, the other's is
// compared without it too (#2292).
test("a panic raised elsewhere disagrees, but not with a run that says no place", () => {
  const hooked = (at: string): Run => {
    const stderr = `\nthread 'main' panicked at ${at}:\nboom\nnote: run with \`RUST_BACKTRACE=1\` environment variable to display a backtrace\n`;
    return { ...clean, stderr, bytes: { stdout: Buffer.alloc(0), stderr: Buffer.from(stderr) }, outcome: { exit: 101 } };
  };
  const [here, there] = [hooked("test/corpus/a.rs:3:5"), hooked("test/corpus/a.rs:4:9")];
  expect(agree(here, there)).toBe(false);
  expect(agree(here, hooked("test/corpus/a.rs:3:5"))).toBe(true);
  expect(agree(unplaced(here), there)).toBe(true);
  expect(agree(here, unplaced(there))).toBe(true);
  expect(unplaced(here).stderr).toContain("panicked at <somewhere>:\nboom\n");
});

// Of std's hook's text, the panic that ends `main` is compared whole, and
// each caught before it left out (#2447; #2453 compares them); of an abort, the frames std
// forces of a panic that can't unwind are left out, which the JS has none
// of (ADR 0262's exception, the guide's difference 64).
test("a run is compared by the hook of the panic that ends it, and an abort without native's frames", () => {
  const run = (stderr: string, exit: number): Run => ({ ...clean, stderr, bytes: { stdout: Buffer.alloc(0), stderr: Buffer.from(stderr) }, outcome: { exit } });
  const hook = (at: string, message: string) => `\nthread 'main' panicked at ${at}:\n${message}\n`;
  const caught = (at: string, message: string, last: string) => run(`${hook(at, message)}caught\n${hook("a.rs:9:1", last)}`, 101);
  expect(agree(caught("a.rs:3:5", "index 18446744073709551615", "boom"), caught("a.rs:4:2", "index 4294967295", "boom"))).toBe(true);
  expect(agree(caught("a.rs:3:5", "x", "boom"), caught("a.rs:3:5", "x", "bang"))).toBe(false);
  expect(compared(caught("a.rs:3:5", "x", "boom")).stderr).toBe(`caught\n${hook("a.rs:9:1", "boom")}`);
  // Of a run that returned, every panic was caught.
  expect(agree(run(hook("a.rs:3:5", "x"), 0), run(hook("b.rs:1:1", "y"), 0))).toBe(true);
  // An abort's frames, natively, left out; the lines around them compared.
  const frames = "stack backtrace:\n   0:        0x1020e2988 - std::backtrace::trace\n                               at /rustc/x/library/std/src/backtrace.rs:117:9\n  37:        0x1020c4fec - _main\n";
  const end = `${hook("/rustc/x/library/core/src/panicking.rs:233:5", "panic in a destructor during cleanup")}thread caused non-unwinding panic. aborting.\n`;
  const native = run(`${hook("a.rs:12:13", "part 3 panics")}${frames}${end}`, 134);
  const js = run(`${hook("a.rs:12:13", "part 3 panics")}${end}`, 134);
  expect(agree(native, js)).toBe(true);
  expect(agree(native, run(`${hook("a.rs:12:13", "part 3 panics")}thread caused non-unwinding panic. aborting.\n`, 134))).toBe(false);
  // Not of a panic that ends a run unaborted: there it's what was printed.
  expect(agree(run(`${hook("a.rs:1:1", "x")}${frames}`, 101), run(hook("a.rs:1:1", "x"), 101))).toBe(false);
});

// A `run-fail` directive's message is checked against native Rust's hook:
// its line after where the panic was raised, as it is, nothing read as a
// pattern (#2447).
test("a hook says the message it was raised with", () => {
  const stderr = "\nthread 'main' panicked at a.rs:3:5:\nindex out of bounds: the len is 1 but the index is 1\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n";
  expect(hookSays(stderr, "index out of bounds: the len is 1 but the index is 1")).toBe(true);
  expect(hookSays(stderr, "index out of bounds")).toBe(false);
  expect(hookSays(stderr, "index out of bounds: the len is . but the index is 1")).toBe(false);
  expect(hookSays("panicked\n", "panicked")).toBe(false);
});

// Each run is its own process's, in a directory of its own, empty as it
// starts: what one writes there, a file of `fs`'s, isn't another's.
test("a run runs in a directory of its own, emptied as it starts", () => {
  const at = mark("programs-own-directory");
  const write = script('const fs = require("fs"); console.log(fs.readdirSync(".").length); fs.writeFileSync("left.txt", "x")');
  expect(runProcess(write, at).stdout).toBe("0\n");
  expect(runProcess(write, at).stdout).toBe("0\n");
});

// A program native Rust itself takes long to run is given as long, twenty
// times over, as JS, before it's taken to never end; one quick natively,
// the usual 10 s (#1106). A run's time is how long this process waited.
test("a JS run may take twenty times as long as native Rust's, and at least 10 s", () => {
  expect(jsLimit({ ...clean, ms: 2_300 })).toBe(46_000);
  expect(jsLimit({ ...clean, ms: 3 })).toBe(10_000);
  expect(jsLimit(clean)).toBe(10_000);
  const run = runProcess(script("setTimeout(() => {}, 300)"), mark("programs-timed"));
  expect(run.ms).toBeGreaterThanOrEqual(300);
});

// A native program built below the disk's floor isn't kept, and goes once
// it's run: a run of thousands, the metamorphic net's, filled its machine's
// disk with them, each in a fixture its test file held till it ended
// (#1108). One kept stays, for the next run of it.
test("a native build that isn't kept goes once it's run, and a kept one stays", () => {
  const unkept = fixture("programs-unkept");
  writeFileSync(join(unkept, "native"), "");
  const kept = fixture("programs-kept");
  writeFileSync(join(kept, "native"), "");
  expect(ranOnce({ binary: join(unkept, "native"), kept: false }, clean)).toBe(clean);
  expect(ranOnce({ binary: join(kept, "native"), kept: true }, clean)).toBe(clean);
  expect([existsSync(unkept), existsSync(join(kept, "native"))]).toEqual([false, true]);
});

// A case's variants, compiled at once into its folder, each run their own
// modules (#2628): a case's modules go beside its root, and a variant's
// `log.js` imported the root another variant wrote last, whose `main` ran
// too, so 5 of 6 stages of `thread_local_set_before_use` printed its 13
// lines twice and the tidy-stages net failed 10 cases a night.
test("a case's variants compiled at once each run their own modules", async () => {
  buildCompiler();
  const file = join(import.meta.dir, "corpus", "thread_local_set_before_use.rs");
  const dir = fixture("programs-variants-at-once");
  const stages = ["flags/settled flags", "values/shared tails", "jumps/chain values", "loops/unlabeled"];
  const made = await Promise.all(stages.map((stage) => compileJsAsync(file, dir, "2024", "binary", {}, ["--tidy-until", stage])));
  const lines = made.map((compiled, at) => ("js" in compiled ? runJs(runtimes[0][1], compiled.js, dir, `variant-${at}`).stdout.split("\n").length : compiled.error));
  expect(lines).toEqual(stages.map(() => 13));
}, 300_000);

// A program that never ends natively is read compiled, nothing run, by
// the one command that runs a case by hand: agents wrote helpers around
// `compileJs` to read a `Box<Repeat>`'s `count`, a second way (#2620).
test("run-case's --compile-only prints a case's JS and runs nothing, natively or as JS", () => {
  buildCompiler();
  const file = join(fixture("programs-compile-only"), "endless.rs");
  writeFileSync(file, 'fn main() {\n    println!("{}", std::iter::repeat(1u8).count());\n}\n');
  const p = Bun.spawnSync([process.execPath, "scripts/run-case.ts", file, "--compile-only"], { cwd: join(import.meta.dir, ".."), stdout: "pipe", stderr: "pipe", timeout: 120_000 });
  const said = p.stdout.toString();
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(said).toStartWith("── its JS (");
  expect(said).toContain("function main(");
  expect(said).not.toContain("── native Rust");
  const given = Bun.spawnSync([process.execPath, "scripts/run-case.ts", "case.js", "--compile-only"], { cwd: join(import.meta.dir, ".."), stdout: "pipe", stderr: "pipe" });
  expect([given.exitCode, given.stderr.toString()]).toEqual([64, "--compile-only compiles a case.rs; case.js is compiled already: read it\n"]);
}, 300_000);
