// What the compiler writes of strings and formatting: a string across lines as
// a template literal, a replacement as Rust's, a RegExp literal, `format!`'s
// arguments, and a writer's `Formatter`. Each test is a program of its own,
// compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildNode, buildReact, compiler, fixture, installRuntime, run, target } from "./support";

beforeAll(buildReact, 600_000);

// A loop over a string's characters is over the string, which JS steps
// through by them, `for (const char of text)`, as react.dev's createFileMap
// reads a meta string: no `Array.from` first, as nothing changes it.
test("a loop over a string's characters is over the string", async () => {
  const dir = fixture("chars-loop");
  writeFileSync(join(dir, "lib.rs"), `pub fn braces(text: &str) -> u32 {
    let mut n = 0;
    for char in text.chars() {
        if char == '{' {
            n += 1;
        }
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  for (const char of text) {");
  expect((await import(join(dir, "lib.js"))).braces("{a}{😀}")).toBe(2);
});

// A string literal written across lines is a template literal with its
// line breaks, as react.dev's Sandpack template writes a file's code; one
// written with `\n` keeps it.
test("a string written across lines keeps its lines", async () => {
  const dir = fixture("multiline-string");
  writeFileSync(join(dir, "lib.rs"), [
    "pub fn page() -> &'static str {",
    "    r#\"<div>",
    "  `${x}` \\ \"q\"",
    "</div>\"#",
    "}",
    "",
    "pub fn cooked() -> &'static str {",
    "    \"a",
    "b\"",
    "}",
    "",
    "pub fn escaped() -> &'static str {",
    "    \"a\\nb\"",
    "}",
    "",
    "pub fn continued() -> &'static str {",
    "    \"a\\n\\",
    "     b\"",
    "}",
    "",
  ].join("\n"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return `<div>\n  \\`\\${x}\\` \\\\ \"q\"\n</div>`;");
  expect(js).toContain("  return `a\nb`;");
  expect(js).toContain('  return "a\\nb";');
  // A `\` ending a line leaves its break out: the one there is `\n`.
  expect(js).toMatch(/export function continued\(\) \{\n  return "a\\nb";/);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.page(), lib.cooked(), lib.escaped(), lib.continued()]).toEqual(['<div>\n  `${x}` \\ "q"\n</div>', "a\nb", "a\nb", "a\nb"]);
});

// React's ReactPerformanceTrackProperties writes `const UNCHANGED =
// ' \xa0';`, a figure space one can't tell from a space: a space but
// `' '`, and a mark one can't see, is written as its escape, in a string
// and a template, its hex in lowercase, `\xa0`, as upstream writes it (ADR
// 0732, amended): `\x` below U+0100, in a string as in a template. What
// joins an emoji's parts, or varies one, is as it is; and a U+� is
// itself.
test("a space one can't tell from ' ' is written as its escape", async () => {
  const dir = fixture("unseen-escapes");
  writeFileSync(join(dir, "lib.rs"), `pub const UNCHANGED: &str = "\\u{2007}\\u{a0}";
pub const MARKS: &str = "a\\u{200b}b\\u{feff}c\\u{fffd}d\\u{ad}";
pub const FAMILY: &str = "👨\\u{200d}👩 ❤\\u{fe0f}";
pub const QUOTED: &str = "\\u{200b}\\"q\\" \\\\ \\t\\n\\r\\u{1}\\u{2028}";

pub fn line(indent: usize, key: &str) -> String {
    format!("{UNCHANGED}{}{key}\\u{2007}\\u{3000}\\u{ad}", "\\u{a0}".repeat(indent))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export const UNCHANGED = "\\u2007\\xa0";');
  expect(js).toContain('export const MARKS = "a\\u200bb\\ufeffc�d\\xad";');
  expect(js).toContain('export const FAMILY = "👨‍👩 ❤️";');
  expect(js).toContain("${key}\\u2007\\u3000\\xad`;");
  expect(js).toContain(`export const QUOTED = '\\u200b"q" \\\\ \\t\\n\\r\\x01\\u2028';`);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.UNCHANGED, lib.MARKS, lib.FAMILY, lib.line(1, "k"), lib.QUOTED]).toEqual([
    " \xa0",
    "a​b﻿c�d\xad",
    "👨‍👩 ❤️",
    " \xa0\xa0k 　\xad",
    "​\"q\" \\ \t\n\r\u0001 ",
  ]);
});

