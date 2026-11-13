// The formatting grid (#941): every format spec of each trait, of each
// value it applies to, a case. A spec is a placeholder's options, width,
// fill and alignment, `+`, `#`, `0`, precision, given or by argument
// (`{:1$}`, `{:.*}`, `{:>w$.p$}`), of `Display`, `Debug`, `LowerHex`,
// `UpperHex`, `Octal`, `Binary`, `LowerExp` and `UpperExp`; a value, each
// integer and float type at its boundaries, `char`s and strings with
// escapes, wide and combining characters, `bool`, `Option`, `Result`,
// tuples, a `Vec`, a `HashMap` of one entry, structs and enums with derived
// `Debug`, and the crate's own `Display` that pads (`f.pad`), asks
// `f.alternate()`, passes its options on or writes past them, and its own
// `Debug` by `f.debug_struct` and the rest. Each case writes its value
// bracketed, `[{:>7}]`, so a pad shows, by `println!`, by `format!` or by
// `write!` into a `String`, in turn; or, of a value whose split
// `display::sink_writes` follows (#1652), by `write!` into a writer of the
// crate's own that records each call it's given, `Log`, or `Chars`, which
// has its own `write_char`, given the value held in a variable, which rustc
// doesn't write into the format string as it does a literal. Its
// \`format_args!\` given on, \`args\`, of such a value is given both to a
// function that prints it, the function that makes its text, and to one
// that prints it and writes it into a \`Log\`, the function that writes into
// the sink it claims (step 6), so each prints the same text as native
// Rust's. test/iterator-matrix.test.ts runs it
// (`ITERATOR_TABLE=format`) in both builds against native Rust, as it runs
// the iterator tables.
//
// A combination that isn't Rust isn't made: each spec says the trait it
// formats by, each value the traits its type has. Nothing is skipped by a
// list. Its output is native Rust's alone to say: a `HashMap` holds one
// entry, so no run depends on its order.

import type { Row } from "./iterator-matrix";

/** The traits a placeholder formats by: `D` `Display`, `G` `Debug`, `x`
 * `LowerHex`, `X` `UpperHex`, `o` `Octal`, `b` `Binary`, `e` `LowerExp`, `E`
 * `UpperExp`. */
type Trait = "D" | "G" | "x" | "X" | "o" | "b" | "e" | "E";

/** A spec: its placeholder, the trait it formats by, and how its value is
 * given beside what else it's given, `v` the value's expression. */
type Spec = { text: string; trait: Trait; args?: (v: string) => string };

const plain = (trait: Trait, ...texts: string[]): Spec[] => texts.map((text) => ({ text, trait }));

export const specs: Spec[] = [
  ...plain("D", "{}", "{:7}", "{:<7}", "{:^7}", "{:>7}", "{:*^9}", "{:-<6}", "{:é>5}", "{:2}", "{:+}", "{:07}", "{:+08}", "{:^08}", "{:<07}"),
  ...plain("D", "{:.0}", "{:.2}", "{:9.3}", "{:<9.1}", "{:.10}", "{:+.1}", "{:#}"),
  { text: "{:1$}", trait: "D", args: (v) => `${v}, 9` },
  { text: "{:.*}", trait: "D", args: (v) => `3, ${v}` },
  { text: "{:>w$.p$}", trait: "D", args: (v) => `${v}, w = 9, p = 2` },
  ...plain("G", "{:?}", "{:#?}", "{:10?}", "{:<10?}", "{:^12?}", "{:.1?}", "{:+?}", "{:08?}", "{:x?}", "{:#X?}", "{:#x?}"),
  { text: "{:>1$?}", trait: "G", args: (v) => `${v}, 11` },
  ...plain("x", "{:x}", "{:#x}", "{:#010x}", "{:8x}", "{:<#8x}"),
  ...plain("X", "{:X}", "{:#X}", "{:^#9X}"),
  ...plain("o", "{:o}", "{:#o}"),
  ...plain("b", "{:b}", "{:#b}", "{:010b}", "{:#010b}"),
  ...plain("e", "{:e}", "{:.3e}", "{:12.2e}", "{:+e}", "{:<12e}", "{:012e}", "{:.0e}"),
  ...plain("E", "{:E}", "{:.2E}", "{:+011.1E}"),
];

