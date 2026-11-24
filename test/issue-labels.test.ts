// The labels every open issue carries (scripts/issue-labels.ts, ADR 1085):
// what `missing` asks of an issue, a P0's `found-by: …` among them (#2481).
import { expect, test } from "bun:test";
import { missing } from "../scripts/issue-labels";

const issue = (...labels: string[]) => ({ number: 1, title: "t", labels: labels.map((name) => ({ name })), author: { login: "nguyenyou" } });
const filed = ["kind: missing case", "area: iterators"];

test("a P0 carries one found-by, as bug-kinds.md says every P0 does", () => {
  expect(missing(issue("P0: wrong answer", ...filed, "found-by: metamorphic"))).toEqual([]);
  expect(missing(issue("P0: wrong answer", ...filed))).toEqual(["a `found-by: …`"]);
  expect(missing(issue("P0: wrong answer", ...filed, "found-by: review", "found-by: audit"))).toEqual([
    "exactly one `found-by: …` (it has found-by: review, found-by: audit)",
  ]);
  // Only a P0's: another priority needs none.
  expect(missing(issue("P2: refused", ...filed))).toEqual([]);
});

test("an issue's priority, kind and area are asked as before", () => {
  expect(missing(issue(...filed))).toEqual(["a priority (`P0: wrong answer` … `P3: readability`), or `tracking`"]);
  expect(missing(issue("P3: readability"))).toEqual(["a `kind: …`", "an `area: …`"]);
  expect(missing(issue("tracking"))).toEqual([]);
  expect(missing(issue("P1: crash or blocker", "dev-loop"))).toEqual([]);
  expect(missing({ ...issue(), title: "Nightly red: 2026-11-24" })).toEqual([]);
});
