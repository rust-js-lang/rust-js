// Programs rust-js once took minutes to compile, each held under a bound
// far above its time now and far below its time then: the corpus's own
// limit, two minutes, is too loose to see one come back (#1214). Each
// bound grows with the machine's load (`underLoad`), to three times.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync } from "node:fs";
import { join } from "node:path";

import { corpus } from "./corpus-cases";
import { buildCompiler, compiler, fixture, root } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildCompiler, 600_000);

/** Each case and its bound: what it took, and takes, on the 12-core M6. */
const bounded: [string, number, string][] = [
  // 116 s before #1214's fix, a walk of what each type argument of a
  // `Drop::drop` call drops, but its type parameters, made once for each
  // path to a type; 0.5 s after.
  ["nested_generic_types_dropped.rs", 20_000, "#1214"],
];

for (const [name, bound, issue] of bounded) {
  test(`${name} compiles in ${bound / 1000} s (${issue})`, () => {
    const dir = fixture(`compile-bounds-${name.replace(/\.rs$/, "")}`);
    mkdirSync(dir, { recursive: true });
    // The compiler run itself, not a compile kept (`runSync`'s, ADR 1099).
    const p = Bun.spawnSync([compiler, join(corpus, name), "-o", join(dir, "case.js"), "--", "--edition=2024", "-Awarnings"], {
      cwd: root,
      stdout: "pipe",
      stderr: "pipe",
      timeout: underLoad(bound),
      killSignal: "SIGKILL",
    });
    expect(p.exitedDueToTimeout ?? false).toBe(false);
    expect(p.exitCode).toBe(0);
  }, underLoad(60_000));
}