/** A value: what kind of type it is, its type, its expression, the traits
 * its type has, and whether its digits in a radix are as many as its
 * type's width, which is wasm32's, not native Rust's (`sized`). */
type Value = { kind: string; ty: string; expr: string; has: Trait[]; sized?: boolean };

const numeric: Trait[] = ["D", "G", "x", "X", "o", "b", "e", "E"];
const float: Trait[] = ["D", "G", "e", "E"];
const shown: Trait[] = ["D", "G"];
const debug: Trait[] = ["G"];
const display: Trait[] = ["D"];

const ints = ["i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize"];

const strings = [
  `""`,
  `"abc"`,
  `"héllo"`,
  `"中文"`,
  `"e\\u{301}"`,
  `"a\\tb"`,
  `"say \\"hi\\""`,
  `"back\\\\slash"`,
  `"it's"`,
  `"😀x"`,
  `"line\\r\\n"`,
  `"\\u{200b}"`,
  `"\\u{7f}\\u{1b}[0m"`,
  `"\\0"`,
  `"\\u{feff}\\u{10ffff}"`,
  `"\\u{301}e"`,
  `"\\u{a0}nb\\u{ad}sp"`,
  `"\\u{2028}\\u{e000}"`,
  `"bell\\u{7}"`,
];

const chars = [
  `'a'`,
  `'é'`,
  `'中'`,
  `'😀'`,
  `'\\n'`,
  `'\\t'`,
  `'\\r'`,
  `'\\''`,
  `'"'`,
  `'\\\\'`,
  `'\\0'`,
  `'\\u{7}'`,
  `'\\u{7f}'`,
  `'\\u{a0}'`,
  `'\\u{ad}'`,
  `'\\u{301}'`,
  `'\\u{200b}'`,
  `'\\u{2028}'`,
  `'\\u{e000}'`,
  `'\\u{fffd}'`,
  `'\\u{10ffff}'`,
];

const floats = (ty: "f32" | "f64") => [
  `0.0_${ty}`,
  `-0.0_${ty}`,
  `1.0_${ty}`,
  `-1.5_${ty}`,
  `0.1_${ty}`,
  `0.3_${ty}`,
  `2.5_${ty}`,
  `1e-7_${ty}`,
  `1e16_${ty}`,
  `1e21_${ty}`,
  `123456.789_${ty}`,
  `0.125_${ty}`,
  `0.375_${ty}`,
  `1.005_${ty}`,
  `9.5_${ty}`,
  `4.35_${ty}`,
  `1e-5_${ty}`,
  `1e22_${ty}`,
  `1e23_${ty}`,
  `(0.1_${ty} + 0.2)`,
  `${ty}::MAX`,
  `${ty}::MIN`,
  `${ty}::MIN_POSITIVE`,
  ty === "f32" ? `1e-45_f32` : `5e-324_f64`,
  `${ty}::INFINITY`,
  `${ty}::NEG_INFINITY`,
  `${ty}::NAN`,
];

