// The reducer (#1088, `scripts/reduce.ts`): a failing program reduced to a
// small one that fails the same way, by the rewriter's reductions
// (`metamorph reduce`), each kept only where rustc still accepts it and it
// still fails as the oracle says.
import { beforeAll, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { buildRewriter, rewriter } from "./metamorphic";
import { buildCompiler, fixture } from "./support";
import { underLoad } from "../scripts/timing";
import { reduce, size } from "../scripts/reduce";

beforeAll(() => {
  buildCompiler();
  buildRewriter();
}, 900_000);

/** A program rust-js refuses, `set_hook`'s, among what has nothing to do
 * with it. */
const refused = `struct Unused(u8);

fn helper(x: u32) -> u32 {
    x + 1
}

fn main() {
    let a = helper(2);
    println!("{a} {}", Unused(3).0);
    std::panic::set_hook(Box::new(|_| {}));
    let v = vec![1, 2, 3];
    if v.len() > 2 {
        println!("{}", v[0]);
    } else {
        println!("short");
    }
}
`;

test("a program rust-js refuses is reduced to what it refuses, the rest gone", async () => {
  const dir = fixture("reduce-refused");
  mkdirSync(dir, { recursive: true });
  const file = join(dir, "refused.rs");
  writeFileSync(file, refused);
  const reduced = await reduce(file, { oracle: "error:set_hook", jobs: 4, budget: 240 });
  if (typeof reduced === "string") throw new Error(reduced);
  expect(reduced.failure.kind).toBe("rejected");
  expect(reduced.program).toContain("set_hook");
  for (const gone of ["helper", "Unused", "vec!", "short"]) expect(reduced.program).not.toContain(gone);
  expect(reduced.to).toBeLessThan(reduced.from / 3);
}, 300_000);

test("a program that runs as native Rust does isn't reduced", async () => {
  const dir = fixture("reduce-passes");
  mkdirSync(dir, { recursive: true });
  const file = join(dir, "passes.rs");
  writeFileSync(file, `fn main() {\n    println!("{}", 1 + 2);\n}\n`);
  expect(await reduce(file, { jobs: 2, budget: 60 })).toContain("runs as native Rust does");
}, underLoad(120_000));

test("each of the rewriter's reductions is smaller, and past the last there's none", () => {
  const dir = fixture("reduce-reductions");
  mkdirSync(dir, { recursive: true });
  const file = join(dir, "refused.rs");
  writeFileSync(file, refused);
  const count = Number(spawnSync(rewriter, ["reduce", file], { encoding: "utf8" }).stdout.trim());
  expect(count).toBeGreaterThan(10);
  // Each smaller, but an inlining, which writes a `let` of each argument.
  let smaller = 0;
  for (let n = 0; n < count; n++) {
    const made = spawnSync(rewriter, ["reduce", file, String(n)], { encoding: "utf8" });
    expect([n, made.status]).toEqual([n, 0]);
    if (size(made.stdout) < size(refused)) smaller++;
  }
  expect(smaller).toBeGreaterThanOrEqual(count - 1);
  expect(spawnSync(rewriter, ["reduce", file, String(count)], { encoding: "utf8" }).status).toBe(2);
});
