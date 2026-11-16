// The iterator tables' catalog of adapters (test/iterator-matrix.ts's
// `adapters` and `stepAdapters`, which the copy matrix steps too) held to
// std's own: each stable method of `Iterator` and `DoubleEndedIterator`
// that takes `self` and makes one of `core::iter::adapters`' structs is an
// adapter of the catalog, or `excludedAdapters` says why it's none. Read
// from the toolchain's std, so a toolchain that adds one fails here until
// the catalog has it. A hand-kept list missed `filter_map` in the copy
// matrix (bug kinds' "enumerated cases").

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { stdLibrary } from "./consumer-net";
import { adapters, excludedAdapters, stepAdapters } from "./iterator-matrix";

/** Each method of `trait` in `file` that takes `self` and returns one of
 * `structs`, and whether it's stable. */
function adapterMethods(file: string, trait: string, structs: Set<string>): { name: string; stable: boolean }[] {
  const text = readFileSync(join(stdLibrary(), file), "utf8");
  const start = text.search(new RegExp(`pub (?:const )?trait ${trait}\\b`));
  const body = text.slice(start, text.indexOf("\n}\n", start));
  const lines = body.split("\n");
  const out: { name: string; stable: boolean }[] = [];
  lines.forEach((line, i) => {
    const fn = /^ {4}fn (\w+)/.exec(line);
    if (!fn) return;
    // Its signature, to its body's brace or its `;`.
    let signature = "";
    for (let j = i; j < lines.length && !/[{;]\s*$/.test(signature); j++) signature += `${lines[j]!.trim()} `;
    const returned = /->\s*([A-Z]\w*)\s*</.exec(signature)?.[1];
    if (!/\(\s*self\b/.test(signature) || returned === undefined || !structs.has(returned)) return;
    let stable = false;
    for (let j = i - 1; j >= 0 && j >= i - 30; j--) {
      const above = lines[j]!.trim();
      if (above.startsWith("#[stable")) stable = true;
      if (above.startsWith("#[unstable")) break;
      if (above.startsWith("fn ") || above === "}") break;
    }
    out.push({ name: fn[1]!, stable });
  });
  return out;
}

test("every stable std adapter is an adapter of the iterator tables' catalog, or excluded with why", () => {
  // The structs `core::iter::adapters` defines.
  const adaptersDir = join(stdLibrary(), "core/src/iter/adapters");
  const structs = new Set(
    [...new Bun.Glob("**/*.rs").scanSync(adaptersDir)].flatMap((file) =>
      [...readFileSync(join(adaptersDir, file), "utf8").matchAll(/^pub struct ([A-Z]\w*)/gm)].map(([, name]) => name!),
    ),
  );
  const std = [...adapterMethods("core/src/iter/traits/iterator.rs", "Iterator", structs), ...adapterMethods("core/src/iter/traits/double_ended.rs", "DoubleEndedIterator", structs)];
  expect(std.map((m) => m.name)).toEqual(expect.arrayContaining(["map", "filter_map", "zip", "rev", "cycle"]));
  const catalog = new Set([...adapters, ...stepAdapters].map((a) => a.name.split(" ")[0]));
  const missing = std.filter((m) => m.stable && !catalog.has(m.name) && !(m.name in excludedAdapters)).map((m) => m.name);
  expect(missing).toEqual([]);
  // And each exclusion is of one std has, stable, and not in the catalog.
  for (const name of Object.keys(excludedAdapters)) {
    expect([name, std.some((m) => m.name === name && m.stable), catalog.has(name)]).toEqual([name, true, false]);
  }
});
