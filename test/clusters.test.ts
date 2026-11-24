// The kind × area count (scripts/clusters.ts, ADR 1085): each cell's open
// and closed issues, all time and over a window, a cell of 3 or more a
// cluster with the refactor docs/bug-kinds.md's table names for its kind,
// read from that table, and the one comment the nightly keeps on the
// campaign.

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { clusterLine, clusterSize, comment, count, type Issue, isCluster, kindsTable, marker, table, trackers } from "../scripts/clusters";
import { root } from "./support";

const issue = (number: number, kind: string | undefined, area: string | undefined, state = "OPEN", createdAt = "2026-11-15T00:00:00Z", login = "nguyenyou"): Issue => ({
  number,
  state,
  createdAt,
  author: { login },
  labels: [...(kind ? [{ name: `kind: ${kind}` }] : []), ...(area ? [{ name: `area: ${area}` }] : []), { name: "P0: wrong answer" }],
});

test("each kind × area cell counts its open and closed issues, all time and over the window, by the trusted authors alone", () => {
  const cells = count(
    [
      issue(1, "protocol gap", "iterators"),
      issue(2, "protocol gap", "iterators", "CLOSED"),
      issue(3, "protocol gap", "iterators", "CLOSED", "2026-10-01T00:00:00Z"),
      issue(4, "missing case", "mut-refs", "OPEN", "2026-11-14T00:00:00Z", "github-actions"),
      issue(5, "missing case", "mut-refs", "OPEN", "2026-11-14T00:00:00Z", "someone-else"),
      issue(6, undefined, "mut-refs"),
      issue(7, "missing case", undefined),
    ],
    new Date("2026-11-02T00:00:00Z"),
  );
  expect(cells).toEqual([
    { kind: "kind: protocol gap", area: "area: iterators", open: [1], closed: [2, 3], recentOpen: [1], recentClosed: [2] },
    { kind: "kind: missing case", area: "area: mut-refs", open: [4], closed: [], recentOpen: [4], recentClosed: [] },
  ]);
  expect(clusterSize).toBe(3);
  expect([isCluster(cells[0]!), isCluster(cells[0]!, true), isCluster(cells[1]!)]).toEqual([true, false, false]);
});

test("a cluster calls for what docs/bug-kinds.md's table says its kind calls for, read from the table", () => {
  const kinds = kindsTable(readFileSync(join(root, "docs/bug-kinds.md"), "utf8"));
  expect(kinds.size).toBeGreaterThanOrEqual(11);
  expect(kinds.get("kind: protocol gap")).toContain("conformance matrix");
  // A model of std's own code, missing what its body does (ADR 1085, amended).
  expect(kinds.get("kind: semantic shortcut")).toBe("std's own body, or a refusal; never a model of it");
  const page = ["| # | Label | A cluster calls for |", "|---|---|---|", "| 1 | `kind: a` | One owner |", "| 2 | `kind: b` | A matrix |", "", "| x | y |"].join("\n");
  expect([...kindsTable(page)]).toEqual([
    ["kind: a", "One owner"],
    ["kind: b", "A matrix"],
  ]);
  expect(() => kindsTable("no table")).toThrow("table of kinds");
  const cells = count([issue(1, "a", "x"), issue(2, "a", "x"), issue(3, "a", "x"), issue(4, "b", "x")], new Date(0));
  const said = table(cells, kindsTable(page), 14);
  expect(said).toContain("| area: x | kind: a | 3 (3 open) | 3 (3 open) | cluster | One owner |");
  expect(said).toContain("| area: x | kind: b | 1 (1 open) | 1 (1 open) |  |  |");
});

test("the campaign's comment lists each open cluster, its issues, and its tracking refactor or that there's none", () => {
  const cells = count([issue(1, "a", "x"), issue(2, "a", "x", "CLOSED"), issue(3, "a", "x"), issue(4, "b", "y"), issue(5, "b", "y", "CLOSED"), issue(6, "b", "y", "CLOSED"), issue(7, "c", "z", "CLOSED"), issue(8, "c", "z", "CLOSED"), issue(9, "c", "z", "CLOSED")], new Date(0));
  expect(clusterLine(cells[0]!)).toBe("cluster: a × x");
  const tracked = trackers(cells, [{ number: 40, body: "The refactor.\n\nCluster: A × X\n" }]);
  const said = comment(cells, tracked, 14, "2026-11-16");
  expect(said.startsWith(marker)).toBe(true);
  expect(said).toContain("- **area: x × kind: a**: 2 open, 1 closed (#1 #3); refactor tracked in #40");
  expect(said).toContain("- **area: y × kind: b**: 1 open, 2 closed (#4); no tracking refactor (an open issue with a `cluster: b × y` line)");
  // A cluster all closed isn't open.
  expect(said).not.toContain("kind: c");
});

test("the nightly keeps the clusters' comment on the campaign, and the periodic count names the script", () => {
  const nightly = readFileSync(join(root, ".github/workflows/nightly.yml"), "utf8");
  expect(nightly).toContain("bun scripts/clusters.ts --comment=1681");
  expect(readFileSync(join(root, "docs/bug-kinds.md"), "utf8")).toContain("bun scripts/clusters.ts");
});
