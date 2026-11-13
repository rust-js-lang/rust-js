// A `write!` into a writer of the crate's own, or a `W: fmt::Write`'s, a
// piece at a time (#1652, ADR 1135): test/writer-sink.rs, whose writers
// record each call they're given, run natively and as JS in both builds,
// where `RUST_JS_WRITES_PIECEWISE=1` writes it so. The switch goes once
// every value's split is `display::sink_writes`', and this case joins the
// corpus then. And the runtime's sink, `$writerSink` and std's `pad` and
// `pad_integral` into it, given each option, call by call.
//
//   bun test test/writer-sink.test.ts

import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { helpersModule } from "./packages-of";
import { compileJs, runJs, runNative, runtimes, show } from "./programs";
import { buildCompiler, fixture, root } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildCompiler);

const piecewise = { RUST_JS_WRITES_PIECEWISE: "1" };

/** The two builds: the default, and std's iterator structs (ADR 0908). */
const builds: [string, string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];

for (const [build, flags] of builds) {
  test(`each write is native Rust's, in the ${build} build`, () => {
    const dir = fixture(`writer-sink-${build}`);
    const file = join(root, "test/writer-sink.rs");
    const native = runNative(file, dir, "2021");
    if (typeof native === "string") throw new Error(native);
    const compiled = compileJs(file, dir, "2021", false, piecewise, flags);
    if (!("js" in compiled)) throw new Error(compiled.error);
    const [, runtime] = runtimes[0];
    const js = runJs(runtime, compiled.js, dir, "js");
    expect(js.stdout).toBe(native.stdout);
    expect(show(js.outcome)).toBe(show(native.outcome));
  }, underLoad(120_000));
}

/** A writer shown into a writer whose split isn't followed yet: each `fmt`
 * of `Shown`, written into `Log`, is refused, naming #1652. */
const refusedWriters: [string, string, string][] = [
  ["a builder's dyn Debug", "dyn", 'let d: &dyn fmt::Debug = &1;\n        f.debug_list().entry(d).finish()'],
  ["a write through a held Formatter", "held where it isn't", 'let held = Held(f);\n        let n = [-1, 2];\n        write!(held.0, "{}", n[0])'],
  ["a closure given the Formatter", "Formatter", "let w = |f: &mut fmt::Formatter<'_>| f.write_str(\"x\");\n        w(f)"],
  ["a HashMap's Debug", "by its `Debug`", 'let m = std::collections::HashMap::from([(1, 2)]);\n        write!(f, "{:?}", m)'],
  ["a write_fmt of Arguments made elsewhere", "made elsewhere", 'f.write_fmt(given(format_args!("{}", 1)))'],
];

test("a writer shown into a writer whose split isn't followed is refused, naming #1652", () => {
  const dir = fixture("writer-sink-refused");
  for (const [name, said, body] of refusedWriters) {
    const file = join(dir, `${name.replaceAll(" ", "_")}.rs`);
    writeFileSync(
      file,
      `use std::fmt::{self, Write};
struct Log(Vec<String>);
impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}
#[allow(dead_code)]
struct Held<'a, 'b>(&'a mut fmt::Formatter<'b>);
#[allow(dead_code)]
fn given(args: fmt::Arguments<'_>) -> fmt::Arguments<'_> {
    args
}
struct Shown;
impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ${body}
    }
}
fn main() {
    let mut log = Log(Vec::new());
    write!(log, "{}", Shown).unwrap();
    println!("{:?}", log.0);
}
`,
    );
    const compiled = compileJs(file, dir, "2021", false, piecewise);
    if ("js" in compiled) throw new Error(`${name}: compiled`);
    expect(compiled.error, name).toContain("#1652");
    expect(compiled.error, name).toContain(said);
  }
}, underLoad(120_000));

/** The runtime's sink helpers, of `@rust-js/std`, and a sink recording
 * each call: `s` a `write_str`'s, `c` a `write_char`'s. */
async function recorded(write: (f: { write(s: string): void; writeChar(c: string): void }, std: Record<string, any>) => void) {
  const std: Record<string, any> = await import(helpersModule);
  const calls: string[] = [];
  const f = std.$writerSink(calls, (w: string[], s: string) => w.push(`s${s}`), (w: string[], c: string) => w.push(`c${c}`));
  write(f, std);
  return calls;
}