export const values: Value[] = [
  ...ints.flatMap((ty) => {
    const signed = ty.startsWith("i");
    // `isize` and `usize` are wasm32's, 32 bits, as rustc checks programs
    // for it (ADR 0090), where native Rust's are 64: their bounds written as
    // wasm32's, and a negative one not in a radix, whose digits are as many
    // as its width.
    const sized = ty === "isize" || ty === "usize";
    const bounds = !sized ? [`${ty}::MIN`, `${ty}::MAX`] : signed ? ["-0x8000_0000_isize", "0x7fff_ffff_isize"] : ["0xffff_ffff_usize"];
    const made = [...bounds, `0_${ty}`, `1_${ty}`, `42_${ty}`, ...(signed ? [`-1_${ty}`] : [])];
    return made.map((expr) => ({ kind: "integer", ty, expr, has: numeric, sized: sized && expr.startsWith("-") }));
  }),
  // What rounds as an exponent's precision cuts it: ties, and a carry.
  ...["1255_u32", "999_u16", "123456789_i64", "-1000000_i32", "0xdead_beef_u32", "250_u8"].map((expr) => ({
    kind: "integer",
    ty: /_(\w+)$/.exec(expr)![1],
    expr,
    has: numeric,
  })),
  ...(["f32", "f64"] as const).flatMap((ty) => floats(ty).map((expr) => ({ kind: "float", ty, expr, has: float }))),
  ...chars.map((expr) => ({ kind: "char", ty: "char", expr, has: shown })),
  ...strings.map((expr) => ({ kind: "str", ty: "&str", expr, has: shown })),
  ...strings.slice(0, 8).map((expr) => ({ kind: "String", ty: "String", expr: `String::from(${expr})`, has: shown })),
  { kind: "bool", ty: "bool", expr: "true", has: shown },
  { kind: "bool", ty: "bool", expr: "false", has: shown },
  { kind: "unit", ty: "()", expr: "()", has: debug },
  { kind: "Option", ty: "Option<i32>", expr: "Some(5)", has: debug },
  { kind: "Option", ty: "Option<i32>", expr: "None::<i32>", has: debug },
  { kind: "Option", ty: "Option<&str>", expr: `Some("a\\"b")`, has: debug },
  { kind: "Option", ty: "Option<f64>", expr: "Some(0.1_f64)", has: debug },
  { kind: "Option", ty: "Option<Option<u8>>", expr: "Some(None::<u8>)", has: debug },
  { kind: "Result", ty: "Result<u8, &str>", expr: "Ok::<u8, &str>(1)", has: debug },
  { kind: "Result", ty: "Result<u8, &str>", expr: `Err::<u8, &str>("bad")`, has: debug },
  { kind: "tuple", ty: "(i32, &str, char)", expr: `(1, "a", 'c')`, has: debug },
  { kind: "tuple", ty: "(f32,)", expr: "(1.5_f32,)", has: debug },
  { kind: "tuple", ty: "((i32, i32), [u8; 2])", expr: "((1, -2), [3_u8, 4])", has: debug },
  { kind: "Vec", ty: "Vec<i32>", expr: "vec![1, -2, 300]", has: debug },
  { kind: "Vec", ty: "Vec<i32>", expr: "Vec::<i32>::new()", has: debug },
  { kind: "Vec", ty: "Vec<&str>", expr: `vec!["x", "y\\n"]`, has: debug },
  { kind: "Vec", ty: "Vec<Option<u8>>", expr: "vec![Some(1_u8), None]", has: debug },
  { kind: "Vec", ty: "Vec<Point>", expr: "vec![Point { x: 1, y: 2 }]", has: debug },
  { kind: "array", ty: "[u8; 3]", expr: "[1_u8, 2, 3]", has: debug },
  { kind: "array", ty: "&[f64]", expr: "&[1.5_f64, 2.0][..]", has: debug },
  { kind: "HashMap", ty: "HashMap<&str, i32>", expr: `HashMap::from([("k", 1)])`, has: debug },
  { kind: "HashMap", ty: "HashMap<u8, Vec<u8>>", expr: "HashMap::from([(7_u8, vec![8_u8])])", has: debug },
  { kind: "HashSet", ty: "HashSet<char>", expr: "HashSet::from(['z'])", has: debug },
  { kind: "struct", ty: "Point", expr: "Point { x: 1, y: -2 }", has: debug },
  { kind: "struct", ty: "Wrap", expr: "Wrap(3)", has: debug },
  { kind: "struct", ty: "Unit", expr: "Unit", has: debug },
  { kind: "struct", ty: "Pair<u8>", expr: "Pair(1_u8, 2_u8)", has: debug },
  { kind: "struct", ty: "Outer", expr: `Outer { name: "o", inner: Point { x: 0, y: 5 }, tags: vec!["a"], opt: None }`, has: debug },
  { kind: "enum", ty: "Shape", expr: "Shape::Dot", has: debug },
  { kind: "enum", ty: "Shape", expr: "Shape::Circle(1.5)", has: debug },
  { kind: "enum", ty: "Shape", expr: "Shape::Rect { w: 2, h: 3 }", has: debug },
  { kind: "own Display", ty: "Padded", expr: `Padded("hi")`, has: display },
  { kind: "own Display", ty: "Alt", expr: "Alt(5)", has: display },
  { kind: "own Display", ty: "Passed", expr: "Passed(7.25)", has: display },
  { kind: "own Display", ty: "Written", expr: "Written(8)", has: display },
  { kind: "own Display", ty: "Nested", expr: "Nested(9)", has: display },
  { kind: "own Display", ty: "Asked", expr: "Asked", has: display },
  { kind: "own Debug", ty: "Manual", expr: "Manual", has: debug },
  { kind: "pointer", ty: "&i32", expr: "&-5_i32", has: numeric },
  { kind: "pointer", ty: "Box<u8>", expr: "Box::new(200_u8)", has: shown },
  { kind: "pointer", ty: "Rc<str>", expr: `Rc::<str>::from("rc")`, has: shown },
];

