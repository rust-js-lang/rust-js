// Campaign 6's stopgap refusals (#2539, ADR 1201): each refusal that's
// there only because a crossing doesn't convert a form or a fact yet says
// so by the refusal API, `Refused::Unconverted(Crossing::…, ..)` or
// `unconverted(span, Crossing::…, ..)`, and `Crossing::lifted_by` maps each
// cell to the one open issue that lifts it. The cells only shrink: one no
// refusal names any more is removed, as its refusals were lifted; a new
// one is added with its open issue. Whether each issue is open is
// GitHub's, checked as one closes and daily (.github/workflows/issue-labels.yml).
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";
import { compilerRustFiles } from "../scripts/compiler-crates";
import { cells, closed, owner } from "../scripts/stopgap-refusals";

const read = (path: string) => readFileSync(join(root, path), "utf8");
const source = read(owner);
const declared = [...source.slice(source.indexOf("pub enum Crossing {")).split("\n}\n")[0].matchAll(/^ {4}(\w+),$/gm)].map((m) => m[1]);

// Each cell is mapped to its issue once, and no two cells share one: a
// fix lifts its cell's refusals together, and an issue two cells shared
// couldn't close for either. The workflow reads the same arms.
test("each stopgap cell is mapped to an issue of its own", () => {
  const listed = cells(source);
  expect(listed.map(({ crossing }) => crossing)).toEqual(declared);
  const issues = listed.map(({ issue }) => issue);
  expect(issues.filter((issue, at) => issues.indexOf(issue) !== at)).toEqual([]);
});

// Each cell is refused somewhere: one whose refusals are all lifted is
// removed with its issue, so the list only shrinks as the campaign lifts
// them. The count is exact, so a refusal lifted or added is seen here.
test("each stopgap cell is a refusal's, and they're counted", () => {
  const uses = compilerRustFiles()
    .filter((file) => file !== owner)
    .flatMap((file) => [...read(file).matchAll(/\bCrossing::(\w+)/g)].map((m) => m[1]));
  expect(declared.filter((crossing) => !uses.includes(crossing))).toEqual([]);
  expect(uses.filter((crossing) => !declared.includes(crossing))).toEqual([]);
  expect(uses.length).toBe(52);
});

// A stopgap is said as what it refuses is: `Unconverted` wraps the words
// it would have had, and nothing else makes one.
test("a stopgap refusal is made by the refusal API alone", () => {
  // `lib.rs`'s `unconverted` is `unsupported`'s, of a stopgap, given its cell.
  const wrapper = "crates/rust-js-lower/src/lib.rs";
  const made = compilerRustFiles().filter((file) => file !== owner && file !== wrapper && /Refused::Unconverted\(/.test(read(file)));
  for (const file of made) {
    for (const m of read(file).matchAll(/Refused::Unconverted\(\s*([\w:]+)/g)) expect(m[1], file).toMatch(/Crossing::\w+$/);
  }
  expect(read(wrapper)).toContain("refusal::Refused::Unconverted(crossing, &refusal::Refused::Feature(what))");
});

// The workflow's check, of states given: a cell whose issue is closed is named.
test("a stopgap cell whose issue is closed is named", () => {
  const listed = cells(source);
  expect(closed(listed, () => "OPEN")).toEqual([]);
  expect(closed(listed, (issue) => (issue === 2546 ? "CLOSED" : "OPEN"))).toEqual([{ crossing: "TrustedLen", issue: 2546 }]);
});