test("pad_integral writes its sign and each fill char apart, as std's", async () => {
  const cases: [boolean, string, string, object | undefined, string[]][] = [
    [false, "", "5", undefined, ["c-", "s5"]],
    [true, "", "5", { plus: true }, ["c+", "s5"]],
    [true, "", "5", { width: 3 }, ["c ", "c ", "s5"]],
    [false, "", "5", { width: 4, zero: true }, ["c-", "c0", "c0", "s5"]],
    [true, "0x", "ff", { alternate: true, width: 7, align: "Left", fill: "*" }, ["s0x", "sff", "c*", "c*", "c*"]],
    [true, "", "7", { alternate: true }, ["s", "s7"]],
    [true, "0b", "1", { alternate: true, width: 6, align: "Center" }, ["c ", "s0b", "s1", "c ", "c "]],
    [true, "", "123", { width: 2 }, ["s123"]],
  ];
  for (const [nonNegative, prefix, digits, options, want] of cases) {
    expect(await recorded((f, std) => std.$writeIntegral(f, nonNegative, prefix, digits, options))).toEqual(want);
  }
});

test("pad cuts to its precision in chars and fills each side by a char at a time", async () => {
  const cases: [string, object, string[]][] = [
    ["héllo", {}, ["shéllo"]],
    ["héllo", { precision: 2 }, ["shé"]],
    ["ab", { width: 4 }, ["sab", "c ", "c "]],
    ["ab", { width: 5, align: "Center", fill: "é" }, ["cé", "sab", "cé", "cé"]],
    ["ab", { width: 3, align: "Right" }, ["c ", "sab"]],
    ["abc", { width: 6, precision: 1, align: "Right" }, ["c ", "c ", "c ", "c ", "c ", "sa"]],
    ["", { precision: 0 }, ["s"]],
  ];
  for (const [text, options, want] of cases) {
    expect(await recorded((f, std) => std.$writePadded(f, text, options))).toEqual(want);
  }
});

test("an integer's Display writes its sign apart, of a number or a BigInt", async () => {
  expect(await recorded((f, std) => std.$writeInteger(f, -2147483648))).toEqual(["c-", "s2147483648"]);
  expect(await recorded((f, std) => std.$writeInteger(f, -5n, { width: 4 }))).toEqual(["c ", "c ", "c-", "s5"]);
  expect(await recorded((f, std) => std.$writeInteger(f, 0))).toEqual(["s0"]);
});

test("an integer's Debug by a writer's options is its hex where {:x?} asks", async () => {
  const debug = (options?: object) => recorded((f, std) => std.$writeDebugInteger(f, -2, "fe", "FE", options));
  expect(await debug()).toEqual(["c-", "s2"]);
  expect(await debug({ debugHex: "x", width: 4 })).toEqual(["c ", "c ", "sfe"]);
  expect(await debug({ debugHex: "X", alternate: true })).toEqual(["s0x", "sFE"]);
});

test("a float is written by flt2dec's parts, as pad_formatted_parts writes them", async () => {
  const cases: [number, string, boolean, object | undefined, string[]][] = [
    [-1.5, "D", false, undefined, ["s-", "s1", "s.", "s5"]],
    [100, "D", false, undefined, ["s1", "s00"]],
    [100, "G", false, undefined, ["s1", "s00", "s.", "s0"]],
    [0.25, "D", false, { width: 7, zero: true }, ["s", "c0", "c0", "c0", "s0.", "s25"]],
    [-0.25, "D", false, { width: 7, align: "Center" }, ["c ", "s-", "s0.", "s25", "c "]],
    [Number.NaN, "D", false, { plus: true }, ["sNaN"]],
    [1e100, "D", false, undefined, ["s1", `s${"0".repeat(64)}`, `s${"0".repeat(36)}`]],
    [1e16, "D", false, { precision: 30 }, ["s10000000000000000", "s.", "s0000", `s${"0".repeat(26)}`]],
    [100, "e", false, undefined, ["s1", "se", "s2"]],
    [0.25, "E", false, { precision: 0 }, ["s2", "sE-", "s1"]],
    [0, "e", false, { precision: 3 }, ["s0.", "s000", "se0"]],
    [1e-7, "G", false, undefined, ["s1", "se-", "s7"]],
    [Math.fround(0.1), "D", true, { precision: 60 }, ["s0.", "s100000001490116119384765625000000000000000", `s${"0".repeat(18)}`]],
  ];
  for (const [x, by, f32, options, want] of cases) {
    expect(await recorded((f, std) => std.$writeFloat(f, x, by, f32, options))).toEqual(want);
  }
});

