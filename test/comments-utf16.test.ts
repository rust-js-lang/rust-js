// Comments placed in a JS line that has non-ASCII text before them (#1668).
// The source map's columns are UTF-16 units, as JS counts a string, where a
// line's text is placed by its bytes: each place in a line is its byte, and
// the map's columns are read and written only through the line's text
// (`MapCol`, ADR 0630, amended). Each case is one of the places where the
// two were mixed, so the comment went inside a string, after another
// argument, or nowhere.
import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { decodeMappings } from "./sourcemap";
import { compiler, fixture, run } from "./support";

/** `rust` compiled in a fixture of `name`: its JS, its module and its map. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown>; map: { mappings: string } }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")), map: JSON.parse(readFileSync(join(dir, "lib.js.map"), "utf8")) };
}

/** Where `code` is first in `text`, as the map counts: UTF-16 units. */
function place(text: string, code: string): [number, number] {
  const lines = text.split("\n");
  const line = lines.findIndex(l => l.includes(code));
  return [line, lines[line].indexOf(code)];
}

// A block comment after an argument, the issue's: it was written inside
// the template's string, or after the first argument, its bytes taken for
// the map's units.
test("a comment after an argument is after it, past non-ASCII text", async () => {
  const { js, lib } = await compiled("utf16-after-an-argument", `fn f(a: &str, x: i32, b: &str) -> String {
    format!("{a}{x}{b}")
}

pub fn shown(x: i32) -> String {
    format!("[{}]", f("日é", x /* c */, "abc"))
}

pub fn counted(x: i32) -> String {
    f("日本語のテキスト", x /* the count */, "it's")
}
`);
  expect(js).toContain('  return `[${f("日é", x /* c */, "abc")}]`;\n');
  expect(js).toContain('  return f("日本語のテキスト", x /* the count */, "it\'s");\n');
  expect([lib.shown(1 as never), lib.counted(2 as never)]).toEqual(["[日é1abc]", "日本語のテキスト2it's"]);
});

// A conditional's branch's comments, before its value (ADR 0658): the
// operator was looked for before the map's column taken as a byte, short
// of it, so the comments were left out.
test("a conditional's branch's comments are before its value, past non-ASCII text", async () => {
  const rust = `pub fn label(name: &str, flag: bool) -> &'static str {
    if name == "日本語" && flag {
        // the first
        "yes"
    } else {
        // the second
        "no"
    }
}
`;
  const { js, lib, map } = await compiled("utf16-branch-comments", rust);
  expect(js).toContain('  return name === "日本語" && flag\n    ? // the first\n      "yes"\n    : // the second\n      "no";\n');
  expect(lib.label("日本語" as never, true as never)).toBe("yes");
  // Each branch's value, on a line of its own after its comments, maps to
  // its Rust: its line's pieces are placed from their bytes, its column
  // the map's units of its piece before it.
  const segments = decodeMappings(map.mappings);
  for (const value of ['"yes"', '"no"']) {
    const [line, column] = place(js, value);
    const segment = segments.find(s => s.jsLine === line && s.jsCol === column);
    expect([value, segment?.srcLine, segment?.srcCol]).toEqual([value, ...place(rust, value)]);
  }
});

// A comment after a re-exported name (rd338): the end of the name was
// looked for from the map's column taken as a byte, so it was after the
// whole statement.
test("a comment after a re-exported name is after it, past non-ASCII text", async () => {
  const { js } = await compiled("utf16-after-a-name", `mod inner {
    pub fn ä() -> u32 {
        1
    }
    pub fn b() -> u32 {
        2
    }
}

pub use inner::{
    ä,
    b, // the second
};
`);
  expect(js).toContain('export {\n  ä,\n  b, // the second\n} from "./inner.js";\n');
});

// The JS after a comment inside its line maps to its Rust, the comment's
// width further: where the comment is was its byte, compared with the
// map's column, so what was between them wasn't moved.
test("the JS after a comment inside its line maps to its Rust, past non-ASCII text", async () => {
  const rust = `fn f(a: &str, x: i32, y: i32) -> String {
    format!("{a}{x}{y}")
}

pub fn shown(x: i32, y: i32) -> String {
    f("日本", x /* 二つ */, y)
}
`;
  const { js, map } = await compiled("utf16-map-after-a-comment", rust);
  expect(js).toContain('  return f("日本", x /* 二つ */, y);\n');
  const segments = decodeMappings(map.mappings);
  // `y`, after the comment: in each, 4 past the comment's `*/`.
  const [line, column] = place(js, "*/, y)");
  const after = segments.find(s => s.jsLine === line && s.jsCol === column + 4);
  const [srcLine, srcCol] = place(rust, "*/, y)");
  expect([after?.srcLine, after?.srcCol]).toEqual([srcLine, srcCol + 4]);
});
