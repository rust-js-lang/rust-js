// std's private specializations rust-js takes for their default, each a row
// of `OBSERVABLY_DEFAULT` (crates/rust-js-lower/src/recognition/specialization.rs,
// #2395): a row says its specializing impls do what the default does, read
// from std's source. Each row pins that source, its item's text with its
// whitespace runs made one space, by its SHA-256, so a std that changes it
// fails here until the row's reason is read again against the new source
// and its digest moved. The catalogue is the one authority; this reads it.
import { expect, test } from "bun:test";
import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const sysroot = Bun.spawnSync(["rustc", "--print", "sysroot"], { cwd: root }).stdout.toString().trim();
const library = join(sysroot, "lib/rustlib/src/rust/library");

type Row = { path: string; file: string; item: string; digest: string };

/** Each `ObservablyDefault { .. }` row of the catalogue, its fields as written. */
export function rows(): Row[] {
  const source = readFileSync(join(root, "crates/rust-js-lower/src/recognition/specialization.rs"), "utf8");
  const table = source.slice(source.indexOf("pub(crate) const OBSERVABLY_DEFAULT"));
  const field = (block: string, name: string) => block.match(new RegExp(`\\b${name}: "([^"]*)"`))?.[1] ?? "";
  return table
    .split("ObservablyDefault {")
    .slice(1)
    .map((block) => ({ path: field(block, "path"), file: field(block, "file"), item: field(block, "item"), digest: field(block, "digest") }));
}

/** `item`'s text in `text`, from its start to its body's closing brace. */
export function itemText(text: string, item: string): string {
  const start = text.indexOf(item);
  if (start < 0) return "";
  let depth = 0;
  for (let at = text.indexOf("{", start); at < text.length; at++) {
    if (text[at] === "{") depth++;
    if (text[at] === "}" && --depth === 0) return text.slice(start, at + 1);
  }
  return "";
}

/** The row's digest of `text`: its item's, its whitespace runs one space. */
export function digest(text: string, item: string): string {
  return createHash("sha256").update(itemText(text, item).replace(/\s+/g, " ")).digest("hex");
}

test("each specialization taken for its default is std's source as it was read", () => {
  const all = rows();
  expect(all.length).toBeGreaterThan(0);
  for (const row of all) {
    const text = readFileSync(join(library, row.file), "utf8");
    expect(itemText(text, row.item), `${row.path}: ${row.item} in ${row.file}`).not.toBe("");
    // A change to std's source of a row: read its reason again, then move
    // its digest.
    expect([row.path, digest(text, row.item)]).toEqual([row.path, row.digest]);
  }
});

// A rule a compiler file mirrors from std's source where no native run
// can tell it apart, hashbrown's `extend_reserve` of an empty table among
// them (ADR 1191): pinned where it's mirrored, a comment line
// `std-source-pin: <file in rust-src's library/> \`<item>\` <digest>`, the
// same digest as a row's, so a std that changes it fails here until the
// rule is read again.
test("each rule mirrored from std's source is std's source as it was read", () => {
  const sources = (dir: string): string[] =>
    readdirSync(join(root, dir), { withFileTypes: true }).flatMap((entry) =>
      entry.isDirectory() ? sources(join(dir, entry.name)) : entry.name.endsWith(".rs") ? [join(dir, entry.name)] : [],
    );
  const pins = sources("crates").flatMap((file) =>
    [...readFileSync(join(root, file), "utf8").matchAll(/std-source-pin: (\S+) `([^`]+)` ([0-9a-f]{64})/g)].map(
      ([, source, item, pinned]) => ({ file, source, item, pinned }),
    ),
  );
  expect(pins.length).toBeGreaterThan(0);
  for (const pin of pins) {
    const text = readFileSync(join(library, pin.source), "utf8");
    expect(itemText(text, pin.item), `${pin.file}: ${pin.item} in ${pin.source}`).not.toBe("");
    expect([pin.file, digest(text, pin.item)]).toEqual([pin.file, pin.pinned]);
  }
});