test("std's PadAdapter writes each line apart, its indent a write of its own, nothing of an empty one", async () => {
  expect(
    await recorded((f, std) => {
      const g = std.$padAdapter(f);
      g.write("a\nb");
      g.write("");
      g.writeChar("\n");
      g.writeChar("c");
      g.write("\n\n");
    }),
  ).toEqual(["s    ", "sa\n", "s    ", "sb", "c\n", "s    ", "cc", "s\n", "s    ", "s\n"]);
});

test("a builder's step of a writing function writes each piece, pretty ones through the adapter", async () => {
  const int = (g: any, n: number) => g.write(String(n));
  expect(
    await recorded((f, std) => {
      const s = std.$debugStruct(f, "P", false);
      s.fieldWith("x", 1, int);
      s.fieldWith("y", 2, int);
      s.finish();
    }),
  ).toEqual(["sP", "s { ", "sx", "s: ", "s1", "s, ", "sy", "s: ", "s2", "s }"]);
  expect(
    await recorded((f, std) => {
      const t = std.$debugTuple(f, "", true);
      t.fieldWith(1, int);
      t.finish();
    }),
  ).toEqual(["s", "s(\n", "s    ", "s1", "s,\n", "s)"]);
  expect(await recorded((f, std) => std.$writeDebugList(f, [1, 2], int, false))).toEqual(["s[", "s1", "s, ", "s2", "s]"]);
  expect(await recorded((f, std) => std.$writeDebugOption(f, undefined, int, false))).toEqual(["sNone"]);
});

test("a str's and a char's Debug write each escape apart", async () => {
  expect(await recorded((f, std) => std.$writeDebugStr(f, 'a"b\\'))).toEqual(['c"', "sa", 's\\"', "sb", "s\\\\", "s", 'c"']);
  expect(await recorded((f, std) => std.$writeDebugStr(f, ""))).toEqual(['c"', "s", 'c"']);
  expect(await recorded((f, std) => std.$writeDebugChar(f, "é"))).toEqual(["c'", "cé", "c'"]);
  expect(await recorded((f, std) => std.$writeDebugChar(f, "\n"))).toEqual(["c'", "s\\n", "c'"]);
});

test("a char's Display by a writer's options is its write_char, or pad given a width", async () => {
  expect(await recorded((f, std) => std.$writeCharOf(f, "é"))).toEqual(["cé"]);
  expect(await recorded((f, std) => std.$writeCharOf(f, "é", { plus: true }))).toEqual(["cé"]);
  expect(await recorded((f, std) => std.$writeCharOf(f, "é", { width: 2 }))).toEqual(["sé", "c "]);
});

test("a writer shown into a writer writes through the sink its first $sink() claims, and no other", async () => {
  const std: Record<string, any> = await import(helpersModule);
  const calls: string[] = [];
  const f = std.$writerSink(calls, (w: string[], s: string) => w.push(`s${s}`), (w: string[], c: string) => w.push(`c${c}`));
  // A writer that writes, then makes a string of its own with another.
  const inner = () => {
    const g = std.$sink();
    g.write("in");
    return g.done();
  };
  const writer = (x: string) => {
    const g = std.$sink();
    g.write(x);
    g.writeChar("!");
    g.write(`[${inner()}]`);
    return g.done();
  };
  std.$writingInto(f, writer, "a");
  expect(calls).toEqual(["sa", "c!", "s[in]"]);
  // A writer that never makes its sink is rust-js's error, not its text whole.
  expect(() => std.$writingInto(f, () => "text")).toThrow("didn't write through its sink");
  // Nothing armed after: a sink made now is a string of its own.
  const after = std.$sink();
  after.write("x");
  expect(after.done()).toBe("x");
});
