// What the compiler writes of strings and formatting: a string across lines as
// a template literal, a replacement as Rust's, a RegExp literal, `format!`'s
// arguments, and a writer's `Formatter`. Each test is a program of its own,
// compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

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
// `undefined`, which an object's field leaves out (ADR 0280).
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
  expect(js).toContain("  if (options?.alternate === true) {\n    return `${size[0]} bytes`;\n  }\n  const w = options?.width;");
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
  expect(js).toContain('  if (mark[0]) {\n    return shown;\n  }\n  return "-";\n}');
  const lib = await import(join(dir, "lib.js"));
  expect(js).toContain('function flagDisplay_fmt(flag) {\n  if (flag[0]) {\n    return "y";\n  }\n  return "n";\n}');
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