// A `const` of a string written across lines, a raw string's, is a template
// literal of them, as an inline one is and as react.dev's
// SandpackWithHTMLOutput writes its sandbox's files; one of `\n`s stays a
// string.
test("a const of a string written across lines is a template literal", async () => {
  const dir = fixture("const-lines");
  writeFileSync(join(dir, "lib.rs"), `const PAGE: &str = r#"<main>
  <h1>Hi</h1>
</main>"#;

const PAIR: &str = "a\\nb";

pub fn page() -> String {
    format!("{PAGE}|{PAIR}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const PAGE = `<main>\n  <h1>Hi</h1>\n</main>`;");
  expect(js).toContain('const PAIR = "a\\nb";');
  const { page } = await import(join(dir, "lib.js"));
  expect(page()).toBe("<main>\n  <h1>Hi</h1>\n</main>|a\nb");
});

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
  expect(js).toContain('  throw new Error("Expected a " + "ref.");');
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
  expect(js).toContain('throw new Error("bad " + `${n}`);');
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

// React's ReactChildren throws `Objects are not valid as a React child
// (found: ${childrenString === '[object Object]' ? .. : childrenString}).`,
// its value across lines, as Prettier breaks one written so; its port's
// argument, an `if`, is written across lines too, and so may its JS be,
// where it's too long for one: a line break after `${` (ADR 0641,
// amended). One that fits stays on its line, and one written on one line
// is never broken; a named argument's and an index's are its argument's.
test("a format argument written across lines may break its template's line", async () => {
  const dir = fixture("format-argument-lines");
  writeFileSync(join(dir, "lib.rs"), `pub fn thrown(children_string: String, keys: Vec<String>) -> String {
    format!(
        concat!(
            "Objects are not valid as a React child (found: {}). ",
            "If you meant to render a collection of children, use an array ",
            "instead."
        ),
        if children_string == "[object Object]" {
            "object with keys {".to_string() + &keys.join(", ") + "}"
        } else {
            children_string
        },
    )
}

pub fn fits(count: u32) -> String {
    format!(
        "{} items",
        if count > 1 {
            "many"
        } else {
            "one"
        }
    )
}

pub fn one_line(name: &str, count: u32) -> String {
    format!("{} has {} items, which is a sentence long enough to be broken", name, if count > 1 { "many many many" } else { "one" })
}

pub fn named(count: u32) -> String {
    format!(
        "{0} counted, {kind} kind of a sentence long enough to be broken across lines",
        count,
        kind = if count > 1 {
            "the plural and a great many more words"
        } else {
            "the single one"
        }
    )
}

pub fn inlined(n: u32) -> String {
    format!(
        "{} {} of a sentence long enough to be broken across lines",
        r"a literal
of lines",
        if n > 1 { "the plural and a great many more words" } else { "the single one" }
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // rustc writes the literal in the template: its placeholder isn't one.
  expect(js).toContain('of lines ${n > 1 ? "the plural and a great many more words" : "the single one"} of a sentence');
  expect(js).toContain(
    "    `Objects are not valid as a React child (found: ${\n" +
      '      childrenString === "[object Object]"\n' +
      '        ? "object with keys {" + keys.join(", ") + "}"\n' +
      "        : childrenString\n" +
      "    }). ` +\n",
  );
  expect(js).toContain('  return `${count > 1 ? "many" : "one"} items`;\n');
  expect(js).toContain('`${name} has ${count > 1 ? "many many many" : "one"} items, which is a sentence long enough to be broken`');
  expect(js).toContain("counted, ${\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.thrown("[object Object]", ["a", "b"]), lib.thrown("x", [])]).toEqual([
    "Objects are not valid as a React child (found: object with keys {a, b}). If you meant to render a collection of children, use an array instead.",
    "Objects are not valid as a React child (found: x). If you meant to render a collection of children, use an array instead.",
  ]);
  expect([lib.fits(2), lib.oneLine("n", 1), lib.named(2)]).toEqual([
    "many items",
    "n has one items, which is a sentence long enough to be broken",
    "2 counted, the plural and a great many more words kind of a sentence long enough to be broken across lines",
  ]);
});

// ADR 0034: a replacement is the text it is, as Rust's is: JS reads `$&` and
// `$1` in a string as what's matched, so one with a `$` doubles it, and one
// not written out is a function's, `() => t`. `replacen(.., 1)` is JS's
// `replace`, of the first.
test("a replacement is the text it is, and replacen's one is the first", async () => {
  const dir = fixture("replacements");
  writeFileSync(join(dir, "lib.rs"), `pub fn all(s: &str, t: &str) -> String {
    s.replace("a", t)
}

pub fn dollar(s: &str) -> String {
    s.replace("a", "$&!")
}

pub fn plain(s: &str) -> String {
    s.replace("a", "b")
}

pub fn first(s: &str) -> String {
    s.replacen("export default ", "let App = ", 1)
}

pub fn first_of(s: &str, t: &str) -> String {
    s.replacen("a", t, 1)
}

pub fn by(s: &str, p: &str, t: &str) -> String {
    s.replace(p, t)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return s.replaceAll("a", () => t);');
  expect(js).toContain('return s.replaceAll("a", "$$&!");');
  expect(js).toContain('return s.replaceAll("a", "b");');
  expect(js).toContain('return s.replace("export default ", "let App = ");');
  expect(js).toContain('return s.replace("a", () => t);');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.all("xa", "$&$&"), lib.dollar("ba"), lib.plain("aa"), lib.first("export default A; export default B"), lib.firstOf("aa", "$1")])
    .toEqual(["x$&$&", "b$&!", "bb", "let App = A; export default B", "$1a"]);
  // A pattern that may be empty is the runtime's (ADR 0063), its replacement as it is too.
  expect([lib.by("xa", "a", "$&"), lib.by("ab", "", "$")]).toEqual(["x$&", "$a$b$"]);
});

// ADR 0066: a format string written across lines keeps them, as a string
// literal does, a template literal of the page react.dev's DownloadButton
// writes; one written with `\n` keeps those.
test("a format string written across lines keeps its lines", async () => {
  const dir = fixture("format-lines");
  writeFileSync(join(dir, "lib.rs"), `pub fn page(code: &str) -> String {
    format!(
        "<p>
{code}
</p>"
    )
}

pub fn escaped(code: &str) -> String {
    format!("<p>\\n{code}\\n</p>")
}

pub fn raw(code: &str) -> String {
    format!(
        r#"<a href="x">
{}
</a>"#,
        code
    )
}

pub fn continued(code: &str) -> String {
    format!("<p>\\n{code}\\
        </p>")
}

pub fn logged(code: &str) {
    println!(
        "<p>
{code}
</p>"
    );
}

pub fn plain() -> String {
    format!(
        "<p>
</p>"
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return `<p>\n${code}\n</p>`;");
  expect(js).toContain("return `<p>\\n${code}\\n</p>`;");
  expect(js).toContain('return `<a href="x">\n${code}\n</a>`;');
  // A line a `\` continues isn't one: its `\n` is written so.
  expect(js).toContain("return `<p>\\n${code}</p>`;");
  expect(js).toContain("console.log(`<p>\n${code}\n</p>`);");
  expect(js).toContain("return `<p>\n</p>`;");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.page("x"), lib.escaped("x"), lib.raw("x")]).toEqual(["<p>\nx\n</p>", "<p>\nx\n</p>", '<a href="x">\nx\n</a>']);
});

// A RegExp of a pattern and flags written as they are is a literal, as JS
// writes one, `/%s/g` (ADR 0243); one JS can't parse is made as it was,
// to throw when it runs.
test("a RegExp of a pattern as it is written is a literal", async () => {
  const dir = fixture("regex-literals");
  writeFileSync(join(dir, "lib.rs"), `use js::reg_exp;

pub fn placeholders(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("%s", "g"), "_")
}

// A literal's \`/\` is escaped, but in a class, where it needn't be.
pub fn slashes(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("a/b|[/]", "g"), "-")
}

// An empty pattern's literal is \`/(?:)/\`: \`//\` is a comment.
pub fn emptied(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("", ""), "^")
}

// A line break can't be in a literal.
pub fn joined(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("\\n", "g"), " ")
}

// One JS can't parse throws as it's made, not as the module is read.
pub fn unparsed() -> bool {
    reg_exp::new("(", "").test("(")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('text.replace(/%s/g, "_")');
  expect(js).toContain('text.replace(/a\\/b|[/]/g, "-")');
  expect(js).toContain('text.replace(/(?:)/, "^")');
  expect(js).toContain('new RegExp("\\n", "g")');
  expect(js).toContain('new RegExp("(", "")');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.placeholders("%s and %s")).toBe("_ and _");
  expect(lib.slashes("a/b /")).toBe("- -");
  expect(lib.emptied("x")).toBe("^x");
  expect(lib.joined("a\nb")).toBe("a b");
  expect(() => lib.unparsed()).toThrow(SyntaxError);
});

// A dictionary of entries written out, each key a string, is an object
// literal of them, as react.dev's RSC template writes its files: the same
// properties, in the same order. Not of a `__proto__` key, which a literal
// makes the object's prototype, nor of a key given twice, nor of a value
// `undefined`, which an object's field leaves out (ADR 0280). A key is any
// text, a `&str` too, as `HashMap::from` takes one, a comment between
// entries kept.
test("a dictionary of entries written out is an object literal", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("dict-literal");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, dict};
pub fn files(code: &str) -> &'static Dict<String> {
    dict::from_entries(vec![
        ("/index.html".to_string(), code.to_string()),
        ("main".to_string(), "m".to_string()),
    ])
}
pub fn twice(code: &str) -> &'static Dict<String> {
    dict::from_entries(vec![("a".to_string(), code.to_string()), ("a".to_string(), "again".to_string())])
}
pub fn proto(code: &str) -> &'static Dict<String> {
    dict::from_entries(vec![("__proto__".to_string(), code.to_string())])
}
pub fn missing() -> &'static Dict<Option<String>> {
    dict::from_entries(vec![("a".to_string(), None)])
}
pub fn named(name: String, code: &str) -> &'static Dict<String> {
    dict::from_entries(vec![(name, code.to_string())])
}
// validAriaProperties, its keys text written out, as HashMap::from takes them.
pub fn aria() -> &'static Dict<u8> {
    dict::from_entries(vec![
        ("aria-current", 0), // state
        // Widget Attributes
        ("role", 0),
    ])
}
pub fn borrowed(name: &String) -> &'static Dict<u8> {
    dict::from_entries(vec![(name, 1)])
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return { "/index.html": code, main: "m" };');
  expect(js).toContain('return Object.fromEntries([\n    ["a", code],\n    ["a", "again"],\n  ]);');
  expect(js).toContain('return Object.fromEntries([["__proto__", code]]);');
  expect(js).toContain("return Object.fromEntries([[name, code]]);");
  expect(js).toContain('return Object.fromEntries([["a", undefined]]);');
  const lib = await import(join(dir, "lib.js"));
  expect(Object.entries(lib.files("c"))).toEqual([["/index.html", "c"], ["main", "m"]]);
  expect(Object.keys(lib.proto("c"))).toEqual(["__proto__"]);
  expect(Object.keys(lib.missing())).toEqual(["a"]);
  expect(js).toContain('return {\n    "aria-current": 0, // state\n    // Widget Attributes\n    role: 0,\n  };');
  expect(js).toContain("return Object.fromEntries([[name, 1]]);");
  expect([lib.aria(), lib.borrowed("x")]).toEqual([{ "aria-current": 0, role: 0 }, { x: 1 }]);
});

// ADR 0238: a `String` kept for good, `.leak()`, is the string itself, as
// react.dev's Page gives Seo the image it makes for a component's
// `'static` props: JS frees nothing itself.
test("a leaked String is the string", async () => {
  const dir = fixture("string-leak");
  writeFileSync(join(dir, "lib.rs"), `pub fn named(n: u32) -> &'static str {
    format!("item-{n}").leak()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return `item-${n}`;");
  const { named } = await import(join(dir, "lib.js"));
  expect(named(3)).toBe("item-3");
});

// `s[a..b].make_ascii_uppercase()` gives `s` its text with that part
// changed (ADR 0334), not a handle on a part.
test("a string's part changed in place gives the string its text", async () => {
  const dir = fixture("str-part-case");
  writeFileSync(join(dir, "lib.rs"), `pub fn shout(mut name: String) -> String {
    name[2..4].make_ascii_uppercase();
    name[..].make_ascii_lowercase();
    name[2..].make_ascii_uppercase();
    name
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("name = $asciiCase(name, true, 2, 4);");
  expect(js).toContain("name = $asciiCase(name, false);");
  expect(js).toContain("name = $asciiCase(name, true, 2);");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.shout("élan Vital")).toBe("éLAN VITAL");
});

// ADR 0054, from MIR (ADR 0364): a writer returns the string it writes,
// `return s` of each way through that writes once, `let f = s` of a first
// write; a derived impl's parameters named as the program's are.
test("a writer from its MIR returns the string it writes", async () => {
  const dir = fixture("mir-writers");
  writeFileSync(join(dir, "lib.rs"), `use std::fmt;

#[derive(Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub enum Shape {
    Dot,
    Line(u32),
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Shape::Dot => write!(f, "a dot"),
            Shape::Line(n) => write!(f, "a line of {n}"),
        }
    }
}

pub struct List(pub Vec<u32>);

impl fmt::Display for List {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (i, x) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{x}")?;
        }
        write!(f, "]")
    }
}

pub fn shown(p: &Point, s: &Shape, l: &List) -> String {
    format!("{p:?} {s} {l}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function pointDebug_fmt(point) {\n  return `Point { x: ${point.x}, y: ${point.y} }`;\n}");
  expect(js).toContain('function shapeDisplay_fmt(shape) {\n  if (shape === "Dot") {\n    return "a dot";\n  }\n  return `a line of ${shape._0}`;\n}');
  expect(js).toContain('function listDisplay_fmt(list) {\n  let f = "[";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown({ x: 1, y: 2 }, { TAG: "Line", _0: 3 }, [[1, 2]]), lib.shown({ x: 1, y: 2 }, "Dot", [[]])]).toEqual(["Point { x: 1, y: 2 } a line of 3 [1, 2]", "Point { x: 1, y: 2 } a dot []"]);
});

// ADRs 0058, 0137, from MIR: a writer takes its `Formatter`'s options, and a
// derived `Debug` gives them to each field.
test("a writer from its MIR takes its Formatter's options", async () => {
  const dir = fixture("mir-writer-options");
  writeFileSync(join(dir, "lib.rs"), `use std::fmt;

#[derive(Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub struct Name(pub String);

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(&self.0)
    }
}

pub fn shown(p: &Point, n: &Name) -> String {
    format!("{p:#?}|{n:>6}|{n:<4}|")
}

`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function pointDebug_fmt(point, options) {\n  const shown = $formatted(String(point.x), options, true);");
  expect(js).toContain("function nameDisplay_fmt(name, options) {\n  return $formatted(name[0], options);\n}");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.shown({ x: 1, y: 2 }, ["ab"])).toBe("Point {\n    x: 1,\n    y: 2,\n}|    ab|ab  |");
});

// ADRs 0137, 0143, from MIR: `f.alternate()` and `f.width()` are what the
// writer's `Formatter` was given.
test("a writer from its MIR answers what its Formatter is asked", async () => {
  const dir = fixture("mir-writer-questions");
  writeFileSync(join(dir, "lib.rs"), `use std::fmt;

pub struct Size(pub u32);

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{} bytes", self.0)
        } else {
            match f.width() {
                Some(w) => write!(f, "{}B/{w}", self.0),
                None => write!(f, "{}B", self.0),
            }
        }
    }
}

pub fn shown(s: &Size) -> String {
    format!("{s} {s:#} {s:7}")
}

`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (options?.alternate === true) {\n    return `${size[0]} bytes`;\n  } else {\n    const w = options?.width;");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.shown([5])).toBe("5B 5 bytes 5B/7");
});

// ADR 0136, from MIR: a `Debug` builder's chain is the string it writes, its
// `entries(v.iter())` all of `v`; a writer writing once each way returns it.
test("a writer's Debug builder from its MIR is the string its chain writes", async () => {
  const dir = fixture("mir-writer-builders");
  writeFileSync(join(dir, "lib.rs"), `use std::fmt;

pub struct P {
    pub x: i32,
}

impl fmt::Debug for P {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("P").field("x", &self.x).finish()
    }
}

pub struct Bag(pub Vec<u32>);

impl fmt::Debug for Bag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entry(&0).entries(self.0.iter()).finish()
    }
}

pub struct Mark(pub bool);

impl fmt::Display for Mark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let shown = if self.0 { "yes" } else { "no" };
        if self.0 {
            f.write_str(shown)
        } else {
            f.write_str("-")
        }
    }
}

pub struct Flag(pub bool);

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 {
            f.write_str("y")?;
        } else {
            f.write_str("n")?;
        }
        Ok(())
    }
}

pub fn shown(p: &P, b: &Bag, m: &Mark) -> String {
    format!("{p:?} {b:?} {m} {} {}", Flag(true), Flag(false))
}

`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function pDebug_fmt(p) {\n  return `P { x: ${p.x} }`;\n}");
  expect(js).toContain('return `[${["0"].concat(Array.from(bag[0], (entry) => String(entry))).join(", ")}]`;');
  expect(js).toContain('  if (mark[0]) {\n    return shown;\n  } else {\n    return "-";\n  }\n}');
  const lib = await import(join(dir, "lib.js"));
  expect(js).toContain('function flagDisplay_fmt(flag) {\n  if (flag[0]) {\n    return "y";\n  } else {\n    return "n";\n  }\n}');
  expect(lib.shown({ x: 1 }, [[2, 3]], [true])).toBe("P { x: 1 } [0, 2, 3] yes y n");
});

test("a `format!`'s arguments read out of order, and a string's `chars()` iterated, from MIR read as written", async () => {
  const dir = fixture("mir-format-chars");
  writeFileSync(join(dir, "lib.rs"), `pub fn lines(code: &str) -> Vec<String> {
    let mut line_output: Vec<String> = Vec::new();
    let mut final_output = Vec::new();
    let mut line_index = 0;
    for c in code.chars() {
        if c == '\\n' {
            let current = line_index;
            final_output.push(format!("{current}:{}", line_output.join("")));
            line_output = Vec::new();
            line_index += 1;
        } else {
            line_output.push(c.to_string());
        }
    }
    final_output
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  for (const c of code) {\n");
  expect(js).toContain('      const arg = lineOutput.join("");\n      finalOutput.push(`${current}:${arg}`);\n');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.lines("ab\ncd\n")).toEqual(["0:ab", "1:cd"]);
});

