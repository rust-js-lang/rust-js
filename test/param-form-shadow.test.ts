// `ParamForm`'s shadow (#2539, Campaign 6 step 0, ADR 1203): of each
// value converted where it crosses into or out of generic code, the form
// the compile takes beside every form generic code's view differs in, the
// plan; of each answer of whether generic code holds an iterator as it is,
// the bounds of it that read its value. No output changes (ADR 1199's
// `param-form` kind).
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildCompiler, compiler, fixture, shadowLines } from "./support";

beforeAll(buildCompiler, 600_000);

/** Of `source` compiled as a binary, with and without the shadow: its
 * lines, each its fields after the kind, and whether the JS is the same. */
function shadowOf(source: string): { lines: string[][]; same: boolean } {
  const dir = fixture("param-form-shadow");
  const input = join(dir, "case.rs");
  const log = join(dir, "shadow.txt");
  writeFileSync(input, source);
  writeFileSync(log, "");
  // Each to the same file, whose name its source map names.
  const compile = (shadow: string[]) => {
    const p = runSync([compiler, ...shadow, input, "-o", join(dir, "case.js"), "--", "--edition=2024", "--crate-type=bin"], dir, 60_000);
    expect(p.code, p.stderr).toBe(0);
    return readFileSync(join(dir, "case.js"), "utf8");
  };
  const shadowed = compile(["--shadow", `param-form=${log}`]);
  const plain = compile([]);
  return { lines: shadowLines(log, "param-form").map((l) => l.split("\t")), same: shadowed === plain };
}

const program = `struct P {
    x: i32,
}

fn put<T>(slot: &mut T, v: T) {
    *slot = v;
}

fn firsts<I: Iterator + Clone>(i: I) -> (Option<I::Item>, Option<I::Item>) {
    let mut a = i.clone();
    let mut b = i;
    (a.next(), b.next())
}

fn total<I: Iterator<Item = u32>>(i: I) -> u32 {
    i.sum()
}

fn main() {
    let mut p = P { x: 1 };
    put(&mut p, P { x: 2 });
    println!("{} {:?} {}", p.x, firsts(0..3), total(1..4));
}
`;

// A `&mut` object given where generic code boxes its `&mut T`: the one form
// that differs, taken, `agrees`; the JS is the same with the shadow or not.
test("a box given where it's the one form that differs agrees", () => {
  const { lines, same } = shadowOf(program);
  expect(same).toBe(true);
  const sites = lines.filter((l) => l[0] === "site").map((l) => l.slice(1, 6).join(" "));
  expect(sites).toContain("given agrees box box put");
});

// An iterator another bound reads, `I: Clone`'s, held as it is, and one
// nothing reads, a JS iterator: each answer beside the bounds that read it.
test("each iterator's form is written beside the bounds that read its value", () => {
  const { lines } = shadowOf(program);
  const iterated = lines.filter((l) => l[0] === "iterated").map((l) => [l[1], l[2], l[3], l[4], l[5]].join(" "));
  expect(iterated).toContain("given agrees as-it-is std::clone::Clone firsts");
  expect(iterated).toContain("given agrees js-iterator - total");
  expect(iterated.filter((l) => l.split(" ")[1] === "drifts")).toEqual([]);
});
