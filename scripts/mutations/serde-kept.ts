// Mutations of serde's snapshots kept for the tests (test/serde-kept.ts): a sweep that skips what another run removed,
// and one that takes a snapshot an hour unasked, never one a live process reads (#2483).
import type { Mutation } from "../mutations";

const file = "test/serde-kept.ts";
const swept = ["test/serde-kept.test.ts", "-t", "a snapshot an hour unasked goes, but not one a live process reads"];
const marked = ["test/serde-kept.test.ts", "-t", "a run marks the snapshot it asks for read"];

export const mutations: Mutation[] = [
  {
    name: "serde-kept-sweep-throws-on-gone",
    breaks: "serde's snapshot store's sweep throws on an entry another run's sweep removed since it was listed: crates.test.ts's ENOENT (2026-11-17)",
    file,
    find: "    const seen = statSync(at, { throwIfNoEntry: false });\n",
    replace: "    const seen = statSync(at);\n",
    tests: ["test/support.test.ts", "-t", "a store's sweep skips an entry gone since it was listed"],
  },
  {
    name: "serde-snapshot-kept-a-day",
    breaks: "a snapshot no run reads is kept a day after it was last asked for, as before #2483: 62 of them, 4.1 GiB, in the M5 Pro's main checkout",
    file,
    find: "export const keptFor = 3600 * 1000;",
    replace: "export const keptFor = 24 * 3600 * 1000;",
    tests: swept,
  },
  {
    name: "serde-snapshot-read-by-the-living-swept",
    breaks: "a snapshot a live process asked for, and whose files its compiles read, is swept an hour after it was asked for: rustc's \"can't find crate for `serde`\" (#1711)",
    file,
    find: "    if (alive(Number(mark[3]))) read.add(mark[1]);\n",
    replace: "    if (alive(Number(mark[3])) && false) read.add(mark[1]);\n",
    tests: marked,
  },
  {
    name: "serde-snapshot-read-by-the-gone-kept",
    breaks: "a snapshot whose only reader's process is gone is kept as if it read it, so every one any process ever asked for is kept for good",
    file,
    find: "    if (alive(Number(mark[3]))) read.add(mark[1]);\n    else rmSync(join(store, name), { force: true });\n",
    replace: "    read.add(mark[1]);\n",
    tests: swept,
  },
  {
    name: "serde-snapshot-asked-unmarked",
    breaks: "the process that asks for a snapshot doesn't mark it read, so another run's sweep takes it from under its compiles an hour after it was asked for",
    file,
    find: '  writeFileSync(readerOf(dir), "");\n',
    replace: "",
    tests: marked,
  },
  {
    name: "serde-snapshot-half-made-kept",
    breaks: "a snapshot a process gone left half made is kept for good",
    file,
    find: '    if (mark?.[2] === "making" && !alive(Number(mark[3]))) rmSync(join(store, name), { recursive: true, force: true });\n',
    replace: "",
    tests: swept,
  },
];
