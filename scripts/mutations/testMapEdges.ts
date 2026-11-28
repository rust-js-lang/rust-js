// The test map's edges read into the map, every crate's or none, and
// asked by the selector (scripts/testMapEdges.ts, ADR 1210, #2608).
import type { Mutation } from "../mutations";

const reader = "scripts/testMapEdges.ts";
const read = ["test/test-map-edges.test.ts", "-t", "a crate's record is read where its sources are the tree's"];

export const mutations: Mutation[] = [
  {
    name: "edges-stale-record-read",
    breaks: "a crate's record of sources since changed is read, its lines and edges another tree's",
    file: reader,
    find: "      if (bytes === undefined || createHash(kind).update(bytes).digest(\"hex\") !== value) return",
    replace: "      if (bytes === undefined) return",
    tests: read,
  },
  {
    name: "edges-failed-walk-read",
    breaks: "a crate whose walk failed is taken for one with no record, never said why",
    file: reader,
    find: "    if (existsSync(join(dir, `${crate}.failed`))) return `the walk of ${crate} failed`;\n",
    replace: "",
    tests: read,
  },
  {
    name: "edges-closure-not-its-functions",
    breaks: "a function's definition holds only the map's function that starts with it, not its closures', so a change a closure reads reaches no test that ran it",
    file: reader,
    find: "      if (start <= from && to <= end && (inner === undefined || start >= defs[inner]!.lines[0])) inner = at;\n",
    replace: "      if (start === from && (inner === undefined || start >= defs[inner]!.lines[0])) inner = at;\n",
    tests: read,
  },
  {
    name: "edges-walk-past-functions",
    breaks: "the walk back along the edges goes on past each function it reaches, to its callers, whose tests already run it, so a constant's change selects every caller's",
    file: reader,
    find: "      if (at !== own && fns.length > 0) continue;\n",
    replace: "",
    tests: ["test/test-map-edges.test.ts", "-t", "by the edges"],
  },
  {
    name: "edges-innermost-outermost",
    breaks: "a line is placed in the outermost definition holding it, not the innermost, so a field's change is its struct's, and selects what names the struct alone",
    file: reader,
    find: "      if (start <= from && to <= end && (inner === undefined || end - start < defs[inner]!.lines[1] - defs[inner]!.lines[0])) inner = at;\n",
    replace: "      if (start <= from && to <= end && (inner === undefined || end - start > defs[inner]!.lines[1] - defs[inner]!.lines[0])) inner = at;\n",
    tests: ["test/test-map-edges.test.ts", "-t", "by the edges"],
  },
  {
    name: "edges-names-unread",
    breaks: "the names of what another crate's a definition names are left out of the map's names, so a new item of the prelude's name reaches none of what names the prelude's",
    file: reader,
    find: "    for (const [at, name] of Object.entries(one.names)) nameOf.set(at, name);\n",
    replace: "",
    tests: read,
  },
];
