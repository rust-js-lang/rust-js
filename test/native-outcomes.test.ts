// A native program's run kept by what it was built from (test/native-
// outcomes.ts, ADR 0567 amended): used again while nothing its build read
// has changed, and not of a case whose run reads the world, nor afresh.

import { afterEach, beforeAll, expect, test } from "bun:test";
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { cases, corpus, directives } from "./corpus-cases";
import { nativeOutcomes } from "./kept";
import { keepRun, keptRun, recheck } from "./native-outcomes";
import { nativeLimitKey, nativeRunKey, runNative, type Run } from "./programs";
import { record, verified } from "./verified-runs";
import { nativeCratesOutput } from "./crates";
import { buildCompiler, fixture, nativeCratesKey, nativeKey, root } from "./support";

beforeAll(buildCompiler, 600_000);

const recheckWas = process.env.RUST_JS_NATIVE_RECHECK;
afterEach(() => {
  if (recheckWas === undefined) delete process.env.RUST_JS_NATIVE_RECHECK;
  else process.env.RUST_JS_NATIVE_RECHECK = recheckWas;
  delete process.env.RUST_JS_OUTCOME_VALUE;
});

const ran = (r: Run | string): Run => {
  if (typeof r === "string") throw new Error(r);
  return r;
};

/** The case `main`, in a fixture of its own, and what's beside it. */
function program(name: string, main: string, beside: Record<string, string> = {}): string {
  const dir = fixture(`native-outcome-${name}`);
  for (const [file, text] of Object.entries(beside)) writeFileSync(join(dir, file), text);
  const file = join(dir, "case.rs");
  writeFileSync(file, main);
  // What an earlier run of this file kept of it, its fixture the same.
  rmSync(join(nativeOutcomes, nativeRunKey(file).slice(0, 32)), { recursive: true, force: true });
  return file;
}

/** The kept run of `file`'s program, its stdout made `said`, as a kept run
 * that a key missed an input of would say. */
function tampered(file: string, said: string) {
  const program = join(nativeOutcomes, nativeRunKey(file).slice(0, 32));
  for (const build of readdirSync(program)) {
    const at = join(program, build, "run.json");
    const kept = JSON.parse(readFileSync(at, "utf8"));
    kept.stdout = said;
    kept.bytes.stdout = Buffer.from(said).toString("base64");
    writeFileSync(at, JSON.stringify(kept));
  }
}

test("a native run is kept by what it was built from, and given again", () => {
  const file = program("kept", 'fn main() {\n    println!("kept");\n}\n');
  expect(keptRun(nativeRunKey(file))).toBeUndefined();
  const first = ran(runNative(file, join(file, "..")));
  expect(first.stdout).toBe("kept\n");
  const kept = keptRun(nativeRunKey(file));
  expect(kept?.stdout).toBe("kept\n");
  expect(kept?.bytes.stdout.equals(first.bytes.stdout)).toBe(true);
  expect(kept?.outcome).toEqual(first.outcome);
  // What's given is the kept run, not the program run again.
  tampered(file, "as kept\n");
  expect(ran(runNative(file, join(file, ".."))).stdout).toBe("as kept\n");
  // And run afresh, the program's own.
  expect(ran(runNative(file, join(file, ".."), "2024", "host", [], { fresh: true })).stdout).toBe("kept\n");
});

test("a native run isn't given again once a file its build included changes", () => {
  const file = program("included", 'fn main() {\n    println!("{}", include!("value.rs"));\n}\n', { "value.rs": "1\n" });
  expect(ran(runNative(file, join(file, ".."))).stdout).toBe("1\n");
  writeFileSync(join(file, "..", "value.rs"), "2\n");
  expect(ran(runNative(file, join(file, ".."))).stdout).toBe("2\n");
});

test("a native run isn't given again once a variable its build read changes", () => {
  const file = program("env", 'fn main() {\n    println!("{}", env!("RUST_JS_OUTCOME_VALUE"));\n}\n');
  process.env.RUST_JS_OUTCOME_VALUE = "a";
  expect(ran(runNative(file, join(file, ".."))).stdout).toBe("a\n");
  process.env.RUST_JS_OUTCOME_VALUE = "b";
  expect(ran(runNative(file, join(file, ".."))).stdout).toBe("b\n");
});

