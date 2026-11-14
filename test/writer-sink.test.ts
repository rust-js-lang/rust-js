// A `write!` into a writer of the crate's own, or a `W: fmt::Write`'s, a
// piece at a time (#1652, ADR 1135), what the corpus's writer_sink_* cases
// don't hold: what's refused, what writes through a sink, the runtime's
// sink, `$writerSink` and std's `pad` and `pad_integral` into it, given
// each option, call by call, and pairs of a library and its consumer.
//
//   bun test test/writer-sink.test.ts

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { chain, contract, counting } from "./crates";
import { helpersModule } from "./packages-of";
import { compileJs, runJs, runNative, runtimes, show } from "./programs";
import { buildCompiler, fixture, root } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildCompiler);



/** A writer shown into a writer whose split isn't followed: each `fmt` of
 * `Shown`, written into `Log`, is refused, naming #1652. */
const refusedWriters: [string, string, string][] = [
  ["a pointer's fmt given the Formatter", "or the like of a `Shown`", "fmt::Pointer::fmt(&self, f)"],
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
    const compiled = compileJs(file, dir, "2021", false);
    if ("js" in compiled) throw new Error(`${name}: compiled`);
    expect(compiled.error, name).toContain("which isn't followed for it (#1652)");
    expect(compiled.error, name).not.toContain("(#1652) yet");
    expect(compiled.error, name).toContain(said);
  }
}, underLoad(120_000));

// Only a writer shown into a writer writes through a sink: one the same
// body only prints makes its text as before, `let f = ""`, so its JS
// reads as it did (ADR 1135, step 2).
test("a writer a body that writes into a writer only prints doesn't write through a sink", () => {
  const dir = fixture("writer-sink-printed-only");
  const file = join(dir, "printed.rs");
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
struct Printed(i32);
impl fmt::Display for Printed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}
fn main() {
    let mut log = Log(Vec::new());
    let n = 1;
    write!(log, "{}", n).unwrap();
    println!("{} {:?}", Printed(2), log.0);
}
`,
  );
  const compiled = compileJs(file, dir, "2021", false);
  if (!("js" in compiled)) throw new Error(compiled.error);
  const js = readFileSync(compiled.js, "utf8");
  const writer = js.slice(js.indexOf("function printedDisplay_fmt"));
  expect(writer.indexOf("\n}")).toBeGreaterThan(0);
  expect(writer.slice(0, writer.indexOf("\n}"))).not.toContain("$sink()");
}, underLoad(120_000));

// A writer that writes through a sink as it prints (#1490), never shown
// into a writer, writes its `write!` as it did, its text made whole, not
// std's pieces (ADR 1135, step 2).
test("a writer that prints, never shown into a writer, writes its text whole", () => {
  const dir = fixture("writer-sink-printing-only");
  const file = join(dir, "printing.rs");
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
struct Noisy(i32);
impl fmt::Display for Noisy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        println!("inside");
        write!(f, "<{}>", self.0)
    }
}
fn main() {
    let mut log = Log(Vec::new());
    let n = 1;
    write!(log, "{}", n).unwrap();
    println!("{} {:?}", Noisy(2), log.0);
}
`,
  );
  const compiled = compileJs(file, dir, "2021", false);
  if (!("js" in compiled)) throw new Error(compiled.error);
  const js = readFileSync(compiled.js, "utf8");
  const writer = js.slice(js.indexOf("function noisyDisplay_fmt"));
  expect(writer.indexOf("\n}")).toBeGreaterThan(0);
  const body = writer.slice(0, writer.indexOf("\n}"));
  expect(body).toContain("$sink()");
  expect(body).not.toContain("$writeInteger");
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

