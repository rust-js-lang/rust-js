// React's shared/ReactPerformanceTrackProperties and enqueueTask, as
// react-in-rust ports them (`test/ports/performance-track-properties`): the
// shapes of their JS the port found rust-js writing apart from upstream,
// each as upstream writes it, or as close as its Rust allows.

import { beforeAll, expect, test } from "bun:test";
import { buildReact, ported } from "./support";

beforeAll(buildReact, 600_000);

let of: (file: string) => string;
beforeAll(() => {
  of = ported("performance-track-properties");
}, 600_000);

// Upstream's last line, `return enqueueTaskImpl(task);`, is the port's
// `ENQUEUE_TASK_IMPL.get().unwrap()(task)`: an `Option`'s `unwrap`, which
// panics of a `None` before the task is read, then the call (ADR 0752).
test("enqueueTask calls its implementation, unwrapped", () => {
  const js = of("enqueueTask.js");
  expect(js).toContain("  $unwrap(enqueueTaskImpl)(task);\n}\n");
  expect(js).not.toContain("if (enqueueTaskImpl) {");
});

// ADRs 0750 and 0751: no temporary MIR held is a `const`, `value$1` or a
// `bool` named `value`, and no binding only a guard reads is written.
test("ReactPerformanceTrackProperties holds no temporary of its own", () => {
  const js = of("ReactPerformanceTrackProperties.js");
  expect(js).not.toContain("value$1");
  expect(js).not.toContain("const value = readReactElementTypeof");
  expect(js).not.toContain("const value = UNCHANGED");
  expect(js).not.toContain("const first = ");
  expect(js).toContain("          if (readReactElementTypeof(nextValue) === REACT_ELEMENT_TYPE) {\n");
});

// ADR 0754: upstream's getArrayKind is a loop of an `if`/`else` chain whose
// arms return, with no block MIR made for the arm others go on to.
test("getArrayKind is a chain of returns, no block MIR made", () => {
  const js = of("ReactPerformanceTrackProperties.js");
  const kind = js.slice(js.indexOf("function getArrayKind("), js.indexOf("export function addObjectToProperties("));
  expect(kind).not.toMatch(/bb\d+/);
  expect(kind).toContain('      if (typeof value === "object" && value !== null) {\n        return "Complex";\n      }\n');
});

// ADR 0755: upstream's Promise case, `if (value.status === 'fulfilled') {
// .. return; } else if (value.status === 'rejected') { .. }`, each `return`
// where it's written, no block MIR made for them and no `typeof` before the
// status's strings.
test("addValueToProperties' Promise case returns where it's written", () => {
  const js = of("ReactPerformanceTrackProperties.js");
  expect(js).not.toContain("typeof status");
  expect(js).toContain(
    '        if (objectName === "Promise") {\n          const status = value.status;\n          if (status === "fulfilled") {\n',
  );
  expect(js).toContain(
    '              insertedEntry[1] = "Promise<" + (insertedEntry[1] || "Object") + ">";\n              return;\n            }\n          } else if (status === "rejected") {\n',
  );
});

// ADR 0790: upstream's `switch (typeof value) { case 'object': .. }`, each
// `desc = ..; break;` where it's written, which the port's one `match
// js::classify(value)` is, its object arm's guard `if let Some(value) =
// value` tested by nothing, and its `'desc` block's `break`s the case's:
// no label anywhere.
test("addValueToProperties is a switch whose case breaks where it's written", () => {
  const js = of("ReactPerformanceTrackProperties.js");
  const add = js.slice(js.indexOf("export function addValueToProperties("), js.indexOf("const REMOVED"));
  expect(add).not.toMatch(/^\s*\w+: \{$/m);
  expect(add).not.toMatch(/break \w+;/);
  expect(add).toContain('  switch (typeof value) {\n    case "object":\n      if (value === null) {\n        desc = "null";\n');
  expect(add).toContain('        if (key == null && propsLength === 0) {\n          desc = "<" + typeName + " />";\n          break;\n        }\n');
  expect(add).toContain(
    '          desc = typeof length === "number" ? `${objectName}(${length})` : objectName;\n          break;\n        }\n',
  );
});

// ADR 0753: upstream's `if (indent >= 3) { // .. } else if (..)`, the
// branch of only comments kept, with them.
test("addObjectDiffToProperties keeps its branch of only comments", () => {
  const js = of("ReactPerformanceTrackProperties.js");
  expect(js).toContain(
    "        if (indent >= 3) {\n          // Just fallthrough to print the two values if we're deep.\n          // This will skip nested properties of the objects.\n        } else if (\n",
  );
});