// rustc's dep-info lists no rlib, so what a directory `-L dependency=`
// names holds is the key's, the crates an `--extern`'s rlib was built
// against: another build of them is another program.
test("a native program's key holds what its dependencies' directory holds", () => {
  const deps = fixture("native-outcome-deps");
  const key = () => nativeKey("fn main() {}\n", deps, ["--edition=2024", "-L", `dependency=${deps}`]);
  writeFileSync(join(deps, "libdep.rlib"), "one");
  const one = key();
  expect(key()).toBe(one);
  writeFileSync(join(deps, "libdep.rlib"), "two");
  expect(key()).not.toBe(one);
  writeFileSync(join(deps, "libother.rlib"), "two");
  expect(key()).not.toBe(one);
});

test("a case whose run reads the world isn't kept, nor a program made of it", () => {
  const marked = program("marked", 'fn main() {\n    println!("{}", std::env::args().count());\n}\n//@ nondeterministic: its run reads its arguments\n');
  ran(runNative(marked, join(marked, "..")));
  expect(keptRun(nativeRunKey(marked))).toBeUndefined();
  const rewrite = join(marked, "..", "rewrite.rs");
  writeFileSync(rewrite, 'fn main() {\n    println!("{}", std::env::args().count() + 0);\n}\n');
  rmSync(join(nativeOutcomes, nativeRunKey(rewrite).slice(0, 32)), { recursive: true, force: true });
  ran(runNative(rewrite, join(marked, ".."), "2024", "host", [], { of: marked }));
  expect(keptRun(nativeRunKey(rewrite))).toBeUndefined();
});

// A chain of crates' program (test/crates.ts) is kept as a case's is, but
// where one of its crates says it reads the world.
test("a chain of crates' native run is kept, but where a crate reads the world", () => {
  const dir = fixture("native-outcome-chain");
  const crate = (name: string, text: string) => {
    writeFileSync(join(dir, `${name}.rs`), text);
    return join(dir, `${name}.rs`);
  };
  const dep = crate("dep", "pub fn main() {\n    println!(\"chained\");\n}\n");
  const main = "fn main() {\n    dep::main();\n}\n";
  const flags = ["--edition=2024"];
  const of = [{ name: "dep", file: dep, uses: [] }];
  rmSync(join(nativeOutcomes, nativeCratesKey(of, main, flags).slice(0, 32)), { recursive: true, force: true });
  expect(nativeCratesOutput(of, main, flags)()).toBe("chained\n");
  expect(keptRun(nativeCratesKey(of, main, flags))?.stdout).toBe("chained\n");
  const marked = crate("marked", "pub fn main() {\n    println!(\"{}\", std::env::args().count());\n}\n//@ nondeterministic: its run reads its arguments\n");
  const ofMarked = [{ name: "marked", file: marked, uses: [] }];
  const markedMain = "fn main() {\n    marked::main();\n}\n";
  rmSync(join(nativeOutcomes, nativeCratesKey(ofMarked, markedMain, flags).slice(0, 32)), { recursive: true, force: true });
  expect(nativeCratesOutput(ofMarked, markedMain, flags)()).toBe("1\n");
  expect(keptRun(nativeCratesKey(ofMarked, markedMain, flags))).toBeUndefined();
});

// macOS's share is of what's free as a test starts, which moves with the
// load: its bytes in the key lost every kept run beside a busy check (#2340).
test("a native run's key says the memory it's given by its kind, never by what's free", () => {
  expect(nativeLimitKey("darwin")).toBe("share");
  expect(nativeLimitKey("linux")).toMatch(/^data \d+ KiB$/);
});

test("a run stopped by its time or memory isn't kept", () => {
  const stopped: Run = { stdout: "", stderr: "", bytes: { stdout: Buffer.from(""), stderr: Buffer.from("") }, outcome: "timed out", stopped: true };
  const store = fixture("native-outcome-stopped-store");
  const build = fixture("native-outcome-stopped-build");
  writeFileSync(join(build, "inputs.json"), JSON.stringify({ files: [], env: [] }));
  keepRun("0".repeat(64), build, stopped, store);
  expect(readdirSync(store)).toEqual([]);
  // The same run, ended, is kept.
  keepRun("0".repeat(64), build, { ...stopped, stopped: undefined, outcome: { exit: 0 } }, store);
  expect(keptRun("0".repeat(64), store)?.outcome).toEqual({ exit: 0 });
});

