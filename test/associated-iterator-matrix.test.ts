// The associated-iterator matrix (#2330, ADR 0061, amended): a trait's
// associated type bound by `Iterator`, given by an impl as each shape of
// iterator (a range, the crate's own iterator, std's JS iterator, std's
// compiled struct), met by each thing generic code does with its value:
// steps it, gives it back to concrete code that reads it as the impl's
// type, steps it and then gives it back, steps it from its back, and makes
// it by a `From<I>` bound of the type. Each built as a program and as a
// library and run as native Rust runs it: the premise that one value has
// one representation, its type's own, in generic code and concrete alike.
// A library's dictionary of one is `test/crates/pairs/
// library_associated_iterator_flows_back`. What gets it wrong is listed in
// `wrong`, keyed `<build>:<shape>` with its issue, so the list only shrinks.
//
//   bun test test/associated-iterator-matrix.test.ts

import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { agree, compileJs, runJs, runNative, runtimes, show } from "./programs";
import { buildCompiler, fixture } from "./support";

beforeAll(buildCompiler);

/** Each shape an impl gives its associated iterator as: the type, the
 * impl's value of it from `n`, and how concrete code reads that value as
 * its own type. */
const shapes: Record<string, { ty: string; make: string; read: string }> = {
  range: { ty: "std::ops::Range<u32>", make: "0..n", read: `format!("{:?} {}", it, it.start)` },
  own: { ty: "Countdown", make: "Countdown(n)", read: `format!("{:?} {}", it, it.0)` },
  vec: { ty: "std::vec::IntoIter<u32>", make: "(0..n).collect::<Vec<u32>>().into_iter()", read: `format!("{:?}", it.as_slice())` },
  compiled: { ty: "std::iter::Map<std::ops::Range<u32>, fn(u32) -> u32>", make: "(0..n).map(double as fn(u32) -> u32)", read: `format!("{:?}", it)` },
};

function program(shape: { ty: string; make: string; read: string }): string {
  return `#[derive(Debug, Clone)]
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|x| 10 + x)
    }
}

impl From<u32> for Countdown {
    fn from(n: u32) -> Self {
        Countdown(n)
    }
}

fn double(x: u32) -> u32 {
    x * 2
}

trait Source {
    type Out: DoubleEndedIterator<Item = u32>;
    fn make(&self, n: u32) -> Self::Out;
}

struct Made;

impl Source for Made {
    type Out = ${shape.ty};
    fn make(&self, n: u32) -> ${shape.ty} {
        ${shape.make}
    }
}

trait Built {
    type Out: From<u32> + Iterator<Item = u32>;
}

impl Built for Made {
    type Out = Countdown;
}

fn stepped<S: Source>(s: &S) -> Option<u32> {
    let mut it = s.make(3);
    it.next();
    it.next()
}

fn given_back<S: Source>(s: &S) -> S::Out {
    s.make(3)
}

fn stepped_then_given_back<S: Source>(s: &S) -> S::Out {
    let mut it = s.make(4);
    it.next();
    it
}

fn from_its_back<S: Source>(s: &S) -> Vec<u32> {
    let mut it = s.make(4);
    let last = it.next_back();
    let mut rest: Vec<u32> = it.rev().collect();
    rest.extend(last);
    rest
}

fn built<B: Built>(n: u32) -> Vec<u32> {
    B::Out::from(n).collect()
}

fn main() {
    println!("{:?}", stepped(&Made));
    let it = given_back(&Made);
    println!("{}", ${shape.read});
    println!("{:?}", it.collect::<Vec<_>>());
    let it = stepped_then_given_back(&Made);
    println!("{}", ${shape.read});
    println!("{:?}", from_its_back(&Made));
    println!("{:?}", built::<Made>(3));
}
`;
}

/** \`IntoIterator\`'s own associated iterator given back to concrete code. */
const intoIter = `struct Bag(Vec<u32>);

impl IntoIterator for Bag {
    type Item = u32;
    type IntoIter = std::ops::Range<u32>;
    fn into_iter(self) -> std::ops::Range<u32> {
        0..self.0.len() as u32
    }
}

fn given_back<T: IntoIterator>(t: T) -> T::IntoIter {
    t.into_iter()
}

fn main() {
    let r = given_back(3..6);
    println!("{:?} {}", r, r.start);
    let b = given_back(Bag(vec![1, 2]));
    println!("{:?} {}", b, b.end);
}
`;

const programs: Record<string, string> = {
  ...Object.fromEntries(Object.entries(shapes).map(([name, shape]) => [name, program(shape)])),
  "into-iter": intoIter,
};

const builds: [string, boolean][] = [
  ["program", false],
  ["library", true],
];

/** The programs that run and differ from native Rust, each a wrong answer
 * filed P0, by its issue. One that agrees is taken off with its fix. */
const wrong: Record<string, string> = {
  "program:into-iter": "#2331",
  "library:into-iter": "#2331",
};

test("what `wrong` lists is the matrix's", () => {
  for (const key of Object.keys(wrong)) {
    const [build, name] = key.split(":");
    expect(builds.map(([b]) => b)).toContain(build);
    expect(Object.keys(programs)).toContain(name);
  }
});

for (const [name, source] of Object.entries(programs)) {
  test(`an associated iterator: ${name}`, () => {
    const dir = fixture(`associated-iterator-${name}`);
    const file = join(dir, "main.rs");
    writeFileSync(file, source);
    const native = runNative(file, dir);
    if (typeof native === "string") throw new Error(native);
    const problems: string[] = [];
    for (const [build, library] of builds) {
      const key = `${build}:${name}`;
      const compiled = compileJs(file, dir, "2024", library);
      let problem = "";
      if (!("js" in compiled)) {
        problem = `refused: ${compiled.error.split("\n").slice(0, 4).join("\n")}`;
      } else {
        const [runtime, cmd] = runtimes[0];
        const ran = runJs(cmd, compiled.js, dir, `${runtime}-${build}`);
        if (!agree(ran, native)) problem = `${show(ran.outcome)}: native ${JSON.stringify(native.stdout)}, JS ${JSON.stringify(ran.stdout)}`;
      }
      if (key in wrong) {
        if (problem === "") problems.push(`${key} runs as Rust does now: take it off \`wrong\`, ${wrong[key]}`);
      } else if (problem !== "") problems.push(`${key}: ${problem}`);
    }
    expect(problems).toEqual([]);
  }, 300_000);
}