/** How a case writes what it formats. */
const sinks = ["println", "format", "write", "panic", "args", "log"] as const;
type Sink = (typeof sinks)[number];

/** Whether `display::sink_writes` follows std's split of `spec` of
 * `value`, so `write!` into a writer of the crate's own writes it a piece
 * at a time (#1652): every value's but `Asked`'s `Debug` of a
 * `fmt::Alignment`, which is written to a `String`. */
const pieced = (_spec: Spec, value: Value) => value.ty !== "Asked";

export type FormatCase = Row & { spec: Spec; value: Value; sink: Sink };

/** Whether `spec` writes in a radix: hex, octal or binary, or `Debug`'s
 * hex, `{:x?}`. */
const radix = (spec: Spec) => ["x", "X", "o", "b"].includes(spec.trait) || /[xX]\?/.test(spec.text);

/** Each spec of each value whose type has its trait, each by a sink in
 * turn; not one in a radix of a value whose digits are its width's. */
export function formatCases(): FormatCase[] {
  const out: FormatCase[] = [];
  for (const value of values) {
    for (const spec of specs) {
      if (!value.has.includes(spec.trait) || (value.sized && radix(spec))) continue;
      const turn = sinks[out.length % sinks.length];
      const sink = turn === "log" && !pieced(spec, value) ? "write" : turn;
      const id = `${spec.text} | ${value.ty} | ${value.expr} | ${sink}`;
      out.push({ n: out.length, id, spec, value, sink });
    }
  }
  return out;
}

