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
