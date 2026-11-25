// The harness's own failures, each made on purpose (ADRs 0088, 0089, 0092):
// a compiler that crashes after the rejection a case expects, one that never
// ends, JS that's wrong under one runtime only, a reduction cut short, a
// merge of shards that aren't one run, and a test native Rust never ends.
// Each must fail the run, or say it's incomplete, with what it saw.

import { beforeAll, expect, test } from "bun:test";
import { chmodSync, existsSync, linkSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { ratchet, runTest, type Shard } from "../scripts/rustc-suite";
import { buildCompiler, bunTest, compiler, compilerBuild, fixture, root, target, unlessUnchanged } from "./support";
import { underLoad } from "../scripts/timing";
import { compilerRustFiles } from "../scripts/compiler-crates";
import { editedOf } from "./edited-compiler";

beforeAll(buildCompiler);

/** A compiler at `name` that runs `script`, a shell script. */
function fake(name: string, script: string): string {
  const path = join(fixture(`harness-${name}`), "rust-js");
  writeFileSync(path, `#!/bin/sh\n${script}\n`);
  chmodSync(path, 0o755);
  return path;
}

/** The real compiler, each build's output that compiled edited by `edit`,
 * a shell script given it as `$1`, a one-shot compile's or a worker's
 * (test/edited-compiler.ts, #2431); and the file of the outputs it
 * edited, which a test asks is not empty, so its edit can't be missed. */
function edited(name: string, edit: string): { compiler: string; edits: string } {
  const dir = fixture(`harness-${name}`);
  const script = join(dir, "edit.sh");
  writeFileSync(script, `${edit}\n`);
  rmSync(editedOf(script), { force: true });
  const path = join(dir, "rust-js");
  writeFileSync(path, `#!/bin/sh\nexec "${process.execPath}" "${join(root, "test/edited-compiler.ts")}" "${compiler}" "${script}" "$@"\n`);
  chmodSync(path, 0o755);
  return { compiler: path, edits: editedOf(script) };
}

/** Whether `edits`' compiler edited any output. */
const madeEdits = (edits: string) => existsSync(edits) && readFileSync(edits, "utf8").trim() !== "";

/** `bun test` of `files`, as another process, with `env`. */
function tested(args: string[], env: Record<string, string>) {
  const p = bunTest(args, { env });
  return { code: p.code, output: p.stdout + p.stderr };
}

test("a compiler that crashes after the rejection a case expects fails it", () => {
  const crashing = fake(
    "crash",
    `echo "error: rust-js does not support raw addresses of statics yet" >&2\necho "thread 'rustc' panicked at crates/rust-js-lower/src/lib.rs:1:1:" >&2\nexit 101`,
  );
  const run = tested(["test/corpus.test.ts", "-t", "static_raw_address"], { RUST_JS_COMPILER: crashing });
  expect(run.code).not.toBe(0);
  expect(run.output).toContain("rust-js crashed: thread 'rustc' panicked at crates/rust-js-lower/src/lib.rs:1:1:");
  expect(run.output).toContain("`compile-fail` expects a rejection");
}, underLoad(120_000));

test("a compiler that never ends is stopped, and fails the case", () => {
  const hanging = fake("hang", "sleep 60");
  const run = tested(["test/corpus.test.ts", "-t", "index_out_of_bounds"], { RUST_JS_COMPILER: hanging, RUST_JS_COMPILE_TIMEOUT: "1000" });
  expect(run.code).not.toBe(0);
  expect(run.output).toContain("rust-js crashed: didn't finish in 1s");
}, underLoad(120_000));

// The real compiler's JS, with a line only the JS as compiled runs, not
// the bundle Vite ships of it: its first, which runs before `main`. One
// appended ran after it, which a binary's `main` that panics never does,
// as `$main` ends the process at once (#2537): the run-fail case's JS was
// right, and its test failed.
const onlyUnbundled = () =>
  edited("unbundled-only", `{ echo 'if (import.meta.url.endsWith("/case.js")) console.log("only unbundled");'; cat "$1"; } > "$1.edited-js" && mv "$1.edited-js" "$1"`);

test("JS that's wrong in one way it's run only fails the case", () => {
  const { compiler: wrong, edits } = onlyUnbundled();
  const run = tested(["test/corpus.test.ts", "-t", "index_out_of_bounds"], { RUST_JS_COMPILER: wrong });
  expect(madeEdits(edits)).toBe(true);
  expect(run.code).not.toBe(0);
  expect(run.output).toMatch(/node stdout:\n\+ only unbundled/);
  expect(run.output).not.toContain("node, minified stdout:");
}, underLoad(120_000));

test("a failing generated program is kept before it's reduced, and a reduction cut short says so", () => {
  const out = join(target, "fuzz");
  const run = tested(["test/fuzz.test.ts"], { RUST_JS_COMPILER: onlyUnbundled().compiler, FUZZ_START: "3", FUZZ_SEEDS: "1", FUZZ_REDUCE_BUDGET: "1" });
  expect(run.code).not.toBe(0);
  expect(run.output).toContain("differs, reduced for 0.001s, not to the end,");
  const evidence = JSON.parse(readFileSync(join(out, "seed-3.json"), "utf8"));
  expect(evidence).toMatchObject({ seed: 3, kind: "differs", compiler: expect.stringContaining("rust-js") });
  expect(evidence.signature).toBe(JSON.stringify([["plain", "node", false, true, true]]));
  expect(evidence.detail).toContain("only unbundled");
  expect(existsSync(join(out, "seed-3.original.rs"))).toBe(true);
  expect(existsSync(join(out, "seed-3.rs"))).toBe(true);
}, 300_000);

test("shards that aren't one whole run aren't checked", () => {
  const dir = fixture("harness-shards");
  const source = Bun.spawnSync(["git", "rev-parse", "HEAD"], { cwd: root }).stdout.toString().trim();
  // Every test the checked-in inventory has, at its rustc commit, as the
  // merge requires.
  const lines = readFileSync(join(root, "test", "rustc-inventory.txt"), "utf8").split("\n");
  const toolchain = lines.find((line) => line.startsWith("# commit: "))!.slice("# commit: ".length);
  const inventory = lines.filter((line) => line && !line.startsWith("#"));
  const shard = (i: number): Shard => {
    const expected = inventory.filter((_, k) => k % 2 === i - 1);
    return { shard: i, of: 2, compiler: "c", toolchain, source, inventory, expected, results: expected.map((test) => ({ test, status: "pass" })) };
  };
  const merge = (...shards: Shard[]) => {
    const files = shards.map((s, k) => {
      const file = join(dir, `results-${k}.json`);
      writeFileSync(file, JSON.stringify(s));
      return file;
    });
    const p = Bun.spawnSync([process.execPath, "scripts/rustc-suite.ts", "--merge", ...files], { cwd: root, stdout: "pipe", stderr: "pipe" });
    return { code: p.exitCode, output: p.stdout.toString() };
  };
  const missing = merge(shard(1));
  expect(missing.code).toBe(1);
  expect(missing.output).toContain("INCOMPLETE\tshards missing, of 2: 2");
  const twice = merge(shard(1), shard(2), shard(2));
  expect(twice.code).toBe(1);
  expect(twice.output).toContain("INCOMPLETE\tshards there more than once, of 2: 2");
  // Whole, it's checked: every listed failure passes, which isn't as listed.
  const whole = merge(shard(1), shard(2));
  expect(whole.output).not.toContain("INCOMPLETE");
  expect(whole.output).toContain("PASSES\t");
  expect(whole.code).toBe(1);
}, underLoad(120_000));

test("a test native Rust never ends has no answer, and isn't as listed", async () => {
  const ui = fixture("harness-ui");
  const file = join(ui, "forever.rs");
  writeFileSync(file, "//@ run-pass\nfn main() {\n    loop {}\n}\n");
  mkdirSync(join(target, "rustc-suite"), { recursive: true });
  const result = await runTest(ui, file);
  expect(result).toEqual({ test: "forever.rs", status: "native", reason: "doesn't pass natively with overflow checks off: didn't finish in 10s" });
  if (result.status !== "native") throw new Error(`Expected native failure, got ${result.status}`);
  expect(ratchet([result], new Map(), new Map()).unanswered).toEqual([result]);
}, underLoad(120_000));

// A batch of generated programs says which seeds: one that isn't a whole
// number fails before any runs, not as a run of none. Found in review.
test("fuzz settings that aren't whole numbers fail the run", () => {
  // A seed is a 32-bit number to the generator, so a batch past 2^32 would
  // repeat seeds, or, past 2^53, not count up at all. Found in review.
  for (const [name, value] of [
    ["FUZZ_START", "invalid"],
    ["FUZZ_SEEDS", "0"],
    ["FUZZ_SEEDS", "1.5"],
    ["FUZZ_REDUCE_BUDGET", "-1"],
    ["FUZZ_START", "9007199254740992"],
    ["FUZZ_START", "4294967296"],
  ]) {
    const run = tested(["test/fuzz.test.ts"], { FUZZ_START: "1", FUZZ_SEEDS: "1", [name]: value });
    expect([name, run.code]).toEqual([name, 1]);
    expect(run.output).toContain(`${name}=${value}: `);
  }
  const past = tested(["test/fuzz.test.ts"], { FUZZ_START: "4294967290", FUZZ_SEEDS: "10" });
  expect(past.code).toBe(1);
  expect(past.output).toContain("FUZZ_START=4294967290 FUZZ_SEEDS=10: the seeds end past 4294967296");
}, underLoad(120_000));

// A test listed as printing what changes is run natively until it does, up
// to 30 times: a coin flip, as a two-entry HashMap's order is, is the same
// three runs in four, and would look like it answers. Found by a run in
// which rustc's issue-3559.rs did.
test("a test that prints what changes, listed so, is run until it does", async () => {
  const ui = fixture("harness-changing");
  const file = join(ui, "coin.rs");
  writeFileSync(
    file,
    "//@ run-pass\nuse std::collections::hash_map::RandomState;\nuse std::hash::{BuildHasher, Hasher};\nfn main() {\n    let flip = RandomState::new().build_hasher().finish() % 2;\n    println!(\"{flip}\");\n}\n",
  );
  mkdirSync(join(target, "rustc-suite"), { recursive: true });
  const listed = new Set(["coin.rs"]);
  for (let i = 0; i < 3; i++) {
    expect(await runTest(ui, file, listed)).toEqual({ test: "coin.rs", status: "native", reason: "prints what changes from run to run" });
  }
}, underLoad(120_000));

/** The real compiler, with JS that prints the bytes FF 0A in its place,
 * which read as UTF-8 are "\u{FFFD}\n", as it prints to stderr: as it
 * runs, as a binary's JS does, not in an `entry` none calls, a library's
 * shape, which since a case's JS is a binary's printed nothing (#2537).
 * And an `entry` that does nothing, which the rustc suite's run calls
 * once the module's loaded (test/corpus-run.ts): without it the run threw,
 * exit 101, not the bytes the test is of (the nightly of 2026-11-25). */
function notUtf8(): { compiler: string; edits: string } {
  const js = join(fixture("harness-not-utf8-js"), "case.js");
  writeFileSync(js, `process.stdout.write(new Uint8Array([0xff, 10]));\nprocess.stderr.write("\\uFFFD\\n");\nexport function entry() {}\n`);
  return edited("not-utf8", `cp "${js}" "$1"`);
}

// What a case printed is compared as bytes, not as the text they read as.
// Found in review: `agree` compared bytes, but the corpus's own checks,
// rustc's tests', and whether native Rust prints what changes, text.
test("JS that prints other bytes than native Rust, reading the same, fails the case", () => {
  const { compiler: other, edits } = notUtf8();
  const run = tested(["test/corpus.test.ts", "-t", "replacement_character"], { RUST_JS_COMPILER: other, RUST_JS_SNAPSHOTS: "ignore" });
  expect(madeEdits(edits)).toBe(true);
  expect(run.code).not.toBe(0);
  expect(run.output).toContain("node stdout:");
  expect(run.output).not.toContain("stderr:");
}, underLoad(120_000));

// A mutation of how the JS reads that its tests miss is judged by the
// snapshots of the cases that reach it alone (scripts/mutations.ts, #864):
// a case whose JS isn't its snapshot fails, and the compiler as it is
// passes, with no run of it natively or as JS.
test("a case's JS alone fails where it isn't its snapshot, and isn't run", () => {
  const other = tested(["test/corpus.test.ts", "-t", "^replacement_character.rs$"], { RUST_JS_COMPILER: notUtf8().compiler, RUST_JS_SNAPSHOTS: "only" });
  expect(other.code).not.toBe(0);
  expect(other.output).toContain("its JS isn't its snapshot, replacement_character.js");
  expect(other.output).not.toContain("node stdout:");
  const same = tested(["test/corpus.test.ts", "-t", "^replacement_character.rs$"], { RUST_JS_COMPILER: compiler, RUST_JS_SNAPSHOTS: "only" });
  expect(same.output).toContain(" 1 pass");
  expect(same.code).toBe(0);
}, underLoad(120_000));

test("a rustc test whose JS prints other bytes, reading the same, fails, and one natively printing either changes", async () => {
  const ui = fixture("harness-bytes");
  const file = join(ui, "replacement.rs");
  writeFileSync(file, `//@ run-pass\nfn main() {\n    println!("\\u{FFFD}");\n    eprintln!("\\u{FFFD}");\n}\n`);
  mkdirSync(join(target, "rustc-suite"), { recursive: true });
  expect(await runTest(ui, file, new Set(), notUtf8().compiler)).toMatchObject({ test: "replacement.rs", status: "fail", reason: "node: different stdout" });
  const either = join(ui, "either.rs");
  writeFileSync(
    either,
    "//@ run-pass\nuse std::collections::hash_map::RandomState;\nuse std::hash::{BuildHasher, Hasher};\nuse std::io::Write;\nfn main() {\n    if RandomState::new().build_hasher().finish() % 2 == 0 {\n        std::io::stdout().write_all(&[0xff, b'\\n']).unwrap();\n    } else {\n        println!(\"\\u{FFFD}\");\n    }\n}\n",
  );
  expect(await runTest(ui, either, new Set(["either.rs"]))).toEqual({ test: "either.rs", status: "native", reason: "prints what changes from run to run" });
}, underLoad(120_000));

// What's built from unchanged inputs isn't built again: the react, webapi and
// js crates' metadata, which every test run asked for anew, three seconds of
// each run's (DEVELOPMENT.md).
test("what's built from unchanged inputs isn't built again", () => {
  const dir = fixture("unless-unchanged");
  const input = join(dir, "input"), output = join(dir, "output"), stamp = join(dir, "stamp");
  writeFileSync(input, "a");
  let runs = 0;
  const work = (into: string) => {
    runs += 1;
    writeFileSync(join(into, "output"), readFileSync(input));
    // What another run reads as this one builds: the output before, whole,
    // never gone or half written (check 44024336584).
    if (runs > 1) expect(readFileSync(output, "utf8")).toBe("a");
  };
  const step = () => unlessUnchanged(stamp, [input], [output], work);
  expect([step(), step()]).toEqual([true, false]);
  writeFileSync(input, "b");
  expect(step()).toBe(true);
  rmSync(output);
  writeFileSync(output, "a");
  writeFileSync(input, "c");
  expect([step(), runs, readFileSync(output, "utf8")]).toEqual([true, 3, "c"]);
  // Nothing of a build's own directory is left.
  expect(readdirSync(dir).filter((name) => name.startsWith(".building-"))).toEqual([]);
});

// What's built by the compiler is keyed by its build, not by where it is:
// `check:fast` gives the tests a link of it of its own each run, so a stamp
// that named its path built the node crate's metadata again in every
// check, 2.4 s on the Mac mini, 4.2 s on the M1 (#2500). Another build, a
// script standing in for it whose text changed here, builds it again.
test("what the compiler builds is kept by its build, not its path", () => {
  buildCompiler();
  const dir = fixture("unless-unchanged-compiler");
  const input = join(dir, "input"), output = join(dir, "output"), stamp = join(dir, "stamp");
  writeFileSync(input, "a");
  const [first, second] = ["first", "second"].map((name) => {
    const link = join(dir, name);
    linkSync(compiler, link);
    return link;
  });
  let runs = 0;
  const step = (by: string) => unlessUnchanged(stamp, [input], [output], (into) => { runs += 1; writeFileSync(join(into, "output"), "made"); }, by);
  expect([step(first!), step(second!), runs]).toEqual([true, false, 1]);
  const script = join(dir, "standing-in");
  writeFileSync(script, "#!/bin/sh\necho one\n");
  chmodSync(script, 0o755);
  expect([step(script), step(script)]).toEqual([true, false]);
  writeFileSync(script, "#!/bin/sh\necho two\n");
  expect([step(script), runs]).toEqual([true, 3]);
}, underLoad(120_000));

// rust-js is built as a shell builds it, with `.cargo/config.toml`'s
// `RUSTC_BOOTSTRAP`, not the one `.env.test` gives the tests' programs:
// cargo, building it after the other, rebuilt every dependency, twenty
// seconds of the next build's (DEVELOPMENT.md). So the tests' build
// environment is a shell's in each variable cargo tracks: its own, those
// build.rs reruns on and those the crate reads as it's compiled. Asked of
// the environments, not of a build: CI's jobs restore the compiler's binary
// alone, so a build there was a whole one, past the test's time (#2066).
test("the tests build rust-js as a shell does", () => {
  // A shell's environment: the tests', less what bun gives them.
  const given = [...readFileSync(join(root, ".env.test"), "utf8").matchAll(/^([A-Z_][A-Z0-9_]*)=/gm)].map((m) => m[1]!);
  const shell = Object.fromEntries(Object.entries(process.env).filter(([name]) => name !== "NODE_ENV" && !given.includes(name)));
  const tests = Object.fromEntries(Object.entries({ ...process.env, ...compilerBuild.env }).filter(([, value]) => value !== undefined));
  const read = (file: string, of: RegExp) => [...readFileSync(join(root, file), "utf8").matchAll(of)].map((m) => m[1]!);
  const rerun = read("build.rs", /rerun-if-env-changed=(\w+)/g);
  const compiled = compilerRustFiles().flatMap((file) => read(file, /\b(?:option_)?env!\("(\w+)"\)/g));
  const tracked = (name: string) => /^(RUSTC\w*|RUSTFLAGS|RUSTDOCFLAGS|CARGO_\w+)$/.test(name) || rerun.includes(name) || compiled.includes(name);
  expect(given).toContain("RUSTC_BOOTSTRAP");
  expect(rerun).toContain("RUSTC");
  const names = [...new Set([...Object.keys(shell), ...Object.keys(tests)])].filter(tracked);
  expect(names.filter((name) => shell[name] !== tests[name])).toEqual([]);
});

// And loading the rustc suite leaves the environment as it was: each
// test's compilers are given it.
test("the rustc suite gives RUSTC_BOOTSTRAP to its compilers alone", () => {
  const { RUSTC_BOOTSTRAP: _, NODE_ENV: __, ...env } = process.env;
  const p = Bun.spawnSync([process.execPath, "-e", 'await import("./scripts/rustc-suite.ts"); console.log(process.env.RUSTC_BOOTSTRAP ?? "unset")'], { cwd: root, env, stdout: "pipe", stderr: "pipe" });
  expect(p.stdout.toString().trim()).toBe("unset");
});
