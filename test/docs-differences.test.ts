// ADR 0262's exceptions are named by their number in the guide's list of
// differences (docs/guide/semantics.md, "Differences from native Rust, in
// one list"), the one list that numbers them: each exception its
// Consequences list names cites `semantics.md`, N, a difference the guide
// has. An `rd` id is the React port's review gap (rd114, rd731, ..),
// never an exception's own name: in the Consequences it's only the gap
// that led to one, written `gap rdN` or `gaps rdN, rdM` (the
// coordinator's decision, 2026-11-11; rd802 and rd803 had been taken as
// the names of two exceptions, ids the port's review gaps already held).
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const adr = readFileSync(
  join(root, "docs/decisions/0262-when-rust-and-js-disagree.md"),
  "utf8",
);
const guide = readFileSync(join(root, "docs/guide/semantics.md"), "utf8");

/** The Consequences list's entries, each one line of words. */
function consequences(): string[] {
  const from = adr.indexOf("\n## Consequences\n");
  const to = adr.indexOf("\n## ", from + 1);
  expect(from).toBeGreaterThan(0);
  return adr
    .slice(from, to < 0 ? undefined : to)
    .split(/\n(?=- \*\*)/)
    .slice(1)
    .map((entry) => entry.split(/\s+/).join(" "));
}

/** The guide's differences, each by its number, one line of words. */
function differences(): Map<number, string> {
  const from = guide.indexOf(
    "\n## Differences from native Rust, in one list\n",
  );
  expect(from).toBeGreaterThan(0);
  const to = guide.indexOf("\n## ", from + 1);
  const list = guide.slice(from, to < 0 ? undefined : to);
  const found = new Map<number, string>();
  for (const item of list.split(/\n(?=\d+\. )/).slice(1)) {
    const number = Number(item.slice(0, item.indexOf(".")));
    found.set(number, item.split(/\s+/).join(" "));
  }
  return found;
}

/** The differences an entry names, `(`semantics.md`, 51)`. */
function named(entry: string): number[] {
  return [...entry.matchAll(/`semantics\.md`, (\d+)\)/g)].map((m) =>
    Number(m[1]),
  );
}

test("each exception ADR 0262's Consequences lists names a difference the guide has", () => {
  const listed = differences();
  const entries = consequences();
  expect(entries.length).toBeGreaterThan(20);
  const wrong: string[] = [];
  for (const entry of entries) {
    const title = entry.slice(0, 80);
    // A rare and harmless exception, the second case's, is one of the
    // guide's differences, by its number; one that's no case now, its
    // difference gone, is none.
    const gone = /is no case now/.test(entry);
    if (/second case/.test(entry) && !gone && named(entry).length === 0)
      wrong.push(`${title}: names no guide difference`);
    for (const number of named(entry)) {
      if (!listed.has(number))
        wrong.push(`${title}: the guide has no difference ${number}`);
    }
  }
  expect(wrong).toEqual([]);
});

test("an rd id in ADR 0262's Consequences is only the port's gap, never an exception's name", () => {
  const named: string[] = [];
  for (const entry of consequences()) {
    // Each `gap rdN` and `gaps rdN, rdM` taken away, no rd id is left.
    const left = entry.replace(/\bgaps? rd\d+\w*(?:, rd\d+\w*)*/g, "");
    for (const m of left.matchAll(/\brd\d+\w*/g))
      named.push(`${m[0]} in ${entry.slice(0, 80)}`);
  }
  expect(named).toEqual([]);
});

test("the metamorphic net's decided exceptions are named by their difference's number, which the guide has", () => {
  // `#difference 39`, never `#rd360`, under which test/metamorphic-known.txt
  // lists a rewrite whose difference the user decided.
  const known = readFileSync(join(root, "test/metamorphic-known.txt"), "utf8");
  const headings = known.split("\n").filter((line) => line.startsWith("#"));
  expect(headings.filter((line) => /^#rd\d/.test(line))).toEqual([]);
  const listed = differences();
  for (const line of headings) {
    const number = /^#difference (\d+) /.exec(line)?.[1];
    if (number !== undefined) expect(listed.has(Number(number)), line).toBe(true);
  }
  expect(headings.some((line) => line.startsWith("#difference "))).toBe(true);
});

test("ADR 0262's exceptions are named by their difference's number, not an rd id", () => {
  // An exception's own name, `difference 51`, is the guide's number of it;
  // one gone, `The guide's difference 37 is gone`, names none.
  const listed = differences();
  for (const entry of consequences()) {
    for (const m of entry.matchAll(/\bdifference (\d+)\b(?! is gone)/g)) {
      expect(named(entry), entry.slice(0, 80)).toContain(Number(m[1]));
      expect(listed.has(Number(m[1]))).toBe(true);
    }
  }
});
