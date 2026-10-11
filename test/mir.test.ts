// Bodies lowered from their MIR (ADR 0364), while the proof of concept
// runs: each corpus case `mir-corpus.txt` lists, compiled with
// `RUST_JS_MIR=1`, does what native Rust does, or one `compile-fail` says
// rust-js refuses is refused, its first error the same. The list only grows: a case
// that passes is added, and one listed that fails is a regression.
//
//   bun scripts/mirCorpus.ts   the corpus under MIR, and what's not yet lowered

import { beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { agree, compileJs, node, runJs, runNative, show } from "./programs";
import { buildCompiler, fixture, root } from "./support";

const listed = readFileSync(join(root, "test/mir-corpus.txt"), "utf8").split("\n").filter(Boolean);

beforeAll(() => buildCompiler());

for (const name of listed) {
  test(`${name}, from its MIR`, () => {
    const file = join(root, "test/corpus", name);
    const dir = fixture(`mir-${name.replace(/\.rs$/, "")}`);
    const source = readFileSync(file, "utf8");
    const edition = /^\/\/@ edition: (\d+)$/m.exec(source)?.[1] ?? "2024";
    const refused = /^\/\/@ compile-fail: (.+)$/m.exec(source)?.[1];
    if (refused !== undefined) {
      const compiled = compileJs(file, dir, edition, false, { RUST_JS_MIR: "1" });
      if (!("error" in compiled)) throw new Error("compiled, but `compile-fail` says it can't be");
      if (compiled.kind === "crashed") throw new Error(compiled.error);
      expect(compiled.error.split("\n").find((line) => line.startsWith("error")) ?? "").toContain(refused);
      return;
    }
    const native = runNative(file, dir, edition);
    if (typeof native === "string") throw new Error(native);
    const compiled = compileJs(file, dir, edition, false, { RUST_JS_MIR: "1" });
    if ("error" in compiled) throw new Error(compiled.error);
    const run = runJs([node ?? "node"], compiled.js, dir, "mir");
    expect(agree(run, native) ? "agrees" : `ended ${show(run.outcome)}, native Rust ${show(native.outcome)}`).toBe("agrees");
    // Built as a library too, as the corpus builds it (ADR 0360), but where
    // a directive says that's refused.
    if (!/^\/\/@ library-refused:/m.test(source)) {
      const library = compileJs(file, dir, edition, true, { RUST_JS_MIR: "1" });
      if ("error" in library) throw new Error(library.error);
      const libraryRun = runJs([node ?? "node"], library.js, dir, "mir-library");
      expect(agree(libraryRun, native) ? "agrees" : `as a library, ended ${show(libraryRun.outcome)}`).toBe("agrees");
    }
  }, 120_000);
}

// And the JSX tests `mir-jsx.txt` lists, which pass from MIR: each runs, and
// passes. `bun scripts/mirJsx.ts` says which others pass now.
test("the JSX tests mir-jsx.txt lists, from their MIR", () => {
  const listed = readFileSync(join(root, "test/mir-jsx.txt"), "utf8").split("\n").filter(Boolean);
  const run = Bun.spawnSync(["bun", "test", "test/jsx.test.ts"], {
    cwd: root,
    env: { ...process.env, RUST_JS_MIR: "1", RUST_JS_MIR_ALL: "0" },
    stdout: "pipe",
    stderr: "pipe",
  });
  const log = run.stdout.toString() + run.stderr.toString();
  expect(run.exitCode === 0 ? "passed" : log.slice(-4000)).toBe("passed");
  expect(Number(/^ (\d+) pass$/m.exec(log)?.[1] ?? 0)).toBe(listed.length);
}, 300_000);
