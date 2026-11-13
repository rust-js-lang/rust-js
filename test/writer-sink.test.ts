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