// The nightly runs a share of the kept runs again (`RUST_JS_NATIVE_RECHECK`):
// one that never says what was kept is a key missing an input, and fails.
test("a kept run the program never says again fails its recheck", () => {
  const file = program("rechecked", 'fn main() {\n    println!("rechecked");\n}\n');
  ran(runNative(file, join(file, "..")));
  process.env.RUST_JS_NATIVE_RECHECK = "1";
  expect(ran(runNative(file, join(file, ".."))).stdout).toBe("rechecked\n");
  tampered(file, "stale\n");
  expect(() => runNative(file, join(file, ".."))).toThrow("its key misses an input");
  // One answer of a few a seed picks, said again once in its tries, passes.
  const kept = ran(runNative(file, join(file, ".."), "2024", "host", [], { fresh: true }));
  let n = 0;
  const other = { ...kept, stdout: "other\n", bytes: { ...kept.bytes, stdout: Buffer.from("other\n") } };
  expect(recheck(kept, () => (n++ < 3 ? other : kept))).toBeUndefined();
  expect(recheck(kept, () => other)).toContain("its key misses an input");
  // A program whose every run says another answer, a `RandomState` map's
  // drops printed in its order, says none of them twice: the kept run is
  // one of its answers, no input missed (#2461).
  let k = 0;
  const each = () => ({ ...kept, stdout: `run ${k}\n`, bytes: { ...kept.bytes, stdout: Buffer.from(`run ${k++}\n`) } });
  expect(recheck(kept, each)).toBeUndefined();
});

// And of the corpus's verified runs (ADR 0567), the nightly's share is run
// again, not taken as verified.
test("a verified run is run again of the nightly's share", () => {
  const cache = fixture("native-outcome-verified");
  record("k", "note", cache, () => Number.POSITIVE_INFINITY);
  expect(verified("k", cache)).toBe(true);
  process.env.RUST_JS_NATIVE_RECHECK = "1";
  expect(verified("k", cache)).toBe(false);
});

// Each case whose code reads the clock, a random seed, its arguments, its
// environment, the file system, its process's id or its input carries
// `//@ nondeterministic: <why>`, so its run is never kept.
test("every corpus case whose run reads the world says it's nondeterministic", () => {
  const apis = [
    /\b(Instant|SystemTime)::now\b/,
    /\bRandomState\b/,
    /\benv::(args|args_os|var|vars|var_os|vars_os|current_dir|temp_dir|home_dir)\b/,
    /\b(fs::|File::|OpenOptions\b)/,
    /\bprocess::id\b/,
    /\bstdin\(\)/,
  ];
  const unmarked = cases.filter((name) => {
    const text = readFileSync(join(corpus, name), "utf8");
    const code = text.replace(/\/\/[^\n]*/g, "");
    const want = directives(text);
    return apis.some((api) => api.test(code)) && typeof want === "object" && want.nondeterministic === undefined;
  });
  expect(unmarked).toEqual([]);
});

// A run still being kept is made where a reader skips it, a name that
// begins with a dot: one named `<build>.<pid>` was listed as a build, and
// its `run.json` read half written by another part of the corpus,
// "Unexpected EOF" (#2299's push). Half of one there, none is read.
test("a run still being kept is never read half written", () => {
  const store = fixture("native-outcomes-making");
  rmSync(store, { recursive: true, force: true });
  const making = join(store, "a".repeat(32), ".build.12345");
  mkdirSync(making, { recursive: true });
  writeFileSync(join(making, "inputs.json"), "{}");
  writeFileSync(join(making, "run.json"), '{"stdout": "half');
  expect(keptRun("a".repeat(32), store)).toBeUndefined();
  const source = readFileSync(join(root, "test", "native-outcomes.ts"), "utf8");
  expect(source).toContain("const making = join(store, key.slice(0, 32), `.${build.split(\"/\").at(-1)!}.${process.pid}`);");
});
