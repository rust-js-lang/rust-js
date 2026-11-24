// The one catalog of the runtime's models (Campaign 5's step 1, ADR 1202):
// which std item each helper of the runtime's stands for, and what proves
// it equal to std's body, crates/rust-js-lower/src/recognition/models.rs,
// which the compiler prints (`rust-js --std-models`) and the twin net reads
// (test/twin-net.ts). No second list, in the compiler or in TypeScript, as
// twin-net.ts' `pitted`, `shared` and `why` once were (#2513, #1458).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] =>
  readdirSync(join(root, directory), { withFileTypes: true })
    .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
    .flatMap((entry) => (entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]));

const catalog = "crates/rust-js-lower/src/recognition/models.rs";

test("the compiler's catalog of the runtime's models is models.rs alone", () => {
  const declares = /\b(?:const|static) MODELS\b|&\[Entry\]/;
  const sources = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), ...files("crates/rust-js-program/src")].filter((file) => file.endsWith(".rs"));
  expect(sources.filter((file) => declares.test(read(file)))).toEqual([catalog]);
});

test("no list of the runtime's models' proofs in TypeScript", () => {
  const reasons = "glue|collections|consumers|iterators|js-own|memory|ranges|sort|format|cells|numbers";
  const entry = new RegExp(`^\\s*"\\$[A-Za-z0-9_]+": "(${reasons})",`, "m");
  const list = /export const (pitted|shared)\b|const why = \{\s*\n\s*glue:/;
  const own = "test/architecture-models.test.ts";
  const ts = (directory: string) => files(directory).filter((file) => file.endsWith(".ts"));
  const found = [...ts("test"), ...ts("scripts")].filter((file) => file !== own && (entry.test(read(file)) || list.test(read(file))));
  expect(found).toEqual([]);
});
