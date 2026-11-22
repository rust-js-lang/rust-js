// What the compiler writes of a `concat!` of literals, and of a format string
// written as one (ADR 0641): its pieces joined by `+`, as written, compiled
// and run. Split from test/lowering-strings.test.ts by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0641: a `concat!` of literals is its pieces joined by `+`, as written,
// as React splits a long message over lines: each its own JS string, in
// order, a string's text and a char's, an integer's, a float's or a bool's
// as `concat!` writes it. A piece written across lines is a template
// literal of them, and one a `\` continues is one piece, as it's one
// literal in Rust; a `concat!` of a macro's value is the one string it is.
// A `static`'s, and a `panic!`'s message, are its pieces too.
test("a concat! of literals is its pieces joined by +", async () => {
  const dir = fixture("concat-pieces");
  writeFileSync(join(dir, "lib.rs"), `use react::webapi::console;

pub const OMITTED_PROP_ERROR: &str = concat!(
    "This object has been omitted by React in the console log ",
    "to avoid sending too much data from the server. Try logging smaller ",
    "or more specific objects."
);

pub fn warn() {
    console::error((concat!("a ", "b"),));
}

pub fn kinds() -> &'static str {
    concat!("v", 1, ".", 2.5, '!', true, -3, 0x10, 1e3)
}

pub fn continued() -> &'static str {
    concat!(
        "a \\
         b",
        "c",
    )
}

pub fn lines() -> &'static str {
    concat!(r#"<a>
</a>"#, r"!")
}

pub fn macro_arg() -> &'static str {
    concat!("a", stringify!(b))
}

pub fn one() -> &'static str {
    concat!("one")
}

pub fn bound() -> usize {
    let s = concat!("x", "yz");
    s.len()
}

pub fn twice() -> (&'static str, &'static str) {
    (OMITTED_PROP_ERROR, OMITTED_PROP_ERROR)
}

pub static GREETING: &str = concat!("hello ", "world");

pub fn fails() {
    panic!(concat!("Expected a ", "ref."));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain([
    "export const OMITTED_PROP_ERROR =",
    '  "This object has been omitted by React in the console log " +',
    '  "to avoid sending too much data from the server. Try logging smaller " +',
    '  "or more specific objects.";',
  ].join("\n"));
  expect(js).toContain('  console.error("a " + "b");');
  expect(js).toContain('  return "v" + "1" + "." + "2.5" + "!" + "true" + "-3" + "16" + "1e3";');
  expect(js).toContain('  return "a b" + "c";');
  expect(js).toContain('  return (\n    `<a>\n</a>` + "!"\n  );');
  expect(js).toContain('  return "ab";');
  expect(js).toContain('  return "one";');
  expect(js).toContain('"x" + "yz"');
  expect(js).toContain("  return [OMITTED_PROP_ERROR, OMITTED_PROP_ERROR];");
  expect(js).toContain('export const GREETING = "hello " + "world";');
  expect(js).toContain('  throw $panic("Expected a " + "ref.", "lib.rs:50:5");');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.OMITTED_PROP_ERROR).toBe("This object has been omitted by React in the console log to avoid sending too much data from the server. Try logging smaller or more specific objects.");
  expect([lib.kinds(), lib.continued(), lib.lines(), lib.macroArg(), lib.one(), lib.bound()])
    .toEqual(["v1.2.5!true-3161e3", "a bc", "<a>\n</a>!", "ab", "one", 3]);
  expect(lib.GREETING).toBe("hello world");
  expect(() => lib.fails()).toThrow("Expected a ref.");
  const logged: unknown[] = [];
  const error = console.error;
  console.error = (...args: unknown[]) => logged.push(args);
  try {
    lib.warn();
  } finally {
    console.error = error;
  }
  expect(logged).toEqual([["a b"]]);
});

// React's lazy() splits its message's `import` with a comment of why,
// inside the `+` of literals: a `concat!`'s comment is before the piece
// it's written before, not after the strings (ADR 0641, amended).
test("a comment inside a concat! is before the piece it's written before", async () => {
  const dir = fixture("concat-comment");
  writeFileSync(join(dir, "lib.rs"), `use react::webapi::console;

pub const MESSAGE: &str = concat!(
    "lazy: Expected the result of a dynamic imp",
    // Break up imports to avoid accidentally parsing them as dependencies.
    "ort() call.",
);

pub fn warn(module_object: &str) {
    console::error((
        concat!(
            "Your code should look like: \\n  ",
            // Break up imports to avoid accidentally parsing them as dependencies.
            "const MyComponent = lazy(() => imp",
            "ort('./MyComponent'))"
        ),
        module_object,
    ));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export const MESSAGE =
  "lazy: Expected the result of a dynamic imp" +
  // Break up imports to avoid accidentally parsing them as dependencies.
  "ort() call.";`);
  expect(js).toContain(`  console.error(
    "Your code should look like: \\n  " +
      // Break up imports to avoid accidentally parsing them as dependencies.
      "const MyComponent = lazy(() => imp" +
      "ort('./MyComponent'))",
    moduleObject,
  );`);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.MESSAGE).toBe("lazy: Expected the result of a dynamic import() call.");
});

// ADR 0641: a format string written as a `concat!` of literals is its
// pieces joined by `+`, as React's formatProdErrorMessage writes its
// message: each a template literal where it holds a placeholder, and
// otherwise a string. A placeholder or a `{{` across pieces, which only the
// string they make has, and a value rustc writes in the template, keep one.
test("a format string of a concat! is its pieces joined by +", async () => {
  const dir = fixture("format-concat");
  writeFileSync(join(dir, "lib.rs"), `use std::fmt;
use std::fmt::Write;

pub fn format_prod_error_message(code: u32, url: &str) -> String {
    format!(
        concat!(
            "Minified React error #{}; visit {} for the full message or ",
            "use the non-minified dev environment for full errors and additional ",
            "helpful warnings."
        ),
        code,
        url
    )
}

pub fn spec(x: f64, n: usize, s: &str) -> String {
    format!(concat!("{1} [{0:>8.2}] ", "{x:.1$}|{s:<4}|", "{s:?}"), x, n, x = x, s = s)
}

pub fn spans(n: u32) -> (String, String, String) {
    (
        format!(concat!("<{", "}>"), n),
        format!(concat!("a{", "{b {}"), n),
        format!(concat!("{{{}}} ", "{} and {}"), n, "lit", n),
    )
}

pub fn written_in(n: u32) -> (String, String) {
    (
        format!(concat!("v ", "{}{}"), "lit", n),
        format!(concat!("a ", "{}"), format_args!("{}b", n)),
    )
}

pub fn escapes(n: u32) -> String {
    format!(concat!("{{{}}} ", "{}}}"), n, n)
}

const N: u32 = 3;

pub fn none() -> (String, String) {
    (format!(concat!("a ", "b")), format!(concat!("n + 1 is ", "{}"), N + 1))
}

fn pair(n: u32) -> (u32, u32) {
    (n, n + 1)
}

pub fn pairs(n: u32) -> String {
    format!(concat!("pairs {:?}", " and {:?}", "!"), pair(n), pair(1))
}

pub fn lines(n: u32) -> String {
    format!(concat!("<p>
{}", "</p>\\n", "<a>{}\\n", "</a>
"), n, n)
}

pub fn logged(n: u32) {
    println!(concat!("n is ", "{}", "!"), n);
    println!(concat!("no ", "placeholder"));
}

pub fn appended(n: u32) -> String {
    let mut s = String::new();
    write!(s, concat!("w ", "{}"), n).unwrap();
    writeln!(s, concat!(" l ", "{}"), n).unwrap();
    s
}

pub struct Point(pub i32, pub i32);

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, concat!("(", "{}, {}", ")"), self.0, self.1)
    }
}

pub fn point(x: i32, y: i32) -> String {
    Point(x, y).to_string()
}

pub fn fails(n: u32) {
    panic!(concat!("bad ", "{}"), n);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // React's, as upstream writes it.
  expect(js).toContain([
    "  return (",
    "    `Minified React error #${code}; visit ${url} for the full message or ` +",
    '    "use the non-minified dev environment for full errors and additional " +',
    '    "helpful warnings."',
    "  );",
  ].join("\n"));
  // Positional, named, width and precision, each in its piece.
  expect(js).toContain([
    "    `${n} [${$toFixed(x, 2).padStart(8)}] ` +",
    '    `${$toFixed(x, n)}|${$pad(s, 4, "<")}|` +',
    "    `${$debugStr(s)}`",
  ].join("\n"));
  // A placeholder or a `{{` across pieces, and a literal rustc writes in
  // the template, are one.
  expect(js).toContain("return [`<${n}>`, `a{b ${n}`, `{${n}} lit and ${n}`];");
  expect(js).toContain("return [`v lit${n}`, `a ${n}b`];");
  expect(js).toContain("return `{${n}} ` + `${n}}`;");
  // Of nothing left to show, its pieces are a string's.
  expect(js).toContain('return ["a " + "b", "n + 1 is " + "4"];');
  // A value's own template, a tuple's `{:?}`, is in its piece's.
  expect(js).toContain("return `pairs (${value[0]}, ${value[1]})` + ` and (${value$1[0]}, ${value$1[1]})` + \"!\";");
  // Each piece's line breaks as it writes them.
  expect(js).toContain('    `<p>\n${n}` +\n    "</p>\\n" +\n    `<a>${n}\\n` +\n    `</a>\n`\n');
  expect(js).toContain('console.log("n is " + `${n}` + "!");');
  expect(js).toContain('console.log("no " + "placeholder");');
  expect(js).toContain('s += "w " + `${n}`;');
  expect(js).toContain('s += " l " + `${n}\\n`;');
  expect(js).toContain('return "(" + `${point[0]}, ${point[1]}` + ")";');
  expect(js).toContain('throw $stringPanic("bad " + `${n}`, "lib.rs:84:5");');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.formatProdErrorMessage(31, "https://react.dev/errors/31")).toBe(
    "Minified React error #31; visit https://react.dev/errors/31 for the full message or use the non-minified dev environment for full errors and additional helpful warnings.",
  );
  expect(lib.spec(3.14159, 3, "ab")).toBe('3 [    3.14] 3.142|ab  |"ab"');
  expect(lib.spans(5)).toEqual(["<5>", "a{b 5", "{5} lit and 5"]);
  expect([lib.writtenIn(5), lib.escapes(1), lib.none(), lib.pairs(2)])
    .toEqual([["v lit5", "a 5b"], "{1} 1}", ["a b", "n + 1 is 4"], "pairs (2, 3) and (1, 2)!"]);
  expect([lib.lines(1), lib.appended(2), lib.point(1, -2)]).toEqual(["<p>\n1</p>\n<a>1\n</a>\n", "w 2 l 2\n", "(1, -2)"]);
  expect(() => lib.fails(4)).toThrow("bad 4");
  const logged: unknown[] = [];
  const log = console.log;
  console.log = (...args: unknown[]) => logged.push(args);
  try {
    lib.logged(7);
  } finally {
    console.log = log;
  }
  expect(logged).toEqual([["n is 7!"], ["no placeholder"]]);
});
