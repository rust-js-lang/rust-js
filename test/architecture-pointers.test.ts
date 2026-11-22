// Pointer provenance's rule of the architecture (#1947): whether a call may
// keep a raw pointer to what a `&mut` it's given refers to is one answer,
// `pointers/keeps.rs`'s `pointer_may_keep` (#1781), which lowering asks of a
// `&mut` it gives generic code and the MIR intervals of a `Vec` they track
// given to the crate's code (#1966). A second walk of what a call may keep,
// written elsewhere, would answer it differently where the two disagree.
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";
import { compilerRustFiles } from "../scripts/compiler-crates";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// The compiler's sources, in name order.
const sources = compilerRustFiles().sort();

test("only pointers/ answers whether a call may keep a pointer", () => {
  // Each function whose name says it answers it, by where it's defined.
  const answers = sources.flatMap((file) =>
    [...read(file).matchAll(/\bfn ([a-z_]*(?:may_keep|keeps_no_pointer|keeps_a_pointer|kept_pointer)[a-z_]*)\b/g)].map((m) => `${file}: ${m[1]}`),
  );
  expect(answers.filter((found) => !found.startsWith("src/lower/pointers/"))).toEqual([]);
  expect(answers).toContain("src/lower/pointers/keeps.rs: pointer_may_keep");
});
