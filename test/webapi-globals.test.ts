// A global a host may lack is read by a call (#1862, ADR 1157):
// `window_global::document()` is JS's bare `document`, read where it's
// called, so the lazy fallback clippy asks for, `doc.unwrap_or_else(
// window_global::document)`, never reads it where a document is given,
// and reads it, throwing JS's ReferenceError, where none is and the host
// has none. Codex's SSR regression: upstream's `activeElement(doc =
// document)` (vercel-labs/ui@220c1a0, packages/core/src/foundation/platform).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

test("a global a host may lack is read only where it's called, its ReferenceError JS's", async () => {
  const dir = fixture("webapi-globals");
  writeFileSync(join(dir, "lib.rs"), `use js::function;
use webapi::{Document, Element, Performance, global, window_global};

/// The focused element of \`doc\`, or of the page's document.
pub fn active_element(doc: Option<&'static Document>) -> Option<&'static Element> {
    let doc = doc.unwrap_or_else(window_global::document);
    doc.active_element()
}

pub fn has_document() -> bool {
    js::is_defined(window_global::document)
}

/// React's \`performance.measure.bind(performance, name)\`: each read of a
/// global that runs nothing is the same, so its \`this\` is the method's.
pub fn measurer(name: &'static str) -> &'static dyn Fn() -> &'static webapi::PerformanceMeasure {
    function::bind(Performance::measure, Some(&global::performance()), (global::performance(), name, None, None))
}
`);
  const built = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...flags], { cwd: root, stderr: "pipe" });
  expect([built.exitCode, built.stderr.toString()]).toEqual([0, ""]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("doc ??= document;");
  expect(js).toContain('return typeof document !== "undefined";');
  expect(js).toContain("return performance.measure.bind(performance, name);");
  // A server's: none of the page's globals.
  const host = globalThis as Record<string, unknown>;
  for (const name of ["window", "document", "HTMLElement", "matchMedia", "navigator"]) Reflect.deleteProperty(host, name);
  const lib = await import(join(dir, "lib.js"));
  const calls: string[] = [];
  const doc = { get activeElement() { calls.push("active"); return null; } };
  // Given a document, the global's never read.
  expect(lib.activeElement(doc)).toBeNull();
  expect(calls).toEqual(["active"]);
  expect(lib.hasDocument()).toBe(false);
  // None given, and none there: JS's own error, as upstream's default
  // parameter throws.
  expect(() => lib.activeElement()).toThrow(ReferenceError);
});
