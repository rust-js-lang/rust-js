// What a person wouldn't write, in generated JS (ADR 0565): each shape, by
// name, as a test of one line of JS. The one list: the readability gate
// counts them over the JS the repository commits (`test/readability.test.ts`,
// `docs/readability.txt`), and `bun run port measure` over what a compile
// of the react.dev port adds.
import { readFileSync } from "node:fs";

export const SHAPES: [string, RegExp][] = [
  ["temporaries", /\b(const|let) _\d+\b/],
  ["labeled blocks", /^\s*bb\d+:/],
  ["while (true)", /^\s*(bb\d+: )?while \(true\)/],
  ["$iter", /\$iter\(/],
  ["Iterator.from", /Iterator\.from/],
  ["return;", /^\s*return;$/],
  ["renamed x$1", /\b[A-Za-z_]\w*\$\d+\b/],
];

/** How many of `lines` have each shape, and how many lines there are. */
export function measure(lines: Iterable<string>): { lines: number; counts: Map<string, number> } {
  const counts = new Map(SHAPES.map(([name]) => [name, 0]));
  let total = 0;
  for (const line of lines) {
    total++;
    for (const [name, shape] of SHAPES) if (shape.test(line)) counts.set(name, counts.get(name)! + 1);
  }
  return { lines: total, counts };
}

/** `measure` of the files, each line of each. */
export function measureFiles(files: string[]): { lines: number; counts: Map<string, number> } {
  return measure(files.flatMap((file) => readFileSync(file, "utf8").split("\n")));
}

/** One file's measure: its lines, and how many of them have each shape. */
export type Measured = { lines: number; counts: Map<string, number> };

/** A share: how many in ten thousand lines, to a tenth, so one more of a
 * shape in a corpus this size shows. */
export function perTenThousand(count: number, lines: number): number {
  return Math.round((count * 100_000) / lines) / 10;
}

/** The gate's file: each shape's count and share over all `files`, by
 * their path from the repository, `measured`; then each file's lines and
 * counts, which the gate compares like for like (`compared`). */
export function report(measured: Map<string, Measured>): string {
  const lines = [...measured.values()].reduce((sum, m) => sum + m.lines, 0);
  const rows = SHAPES.map(([name]) => {
    const count = [...measured.values()].reduce((sum, m) => sum + m.counts.get(name)!, 0);
    return `${name}\t${count}\t${perTenThousand(count, lines)}`;
  });
  const files = [...measured].sort(([a], [b]) => (a < b ? -1 : 1)).map(
    ([file, m]) => `${file}\t${m.lines}\t${SHAPES.map(([name]) => m.counts.get(name)).join("\t")}`,
  );
  return [
    `# shape\tcount\tper 10,000 lines`,
    `files\t${measured.size}`,
    `lines\t${lines}`,
    ...rows,
    `# file\tlines\t${SHAPES.map(([name]) => name).join("\t")}`,
    ...files,
    "",
  ].join("\n");
}

/** `report`'s file, read: each file's measure. */
export function measuredOf(text: string): Map<string, Measured> {
  const files = text.split("# file\t")[1]?.split("\n").slice(1) ?? [];
  return new Map(
    files
      .filter((line) => line)
      .map((line) => {
        const [file, lines, ...counts] = line.split("\t");
        return [file, { lines: Number(lines), counts: new Map(SHAPES.map(([name], i) => [name, Number(counts[i])])) }];
      }),
  );
}

/** Each shape's share before and now, over the files both have, like for
 * like: a case added or removed moves none of them (ADR 0565, amended).
 * And the files only `now` has. */
export function compared(
  before: Map<string, Measured>,
  now: Map<string, Measured>,
): { before: Map<string, number>; now: Map<string, number>; added: string[] } {
  const shared = [...now.keys()].filter((file) => before.has(file));
  const share = (of: Map<string, Measured>) => {
    const lines = shared.reduce((sum, file) => sum + of.get(file)!.lines, 0);
    return new Map(
      SHAPES.map(([name]) => [
        name,
        perTenThousand(shared.reduce((sum, file) => sum + of.get(file)!.counts.get(name)!, 0), Math.max(lines, 1)),
      ]),
    );
  };
  return { before: share(before), now: share(now), added: [...now.keys()].filter((file) => !before.has(file)) };
}
