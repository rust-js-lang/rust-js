// The js crate's `string` module: a JS string's own methods, where Rust's
// `str` has none that does what JS's does.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// `localeCompare`, an `Ordering`, as an Intl collator gives -1, 0 or 1: a
// sort's comparator, as react.dev's LanguageList sorts languages by name.
test("a string's localeCompare is a sort's comparator", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("locale-compare");
  writeFileSync(join(dir, "lib.rs"), `use js::string;

pub fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort_by(|a, b| string::locale_compare(a, b));
    names
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("names.sort((a, b) => a.localeCompare(b));");
  expect((await import(join(dir, "lib.js"))).sorted(["Ébène", "zèbre", "abeille", "Zoo"])).toEqual(["abeille", "Ébène", "zèbre", "Zoo"]);
});

// rd161: `toLocaleLowerCase` and `toLocaleUpperCase` are a locale's, which
// `to_lowercase` isn't: Turkish lowers `I` to a dotless `ı`. React DOM's
// validateDOMNesting lowers a tag so; with no locales, the host's, left out.
test("a string's toLocaleLowerCase is its locale's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("locale-case");
  writeFileSync(join(dir, "lib.rs"), `use js::string;

pub fn lowered(tag: &str) -> String {
    string::to_locale_lower_case(tag, None)
}

pub fn turkish(text: &str) -> (String, String) {
    (
        string::to_locale_lower_case(text, Some(&["tr"])),
        string::to_locale_upper_case(text, Some(&["tr", "en"])),
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return tag.toLocaleLowerCase();\n");
  expect(js).toContain('[text.toLocaleLowerCase(["tr"]), text.toLocaleUpperCase(["tr", "en"])]');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.lowered("DIV")).toBe("div");
  expect(lib.turkish("Iİi")).toEqual(["ıii", "Iİİ"]);
});

// rd406: JS's `repeat` of a number, as React's Flight client indents by
// `' '.repeat(indent)`: its fraction dropped, a count below zero JS's
// `RangeError`, where Rust's `str::repeat` takes a `usize`, a cast.
test("a string's repeat takes a number, as JS's does", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("string-repeat");
  writeFileSync(join(dir, "lib.rs"), `use js::string;

pub fn indented(indent: f64, line: &str) -> String {
    string::repeat(" ", indent) + line
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain('  return " ".repeat(indent) + line;\n');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.indented(2, "a"), lib.indented(1.9, "b"), lib.indented(0, "c")]).toEqual(["  a", " b", "c"]);
  expect(() => lib.indented(-1, "d")).toThrow(RangeError);
});

// #1914: `toLowerCase` and `toUpperCase`, the host's Unicode tables, no
// locale's, for a faithful port of JS, beside `str::to_lowercase`, Rust's
// own, its toolchain's tables (ADR 1163): of a character a newer Unicode
// gave a case, U+0277 and U+A7DD, each answers its own.
test("a string's toLowerCase is the host's, str::to_lowercase Rust's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("host-case");
  writeFileSync(join(dir, "lib.rs"), `use js::string;

pub fn host(text: &str) -> (String, String) {
    (string::to_lower_case(text), string::to_upper_case(text))
}

pub fn rust(text: &str) -> (String, String) {
    (text.to_lowercase(), text.to_uppercase())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("  return [text.toLowerCase(), text.toUpperCase()];\n");
  const lib = await import(join(dir, "lib.js"));
  for (const text of ["ɷ", "꟝", "Ada"]) {
    expect(lib.host(text)).toEqual([text.toLowerCase(), text.toUpperCase()]);
  }
  // Rust's, as its std answers natively (#1888): U+A7DD lowers to U+0277,
  // U+0277 raises to U+A7DD.
  expect([lib.rust("꟝")[0], lib.rust("ɷ")[1]]).toEqual(["ɷ", "꟝"]);
});

test("a string's replaceAll uses JavaScript replacement templates", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("string-replace-all");
  writeFileSync(join(dir, "lib.rs"), `use js::string;

pub fn replace(text: &str, pattern: &str, replacement: &str) -> String {
    string::replace_all(text, pattern, replacement)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return text.replaceAll(pattern, replacement);");
  const lib = await import(join(dir, "lib.js"));
  const cases = [
    ["de_DE_POSIX", "_", "-"],
    ["a_a", "a", "$$-$&-$`-$'"],
    ["😀", "", "."],
    ["a.b.", ".", "-"],
  ] as const;
  for (const [text, pattern, replacement] of cases) {
    expect(lib.replace(text, pattern, replacement)).toBe(text.replaceAll(pattern, replacement));
  }
});