// \`replacen\` of any count, as Rust's: the first \`n\` matches, an empty
// pattern's at each \`char\`'s boundary.
test("replacen replaces as many as it's told", async () => {
  const dir = fixture("replacen-count");
  writeFileSync(join(dir, "lib.rs"), `pub fn two(s: &str) -> String {
    s.replacen('l', "_", 2)
}
pub fn many(s: &str, n: usize) -> String {
    s.replacen("ab", "X", n)
}
pub fn empty(s: &str) -> String {
    s.replacen("", "-", 2)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.two("hello world"), lib.many("ababab", 0), lib.many("ababab", 2), lib.many("ababab", 9), lib.empty("héé")]).toEqual(["he__o world", "ababab", "XXab", "XXX", "-h-éé"]);
});

// What's left of a string's \`chars()\` once stepped, \`as_str()\`, from MIR:
// the rest of it, \`$restStr(c)\`.
test("the rest of a string's chars from MIR is a string", async () => {
  const dir = fixture("mir-chars-rest");
  writeFileSync(join(dir, "lib.rs"), `pub fn cap(w: &str) -> String {
    let mut c = w.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
// Stepped from its end, and not stepped at all.
pub fn init(w: &str) -> String {
    let mut c = w.chars();
    c.next_back();
    c.as_str().to_string()
}
pub fn whole(w: &str) -> String {
    w.chars().as_str().to_string()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("$restStr(c)");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.cap("hello"), lib.cap(""), lib.cap("é!")]).toEqual(["Hello", "", "É!"]);
  expect([lib.init("abc"), lib.init(""), lib.whole("abc")]).toEqual(["ab", "", "abc"]);
  expect(js).toContain('  return Array.from(w).join("");\n');
});

// `{:?}` of a value it reads more than once, a `Result`'s tag and what it
// holds, given as it's made: held first, as a value read twice is
// everywhere else, not a function called in place, `((result) => ..)(f())`.
// Not past a call before it, whose order it would change.
test("a value `{:?}` reads twice is held first, not given to a function called in place", async () => {
  const dir = fixture("mir-applied-values");
  writeFileSync(join(dir, "lib.rs"), `fn parse(s: &str) -> Result<u8, String> { s.parse::<u8>().map_err(|_| s.to_string()) }
fn bump(n: &mut u8) -> Option<u8> { *n += 1; Some(*n) }
pub fn main() {
    let first = parse("1");
    println!("{:?} {:?}", first, parse("x"));
    let mut n = 1;
    println!("{} {:?}", n.clone(), bump(&mut n));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('const value = parse("x");');
  expect(js).not.toContain("((result) =>");
});

// ADR 0652: a `String` appended to twice, one after the other, is one `+=`
// of the two joined by `+`, as React's DefaultPrepareStackTraceV8 writes
// `stack += '\n    at ' + structuredStackTrace[i].toString();`. Its JS was
//
//   for (const callSite of structuredStackTrace) {
//     stack += '\n    at ';
//     stack += callSite.toString();
//   }
//
// Two pieces of text written out are one. Not where the second reads the
// string, which sees the first appended; nor a number's, whose `+` rounds
// at each step; nor two strings' appends, one each; and two values, each
// with its text, a line each.
test("consecutive appends to a string are one +=", async () => {
  const dir = fixture("consecutive-appends");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use js::JsError;
use node::CallSite;

pub fn prepare_stack_trace(error: &JsError, structured_stack_trace: &[&CallSite]) -> String {
    let mut name = error.name();
    if name.is_empty() {
        name = "Error".to_string();
    }
    let message = error.message();
    let mut stack = name + ": " + &message;
    for call_site in structured_stack_trace {
        stack += "\\n    at ";
        stack += &call_site.to_string();
    }
    stack
}

pub fn asked(x: &str) -> String {
    let mut t = x.trim().to_string();
    t.push('!');
    t.push('?');
    t.push_str(x);
    t
}

pub fn doubled(t: &str) -> String {
    let mut s = t.to_string();
    s.push('a');
    s.push_str(&s.clone());
    s
}

pub fn stepped(x: f64) -> f64 {
    let mut f = x;
    f += 1.0;
    f += 1.0;
    f
}

pub fn lines(a: &str, b: &str) -> String {
    let mut out = String::new();
    out += a;
    out += "\\n";
    out += b;
    out += "\\n";
    out
}

pub fn paired(x: &str) -> String {
    let mut a = String::new();
    let mut b = String::new();
    a.push_str(x);
    b.push('-');
    format!("{b}|{a}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "--extern", `js=${join(node, "libjs.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`  let stack = name + ": " + message;
  for (const callSite of structuredStackTrace) {
    stack += "\\n    at " + callSite.toString();
  }
`);
  expect(js).toContain('  let t = $trim(x);\n  t += "!?" + x;\n');
  expect(js).toContain('  s += "a";\n  s += s;\n');
  expect(js).toContain("  f++;\n  f++;\n");
  // Of a value each, a line each.
  expect(js).toContain('  out += a + "\\n";\n  out += b + "\\n";\n');
  const lib = await import(join(dir, "lib.js"));
  const sites = [{ toString: () => "f (a.js:1)" }, { toString: () => "g (b.js:2)" }];
  expect(lib.prepareStackTrace({ name: "", message: "m" }, sites)).toBe("Error: m\n    at f (a.js:1)\n    at g (b.js:2)");
  expect([lib.asked(" a "), lib.doubled("x"), lib.stepped(1e16), lib.paired("x"), lib.lines("a", "b")]).toEqual([
    "a!? a ",
    "xaxa",
    1e16,
    "-|x",
    "a\nb\n",
  ]);
  // V8's own hook, in Node, of its call sites, an error's name its own or
  // none.
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { prepareStackTrace } from "./lib.js";

Error.prepareStackTrace = prepareStackTrace;
const named = new TypeError("boom").stack.split("\\n");
const error = new Error("plain");
error.name = "";
const unnamed = error.stack.split("\\n");
console.log(JSON.stringify({ named: named[0], at: named[1].startsWith("    at "), unnamed: unnamed[0] }));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(JSON.parse(p.stdout.toString())).toEqual({ named: "TypeError: boom", at: true, unnamed: "Error: plain" });
});

// ADR 0632, amended: a string's count of UTF-16 units, `s.encode_utf16()
// .count()`, is its `length`, which counts them, as React's
// `getIODescription` tests `value.length < 5`; it was
// `Iterator.from($encodeUtf16(s)).reduce((n) => n + 1, 0)`. And it's a JS
// string's length to the ranges, as `js::string::length` is, so a counter
// below it doesn't wrap. `chars().count()` counts code points, which
// `length` doesn't, and `len()` bytes.
test("a string's count of UTF-16 units is its length", async () => {
  const dir = fixture("utf16-count");
  writeFileSync(join(dir, "lib.rs"), `pub fn units(s: &str) -> usize {
    s.encode_utf16().count()
}

pub fn points(s: &str) -> usize {
    s.chars().count()
}

pub fn bytes(s: &str) -> usize {
    s.len()
}

pub fn useful(text: &str) -> bool {
    !(text.encode_utf16().count() < 5 || text.encode_utf16().count() > 500)
}

pub fn owned(text: String) -> usize {
    text.encode_utf16().count()
}

pub fn pairs(s: &str) -> u32 {
    let mut n = 0;
    let mut i = 0;
    while i + 1 < s.encode_utf16().count() {
        if i % 3 != 0 {
            i += 1;
            continue;
        }
        i += 1;
        n += 1;
        i += 1;
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function units(s) {\n  return s.length;\n}");
  expect(js).toContain("export function points(s) {\n  return Array.from(s).length;\n}");
  expect(js).toContain("export function bytes(s) {\n  return $byteLen(s);\n}");
  expect(js).toContain("return !(text.length < 5 || text.length > 500);");
  expect(js).toContain("export function owned(text) {\n  return text.length;\n}");
  expect(js).toContain("  while (i + 1 < s.length) {\n");
  expect(js).toContain("    i++;\n");
  expect(js).not.toContain("$encodeUtf16");
  writeFileSync(join(dir, "run.mjs"), `import { units, points, bytes, useful, owned, pairs } from "./lib.js";
const texts = ["", "hi", "hello", "h\\u00e9llo", "\\u{1F600}\\u{1F600}\\u{1F600}", "ab\\u{1F600}", "x".repeat(501)];
console.log(JSON.stringify(texts.map((t) => [units(t), points(t), bytes(t), useful(t), owned(t), pairs(t)])));
`);
  installRuntime(dir);
  const p = Bun.spawnSync(["node", "run.mjs"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(JSON.parse(p.stdout.toString())).toEqual([
    [0, 0, 0, false, 0, 0],
    [2, 2, 2, false, 2, 1],
    [5, 5, 5, true, 5, 2],
    [5, 5, 6, true, 5, 2],
    [6, 3, 12, true, 6, 2],
    [4, 3, 6, false, 4, 1],
    [501, 501, 501, false, 501, 167],
  ]);
});

// A string's `repeat(n)` is JS's own, as React's
// ReactPerformanceTrackProperties writes `prefix + '\xa0\xa0'.repeat(indent)`:
// no room is checked first. A JS string is less than `isize::MAX` bytes, so
// a repeat std refuses, `capacity overflow`, the engine refuses first, a
// `RangeError` (ADR 0774, amended; a difference the guide lists). A `Vec`'s
// room is checked as std checks it still.
test("a string's repeat is JS's own, and one too long is JS's RangeError", async () => {
  const dir = fixture("string-repeat");
  writeFileSync(join(dir, "lib.rs"), `pub fn indented(prefix: &str, indent: usize, name: &str) -> String {
    prefix.to_string() + &"\\u{a0}\\u{a0}".repeat(indent) + name
}

pub fn padded(text: &str, start: usize) -> usize {
    format!("{text}!").repeat(start).len()
}

pub fn room(n: usize) -> usize {
    Vec::<u32>::with_capacity(n).len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  return prefix + "\\xa0\\xa0".repeat(indent) + name;\n');
  expect(js).toContain("  return $byteLen(`${text}!`.repeat(start));\n");
  expect(js).toContain("  $roomFor(n, 4);\n");
  expect(js.match(/\$roomFor\(/g)).toHaveLength(1);
  installRuntime(dir);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.indented("-", 2, "k"), lib.padded("ab", 2)]).toEqual(["-\xa0\xa0\xa0\xa0k", 6]);
  // Past the engine's string length: its RangeError, where std panics.
  expect(() => lib.padded("ab", 2 ** 30)).toThrow(RangeError);
});

// ADR 0774, amended: a signed count of written text's `repeat`, as React's
// hydration diff indents, `'  '.repeat(indent)`: a negative one throws
// JS's `RangeError` as its `usize`, 2^31 or more, does, where std panics.
// Of text not written there, which may be empty, and of an integer wider
// than 32 bits, it's cast.
test("a repeat's count of a signed integer is the integer", async () => {
  const dir = fixture("string-repeat-signed");
  writeFileSync(join(dir, "lib.rs"), `pub fn indentation(indent: i32) -> String {
    "  ".to_owned() + &"  ".repeat(indent as usize)
}

pub fn of_text(text: &str, n: i8) -> String {
    text.repeat(n as usize)
}

pub fn wide(n: i64) -> String {
    "ab".repeat(n as usize)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  return "  " + "  ".repeat(indent);\n');
  expect(js).toContain("  return text.repeat(n >>> 0);\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.indentation(1), lib.ofText("", -1), lib.wide(2n ** 32n + 1n)]).toEqual(["    ", "", "ab"]);
  expect(() => lib.indentation(-1)).toThrow(RangeError);
});

// ADR 0945: a sort by UTF-16 units, `a.encode_utf16().cmp(b.encode_utf16())`,
// is JS's `sort()` (test/corpus/utf16_sort.rs runs it against native
// Rust); a comparator of the two the other way, or of something else of
// their units, stays a comparator.
test("only a sort by the first's units then the second's is JS's sort()", async () => {
  const dir = fixture("utf16-sort-others");
  writeFileSync(join(dir, "lib.rs"), `use std::cmp::Ordering;
use std::str::EncodeUtf16;

pub fn ascending(mut words: Vec<&str>) -> Vec<&str> {
    words.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    words
}

pub fn descending(mut words: Vec<&str>) -> Vec<&str> {
    words.sort_by(|a, b| b.encode_utf16().cmp(a.encode_utf16()));
    words
}

pub fn by_last(mut words: Vec<&str>) -> Vec<&str> {
    words.sort_by(|a, b| last_unit(a.encode_utf16(), b.encode_utf16()));
    words
}

fn last_unit(a: EncodeUtf16, b: EncodeUtf16) -> Ordering {
    a.last().cmp(&b.last())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  words.sort();\n");
  expect(js.match(/words\.sort\(\);/g)).toHaveLength(1);
  installRuntime(dir);
  const lib = await import(join(dir, "lib.js"));
  const words = ["b\u{1F600}", "b\u{FF61}", "a", "b\u{E000}", "", "b\u{10000}", "ab", "B", "ca"];
  const ascending = ["", "B", "a", "ab", "b\u{10000}", "b\u{1F600}", "b\u{E000}", "b\u{FF61}", "ca"];
  expect(lib.ascending([...words])).toEqual(ascending);
  expect(lib.descending([...words])).toEqual([...ascending].reverse());
  expect(lib.byLast([...words])).toEqual(["", "B", "a", "ca", "ab", "b\u{10000}", "b\u{1F600}", "b\u{E000}", "b\u{FF61}"]);
});
