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

// ADR 0753: upstream's `if (indent >= 3) { // .. } else if (..)`, the
// branch of only comments kept, with them.
test("addObjectDiffToProperties keeps its branch of only comments", () => {
  const js = of("ReactPerformanceTrackProperties.js");
  expect(js).toContain(
    "        if (indent >= 3) {\n          // Just fallthrough to print the two values if we're deep.\n          // This will skip nested properties of the objects.\n        } else if (\n",
  );
});
