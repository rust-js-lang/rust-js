// The known bugs the tests must catch (ADR 0093): each applies to the
// compiler as it is, exactly once, and names tests to catch it. Building
// and running them is scripts/mutations.ts's.

import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { judge, lists, mutate, mutations, syncTree } from "../scripts/mutations";
import type { Exit } from "./child";
import { fixture, root } from "./support";

test("every mutation applies to the compiler as it is, once", () => {
  expect(mutations.length).toBeGreaterThan(0);
  expect(new Set(mutations.map((m) => m.name)).size).toBe(mutations.length);
  for (const m of mutations) {
    const mutated = mutate(readFileSync(join(root, m.file), "utf8"), m);
    expect([m.name, typeof mutated === "string" ? "applies" : mutated.problem]).toEqual([m.name, "applies"]);
    expect(m.tests.length).toBeGreaterThan(0);
  }
});

// So code that moves takes its mutations with it, to the list of where it went.
test("each source file's mutations are in its own list", () => {
  for (const list of lists) {
    const source = `src/${list.path.replace(/\.ts$/, "")}`;
    for (const m of list.mutations) expect([m.name, m.file.replace(/\.[a-z]+$/, "")]).toEqual([m.name, source]);
  }
});

test("a mutation whose code moved, or is there twice, says so", () => {
  const m = { name: "x", breaks: "", file: "a.rs", find: "let a = 1;", replace: "let a = 2;", tests: ["t"] };
  expect(mutate("fn f() { let a = 1; }", m)).toBe("fn f() { let a = 2; }");
  expect(mutate("fn f() {}", m)).toEqual({ problem: "doesn't apply: a.rs has no `let a = 1;`" });
  expect(mutate("let a = 1; let a = 1;", m)).toEqual({ problem: "applies more than once in a.rs" });
});

// A mutant is caught only by a test that failed: a runner that ran out of
// time, was stopped, or failed before any test did, says nothing of it.
// Found in review: every failed process counted as caught.
test("a mutant is caught, survives, or the run says nothing of it", () => {
  const exit = (code: number | null, more: Partial<Exit> = {}): Exit => ({ code, signal: null, timedOut: false, overflowed: false, stdout: "", stderr: "", bytes: { stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) }, ...more });
  const failed = "(fail) wrapping.rs [12.00ms]\n\n 0 pass\n 1 fail\n";
  expect(judge(exit(1), failed)).toBe("caught");
  expect(judge(exit(0), "(pass) wrapping.rs\n\n 1 pass\n 0 fail\n")).toBe("survived");
  expect(judge(exit(null, { timedOut: true, signal: "SIGTERM" }), failed)).toBe("inconclusive");
  expect(judge(exit(null, { signal: "SIGKILL" }), "")).toBe("inconclusive");
  expect(judge(exit(1), "error: Cannot find module './corpus'\n\n 0 pass\n 1 fail\n")).toBe("inconclusive");
  expect(judge(exit(1), "(fail) (unnamed) [0.3ms]\n\n 0 pass\n 1 fail\n")).toBe("inconclusive");
  expect(judge(exit(0), " 0 pass\n 0 fail\n")).toBe("inconclusive");
  // A long diff bun test cuts short doesn't end its last line, and the
  // `(fail)` after it is printed on that line. Found running `f32_digits`.
  expect(judge(exit(1), "+ 38818(fail) f32_digits.rs [409.30ms]\n\n 0 pass\n 1 fail\n")).toBe("caught");
  // A test that ran out of time failed as bun test prints it, but said
  // nothing of what the JS does. Found in review: it counted as caught.
  const timedOut = "(fail) wrapping.rs [5001.00ms]\n  ^ this test timed out after 5000ms.\n";
  expect(judge(exit(1), `${timedOut}\n 0 pass\n 1 fail\n`)).toBe("inconclusive");
  expect(judge(exit(1), `${timedOut}${failed}`)).toBe("caught");
  // A hook that ran out of time, as bun test prints it: the test never ran.
  const hookTimedOut = "(fail) never reaches assertion [101.76ms]\n  ^ a beforeEach/afterEach hook timed out for this test.\n";
  expect(judge(exit(1), `${hookTimedOut}\n 0 pass\n 1 fail\n`)).toBe("inconclusive");
});

// The mutated crate is kept between mutations and runs: only what differs
// from the checkout is written again, so cargo rebuilds what changed, not all
// of rust-js, nine seconds of each mutation's (DEVELOPMENT.md).
test("the crate a mutation is built in is written only where it differs", () => {
  const from = fixture("sync-from"), to = fixture("sync-to");
  mkdirSync(join(from, "src"), { recursive: true });
  writeFileSync(join(from, "src", "a.rs"), "a");
  writeFileSync(join(from, "src", "b.rs"), "b");
  writeFileSync(join(from, "Cargo.toml"), "toml");
  expect(syncTree(from, to, ["Cargo.toml", "src"]).sort()).toEqual(["Cargo.toml", "src/a.rs", "src/b.rs"]);
  const kept = statSync(join(to, "src", "a.rs")).mtimeMs;
  writeFileSync(join(from, "src", "b.rs"), "b2");
  rmSync(join(from, "Cargo.toml"));
  writeFileSync(join(to, "src", "stale.rs"), "gone");
  expect(syncTree(from, to, ["Cargo.toml", "src"]).sort()).toEqual(["Cargo.toml", "src/b.rs", "src/stale.rs"]);
  expect([statSync(join(to, "src", "a.rs")).mtimeMs, readFileSync(join(to, "src", "b.rs"), "utf8"), existsSync(join(to, "src", "stale.rs")), existsSync(join(to, "Cargo.toml"))])
    .toEqual([kept, "b2", false, false]);
});