const prelude = `// Generated by test/format-grid.ts (#941). Each case prints its name,
// then what it formats, bracketed.
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write};
use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Wrap(i32);

#[derive(Debug)]
struct Unit;

#[derive(Debug)]
struct Pair<T>(T, T);

#[derive(Debug)]
struct Outer {
    name: &'static str,
    inner: Point,
    tags: Vec<&'static str>,
    opt: Option<i32>,
}

#[derive(Debug)]
enum Shape {
    Dot,
    Circle(f64),
    Rect { w: u32, h: u32 },
}

/** Its text, padded as it's asked, \`f.pad\`'s. */
struct Padded(&'static str);

impl fmt::Display for Padded {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(self.0)
    }
}

/** Another text where it's asked \`#\`. */
struct Alt(u32);

impl fmt::Display for Alt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "alt {}", self.0)
        } else {
            write!(f, "{}", self.0)
        }
    }
}

/** Its number, with the options it's given. */
struct Passed(f64);

impl fmt::Display for Passed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/** Its number, past the options it's given. */
struct Written(u8);

impl fmt::Display for Written {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

/** Its number padded by its own placeholder, past the options it's given. */
struct Nested(u8);

impl fmt::Display for Nested {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{:>4}|{:<+3}>", self.0, self.0)
    }
}

/** What \`Formatter\` says it was asked. */
struct Asked;

impl fmt::Display for Asked {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "w={:?} p={:?} fill={:?} align={:?} plus={} alt={} zero={}",
            f.width(),
            f.precision(),
            f.fill(),
            f.align(),
            f.sign_plus(),
            f.alternate(),
            f.sign_aware_zero_pad()
        )
    }
}

/** Each of \`Formatter\`'s debug builders. */
struct Manual;

impl fmt::Debug for Manual {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Manual").field("a", &1).field("b", &"two").finish()?;
        f.debug_tuple("T").field(&3.5).finish()?;
        f.debug_list().entries([1, 2]).finish()?;
        f.debug_map().entry(&"k", &'v').finish()?;
        f.debug_set().entry(&Unit).finish()?;
        f.debug_struct("More").field("c", &None::<u8>).finish_non_exhaustive()
    }
}

/** A writer that records each call it's given, std's \`write_char\` a
 * \`write_str\` of the \`char\`. */
struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

/** One with its own \`write_char\`, which marks each \`char\`. */
struct Chars(Vec<String>);

impl Write for Chars {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.0.push(format!("<{c}>"));
        Ok(())
    }
}

/** What a panic said, its \`String\`. */
fn said(ended: Result<(), Box<dyn Any + Send>>) -> String {
    match ended {
        Ok(()) => "no panic".to_string(),
        // A \`String\`, or a \`&str\` where its arguments are constants,
        // \`format_args!\` folds them into its text.
        Err(payload) => match payload.downcast::<String>() {
            Ok(text) => *text,
            Err(payload) => payload.downcast_ref::<&str>().map_or("?", |text| text).to_string(),
        },
    }
}

/** What \`format_args!\` makes, given on and written. */
fn shown(args: fmt::Arguments) {
    println!("{args}");
}

/** What \`format_args!\` makes, given on, written, and written into a
 * writer a piece at a time, which makes it the function that writes into
 * the sink it claims, called plainly first (#1652, step 6). */
fn logged(args: fmt::Arguments) {
    println!("{args}");
    let mut log = Log(Vec::new());
    log.write_fmt(args).unwrap();
    println!("{:?}", log.0);
}

fn run(name: &str, case: fn()) {
    println!("case {name}");
    if catch_unwind(case).is_err() {
        println!("panicked");
    }
}`;

const escaped = (text: string) => text.replaceAll("\\", "\\\\").replaceAll('"', '\\"');

/** A case's function, `case_<i>`. */
function caseFn(c: FormatCase, i: number): string {
  const args = c.spec.args?.(c.value.expr) ?? c.value.expr;
  const format = `"[${escaped(c.spec.text)}]", ${args}`;
  const body =
    c.sink === "println"
      ? [`println!(${format});`]
      : c.sink === "format"
        ? [`let s = format!(${format});`, `println!("{s}");`]
        : c.sink === "write"
          ? [`let mut s = String::new();`, `write!(s, ${format}).unwrap();`, `println!("{s}");`]
          : c.sink === "panic"
            ? [`let ended = catch_unwind(AssertUnwindSafe(|| -> () { panic!(${format}) }));`, `println!("{}", said(ended));`]
            : c.sink === "log"
              ? [
                  `let v = ${c.value.expr};`,
                  `let mut log = ${i % 2 === 0 ? "Log" : "Chars"}(Vec::new());`,
                  `write!(log, "[${escaped(c.spec.text)}]", ${c.spec.args?.("v") ?? "v"}).unwrap();`,
                  `println!("{:?}", log.0);`,
                ]
              : pieced(c.spec, c.value)
                ? [`shown(format_args!(${format}));`, `logged(format_args!(${format}));`]
                : [`shown(format_args!(${format}));`];
  return `fn case_${i}() {\n${body.map((line) => `    ${line}`).join("\n")}\n}`;
}

/** A program of `cases`, which runs each in turn. */
export function formatProgram(cases: FormatCase[]): string {
  const fns = cases.map((c, i) => caseFn(c, i));
  const main = `fn main() {\n${cases.map((c, i) => `    run(${JSON.stringify(c.id)}, case_${i});`).join("\n")}\n}`;
  return [prelude, ...fns, main].join("\n\n") + "\n";
}

/** What a run without `ITERATOR_MATRIX` covers, of a case (`slice()` in
 * test/iterator-matrix.ts): its spec, its value's type, and its sink. */
export const formatStrata = (c: FormatCase) => [c.spec.text, c.value.ty, c.sink];
