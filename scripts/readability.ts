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

/** A shape's share: how many in ten thousand lines, to a tenth, so one
 * more of a shape in a corpus this size shows. */
export function perTenThousand(count: number, lines: number): number {
  return Math.round((count * 100_000) / lines) / 10;
}

/** The gate's file: each shape, its count and its share, after the lines
 * and files counted. */
export function report(files: number, measured: { lines: number; counts: Map<string, number> }): string {
  const rows = [...measured.counts].map(
    ([name, count]) => `${name}\t${count}\t${perTenThousand(count, measured.lines)}`,
  );
  return [`# shape\tcount\tper 10,000 lines`, `files\t${files}`, `lines\t${measured.lines}`, ...rows, ""].join("\n");
}

/** `report`'s file, read: each shape's share. */
export function shares(text: string): Map<string, number> {
  return new Map(
    text
      .split("\n")
      .filter((line) => line && !line.startsWith("#") && !/^(files|lines)\t/.test(line))
      .map((line) => {
        const [name, , share] = line.split("\t");
        return [name, Number(share)];
      }),
  );
}