test("an integer's {:e} is exp_u64's text by pad_integral, or its parts where a precision adds zeros", async () => {
  const cases: [number | bigint, boolean, object | undefined, string[]][] = [
    [-1500, false, undefined, ["c-", "s1.5e3"]],
    [120, true, undefined, ["s1.2E2"]],
    [0, false, undefined, ["s0e0"]],
    [2500, false, { precision: 0 }, ["s2e3"]],
    [3500, false, { precision: 0 }, ["s4e3"]],
    [9_995_000_000n, false, { precision: 2 }, ["s1.00e10"]],
    [120, false, { precision: 4 }, ["s1.20", "s00", "se2"]],
    [-15, false, { width: 8, zero: true, precision: 2 }, ["s-", "c0", "s1.5", "s0", "se1"]],
    [7, false, { width: 5 }, ["c ", "c ", "s7e0"]],
  ];
  for (const [n, upper, options, want] of cases) {
    expect(await recorded((f, std) => std.$writeIntExp(f, n, upper, options))).toEqual(want);
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

/** Pairs of crates, a library and its consumer, whose writes into a writer
 * are followed a piece at a time, which test/crates.test.ts's pairs, each
 * compiled as it is, aren't till ADR 1135's last step. */
const piecewisePairs = join(root, "test", "crates", "piecewise");

/** `cases/name`'s crates, as `chain` compiles them, where writes into a
 * writer are followed a piece at a time. */
function piecewiseChain(cases: string, name: string, refused?: string[]) {
  return chain(cases, name, ["dep", "app"], refused);
}

// Step 5: a library's generic writer writes what a consumer gives it by
// the dictionary the consumer gives, whose `fmtInto` its contract asks for
// (`written_into` of the parameter); and a consumer writes the library's
// type through its own generic code by the library's dictionary, which its
// contract lists (`written_into`).
test("a library's generic writer, and a library's type written through generic code, write a piece at a time", () => {
  const out = piecewiseChain(piecewisePairs, "writer_into_a_librarys_generic_writer");
  const library = contract(out);
  expect(library.written_into.length).toBe(1);
  const framed = library.items.find((item: { export: string }) => item.export === "framed");
  const written = framed.params.filter((param: { marks: string[] }) => param.marks.includes("written_into"));
  expect(written.map((param: { index: number }) => param.index)).toEqual([1]);
}, underLoad(300_000));

// A library's type reachable from outside it is published: its writer
// writes through a sink and its dictionary carries `fmtInto`, which its
// contract lists, so a consumer writes it into a writer a piece at a time,
// through generic code and directly; one of a lifetime alone too, one
// whose derived `Debug` the consumer compiles its own of, and one holding a
// private type whose `Debug` nothing shows.
test("a library's published type is written into a consumer's writer a piece at a time", () => {
  const out = piecewiseChain(piecewisePairs, "librarys_published_type_through_generic_code");
  // Not `Bytes`, the private type `Der` holds, which nothing writes.
  expect(contract(out).written_into.length).toBe(6);
}, underLoad(300_000));

// A library's writer function, which no writer of its own calls, is
// framed, as a consumer's writer may give it its `Formatter`.
test("a library's writer function given a consumer's Formatter writes a piece at a time", () => {
  piecewiseChain(piecewisePairs, "librarys_writer_function_given_a_consumers_formatter");
}, underLoad(300_000));

// A library's trait's method that writes the `Arguments` it's given, as
// its contract says (`written_trait_arguments`), given a consumer's through
// a `dyn` (#1735); and a consumer's callables a library's writer calls
// with its `Formatter`, framed (#1736).
test("a consumer's Arguments given a library's trait's method through a dyn writes a piece at a time", () => {
  const out = piecewiseChain(piecewisePairs, "arguments_through_a_librarys_dyn_trait");
  expect(contract(out).written_trait_arguments.length).toBe(1);
}, underLoad(300_000));

test("a consumer's closure and fn pointer a library's writer calls write a piece at a time", () => {
  piecewiseChain(piecewisePairs, "consumers_closure_called_by_a_librarys_writer");
}, underLoad(300_000));

// `$writeBy` (#1712): a dictionary's `fmtInto` where it carries one, each
// piece a call of the writer's; else, into a sink no writer armed, `fmt`'s
// text; else, into an armed sink, a `PadAdapter`'s over one too, a thrown
// rust-js error, never the text whole.
test("$writeBy writes by fmtInto, else its text where nothing's armed, else throws", async () => {
  const std: Record<string, any> = await import(helpersModule);
  const log: string[] = [];
  const f = std.$writerSink(log, (w: string[], s: string) => w.push(`s${s}`), (w: string[], c: string) => w.push(`c${c}`));
  const plain = { fmt: (x: number) => `<${x}>` };
  expect(() => std.$writeBy(f, plain, 1)).toThrow("a bug of rust-js's");
  expect(() => std.$writeBy(std.$padAdapter(f), plain, 1)).toThrow("a bug of rust-js's");
  std.$writingInto(f, () => {
    const armed = std.$sink();
    expect(() => std.$writeBy(armed, plain, 1)).toThrow("a bug of rust-js's");
    return armed.done();
  });
  const text = std.$sink();
  std.$writeBy(text, plain, 2);
  expect(text.done()).toBe("<2>");
  const pieces = { fmt: plain.fmt, fmtInto: (g: any, x: number) => [g.write("a"), g.write(String(x))] };
  std.$writeBy(f, pieces, 3);
  expect(log).toEqual(["sa", "s3"]);
});

test("a writer armed that gives back its text, a writer it called claiming the sink, is an error, not that text", async () => {
  const std: Record<string, any> = await import(helpersModule);
  const f = std.$writerSink([], (w: string[], s: string) => w.push(s), (w: string[], c: string) => w.push(c));
  const claiming = () => {
    const g = std.$sink();
    g.write("x");
    return g.done();
  };
  expect(() => std.$writingInto(f, () => `a${claiming()}`)).toThrow("didn't write through its sink");
  expect(() => std.$writingInto(f, claiming)).not.toThrow();
});

// A library's derived `Debug` of more than five fields makes `&dyn Debug`s
// of them that never leave it: it makes no `dyn` a consumer may write, so
// a consumer's `dyn` written into a writer isn't refused (#1688).
test("a consumer's dyn written beside a library's derived Debug of six fields writes a piece at a time", () => {
  const out = piecewiseChain(piecewisePairs, "dyn_written_beside_a_librarys_derived_debug");
  expect(contract(out).makes_fmt_dyns ?? false).toBe(false);
}, underLoad(300_000));

// A type of the consumer's given only to the library's generic writer,
// which the consumer writes into nothing itself: its writer is framed, as
// what the library's parameter is given is written a piece at a time.
test("a consumer's type given only to a library's generic writer writes a piece at a time", () => {
  piecewiseChain(piecewisePairs, "own_type_into_a_librarys_generic_writer");
}, underLoad(300_000));

// Step 6: a library's function that writes an `Arguments` it's given into
// a writer, itself or through another, says so in its contract
// (`written_arguments`), so a consumer's `format_args!` given it is the
// function that writes into the sink it claims; and a library's writer
// with its own `write_fmt`, written into through a consumer's generic
// code, is given that function by its dictionary's `writeFmt`.
test("a library's function given an Arguments, and a library's own write_fmt, write a piece at a time", () => {
  const out = piecewiseChain(piecewisePairs, "arguments_into_a_librarys_writer");
  const library = contract(out);
  const written = (name: string) => library.items.find((item: { export: string }) => item.export === name)?.written_arguments;
  expect(written("logInto")).toEqual([1]);
  expect(written("relay")).toEqual([1]);
}, underLoad(300_000));

// A library's `dyn Debug`, whose pair's dictionary its contract doesn't say
// writes a piece at a time (`makes_fmt_dyns`), written into a writer:
// refused, naming #1688.
test("a library's dyn Debug written into a writer is refused", () => {
  const native = piecewiseChain(piecewisePairs, "dyn_debug_of_a_librarys", [
    "writing a `dyn` into a writer of the crate's own or a generic one where a library it uses makes such a `dyn`",
    "#1688",
  ]);
  expect(native).toBe('["Some", "(", "7", ")"]\n');
}, underLoad(300_000));

// A library makes a `dyn` by a coercion or std's conversion, `?`'s
// `From<E> for Box<dyn Error>`, never by passing one on, either's
// `self.0.source()` (`makes_fmt_dyns`, #1712): a consumer's `dyn` written
// beside the second is written a piece at a time; one of the first's,
// refused, naming #1688.
test("a library that passes a dyn on makes none; one whose ? converts into one does", () => {
  const out = piecewiseChain(piecewisePairs, "dyn_passed_on_by_a_library");
  expect(contract(out).makes_fmt_dyns ?? false).toBe(false);
  const native = piecewiseChain(piecewisePairs, "dyn_made_by_a_librarys_conversion", [
    "writing a `dyn` into a writer of the crate's own or a generic one where a library it uses makes such a `dyn`",
    "#1688",
  ]);
  expect(native).toBe('["[", "bad ", "12", "]"]\n');
}, underLoad(300_000));

// A library frames each writer function another crate can reach, as a
// consumer may name one a `fn` pointer its framed writer calls (#1760).
test("a library's writer function a consumer names a fn pointer writes a piece at a time", () => {
  const out = piecewiseChain(piecewisePairs, "librarys_writer_function_named_a_fn_pointer");
  expect(contract(out).framed_writers.length).toBe(2);
}, underLoad(300_000));

// A library's generic types are published (#1712): an `Either`, a list of
// a constant capacity, thiserror's `Var` and a derived `Debug`, written
// into a consumer's writer and formatted plainly, of std values, the
// consumer's own types and one nested in another. Each writer writes its
// parameters by `$writeBy`, so a consumer's dictionaries carry `fmtInto`
// only where it writes the type into a writer.
test("a library's generic types are written into a consumer's writer a piece at a time", () => {
  const out = piecewiseChain(piecewisePairs, "librarys_generic_types_written_and_formatted");
  expect(contract(out).written_into.length).toBe(4);
}, underLoad(300_000));

// A library's types it gives out as an `impl Display`, one through a
// private function's, are published as its reachable ones are (#1743).
// Where a library's contract doesn't list one, a consumer writing it
// through generic code is refused, naming #1688, as the type it hides.
test("a library's hidden types are written into a consumer's writer; one its contract doesn't list is refused", () => {
  const out = piecewiseChain(piecewisePairs, "librarys_hidden_type_through_generic_code");
  const manifest = join(out, "dep", "lib.manifest.json");
  const read = JSON.parse(readFileSync(manifest, "utf8"));
  expect(read.library.written_into.length).toBe(2);
  read.library.written_into = [];
  writeFileSync(manifest, JSON.stringify(read));
  const app = readFileSync(join(piecewisePairs, "librarys_hidden_type_through_generic_code", "app.rs"), "utf8");
  expect(() => counting(out, app)).toThrow("writing a `dep::Hidden` into a writer");
}, underLoad(300_000));

test("serde_json's error and an OS error write their Display a piece at a time, as their crates do", async () => {
  expect(await recorded((f, std) => std.$writeJsonError(f, { message: "expected value", line: 0, column: 0 }))).toEqual(["sexpected value"]);
  expect(await recorded((f, std) => std.$writeJsonError(f, { message: "EOF", line: 2, column: 15 }))).toEqual([
    "sEOF",
    "s at line ",
    "s2",
    "s column ",
    "s15",
  ]);
  expect(await recorded((f, std) => std.$writeIoError(f, { code: 2, message: "No such file or directory", kind: "NotFound" }))).toEqual([
    "sNo such file or directory",
    "s (os error ",
    "s2",
    "s)",
  ]);
  expect(await recorded((f, std) => std.$writeIoError(f, { message: "failed to fill whole buffer", kind: "UnexpectedEof" }, { width: 30 }))).toEqual([
    "sfailed to fill whole buffer",
    "c ",
    "c ",
    "c ",
  ]);
});
