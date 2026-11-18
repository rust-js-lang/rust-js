// A JS class's instances are objects, so each binding of one is what every
// object's is, `OBJECT_MARKERS`: never `undefined` nor `null`, an `Option`
// of one the object itself (ADR 0051), and what a `WeakMap` or a `WeakSet`
// holds (ADR 0677). The generators write them by `objectMarkers`, and this
// holds every binding crate's `Classed` type to them, written by hand too:
// `Intl.Locale` was neither, so `js::unknown(locale)` was refused (ADR
// 1172, amended).

import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { OBJECT_MARKERS, objectMarkers } from "../scripts/object-markers";
import { root } from "./support";

const crates = ["builtins/src", "webapi/src", "node/src", "react/src", "next/src"];

/** Each type of `source` an `impl` of `marker` names, by its name alone. */
const implementors = (source: string, marker: string) =>
  new Set([...source.matchAll(new RegExp(`impl(?:<[^>{]*>)? (?:\\w+::)*${marker} for (\\w+)`, "g"))].map((m) => m[1]));

test("each crate's class binding is an object's", () => {
  const missing: string[] = [];
  let classes = 0;
  for (const crate of crates) {
    const dir = join(root, crate);
    const source = readdirSync(dir, { recursive: true })
      .map(String)
      .filter((file) => file.endsWith(".rs"))
      .map((file) => readFileSync(join(dir, file), "utf8"))
      .join("\n");
    const classed = implementors(source, "Classed");
    classes += classed.size;
    for (const marker of OBJECT_MARKERS) {
      const marked = implementors(source, marker);
      for (const name of classed) if (!marked.has(name)) missing.push(`${crate}: ${name} is no \`${marker}\``);
    }
  }
  expect(classes).toBeGreaterThan(700);
  expect(missing).toEqual([]);
});

test("the markers are written one way", () => {
  expect(objectMarkers("Locale", "super::")).toEqual(["unsafe impl super::Defined for Locale {}", "unsafe impl super::WeakKey for Locale {}"]);
  expect(objectMarkers("DataView<B>", "super::", "<B>")).toEqual([
    "unsafe impl<B> super::Defined for DataView<B> {}",
    "unsafe impl<B> super::WeakKey for DataView<B> {}",
  ]);
});
