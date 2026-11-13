// A CSS Module's binding, `#[rust_js::css_module]` (#1512, ADR 1133): a
// struct of the classes a program reads, each a getter, and the module's
// default import a `static` of it. Each compile reads the file and checks
// each getter's class is one it exports, so a misspelled or deleted class
// is a compile error at the getter naming the file's classes.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { parseRecord } from "./compiles";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** A crate binding `./site.module.css` as `Styles`, its getters `getters`,
 * with `css` the file's text, or none. */
function styles(getters: string, { css, linkName = "./site.module.css#default", more = "" }: { css?: string; linkName?: string; more?: string }) {
  const lib = `use std::marker::PhantomData;

use js::JsObject;

/// The classes of \`site.module.css\` the app reads.
#[cfg_attr(rust_js, rust_js::css_module)]
pub struct Styles(PhantomData<JsObject>);

impl Styles {
${getters}
}

unsafe extern "Rust" {
    #[link_name = "${linkName}"]
    pub safe static STYLES: &'static Styles;
}
${more}`;
  return { lib, css };
}

const getter = (name: string, key: string) => `    #[cfg_attr(rust_js, rust_js::link_name = "get ${key}")]
    pub fn ${name}(&self) -> String {
        unreachable!()
    }
`;

/** Compile `crate` in a fixture of its own, its JS written to `out` there:
 * the JS, or the compiler's errors. */
function compile(name: string, crate: { lib: string; css?: string }, out = "lib.js") {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), crate.lib);
  if (crate.css !== undefined) writeFileSync(join(dir, "site.module.css"), crate.css);
  const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, out), ...withJs], { cwd: root, stderr: "pipe" });
  const js = result.exitCode === 0 ? readFileSync(join(dir, out), "utf8") : "";
  return { dir, js, errors: result.stderr.toString(), exitCode: result.exitCode };
}

const read = `pub fn classes() -> String {
    format!("{} {}", STYLES.code_block(), STYLES.card_title())
}
`;

// Each class by its key, as the file exports it: a name that's no JS
// identifier, `card-title`, `10`, `a:b`, a quoted key; one that's a Rust
// keyword's, `type`, by its own name.
test("a CSS Module's classes a binding names are read of its default import", () => {
  const css = ".codeBlock { color: red }\n.card-title:hover { color: blue }\n.\\31 0, .a\\:b, .type { color: green }\n.unused {}\n";
  const getters = [getter("code_block", "codeBlock"), getter("card_title", "card-title"), getter("ten", "10"), getter("a_b", "a:b"), getter("r#type", "type")];
  const more = `${read}pub fn others() -> [String; 3] {\n    [STYLES.ten(), STYLES.a_b(), STYLES.r#type()]\n}\n`;
  const { js, errors } = compile("css-module-read", styles(getters.join(""), { css, more }));
  expect(errors).toBe("");
  expect(js).toContain('import STYLES from "./site.module.css";');
  expect(js).toContain('`${STYLES.codeBlock} ${STYLES["card-title"]}`');
  expect(js).toContain('[STYLES["10"], STYLES["a:b"], STYLES.type]');
});

test("a class the file doesn't export is a compile error at its getter naming the file's classes", () => {
  const css = ".codeBlock { color: red }\n.card-title { color: blue }\n";
  const { errors, exitCode } = compile("css-module-misspelled", styles(getter("code_block", "codeBlok") + getter("card_title", "card-title"), { css, more: read }));
  expect(exitCode).not.toBe(0);
  expect(errors).toContain("rust-js: `./site.module.css` exports no class `codeBlok`; its classes are `card-title` and `codeBlock`");
  expect(errors).toContain('rust_js::link_name = "get codeBlok"');
  expect(errors).not.toContain("card-title`;");
});

test("a CSS Module's binding is checked where no code reads it", () => {
  const { errors, exitCode } = compile("css-module-unread", styles(getter("gone", "gone"), { css: ".kept {}\n" }));
  expect(exitCode).not.toBe(0);
  expect(errors).toContain("exports no class `gone`; its classes are `kept`");
});

test("a CSS Module's file is what the build read, so a save of it checks again", () => {
  const crate = styles(getter("code_block", "codeBlock"), { css: ".codeBlock { color: red }\n" });
  const dir = fixture("css-module-record");
  const record = join(dir, "lib.d");
  writeFileSync(join(dir, "lib.rs"), crate.lib);
  writeFileSync(join(dir, "site.module.css"), crate.css!);
  const args = [compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs, `--emit=dep-info=${record}`];
  expect(Bun.spawnSync(args, { cwd: root, stderr: "pipe" }).stderr.toString()).toBe("");
  expect(parseRecord(readFileSync(record, "utf8")).read).toContain(join(dir, "site.module.css"));
  writeFileSync(join(dir, "site.module.css"), ".codeBlocks { color: red }\n");
  expect(Bun.spawnSync(args, { cwd: root, stderr: "pipe" }).stderr.toString()).toContain("exports no class `codeBlock`; its classes are `codeBlocks`");
});

// Found as a binding's JS file is: relative to the crate root's JS where
// it's written, or where it goes in source, beside the Rust.
test("a CSS Module beside the Rust is found where the JS is written elsewhere", () => {
  const { errors, exitCode } = compile("css-module-in-source", styles(getter("code_block", "codeBlock"), { css: ".codeBlock {}\n" }), "out/lib.js");
  expect([exitCode, errors]).toEqual([0, ""]);
});

test("a CSS Module's binding of no file, of another import, or not a getter is refused, saying what to write", () => {
  const missing = compile("css-module-missing", styles(getter("code_block", "codeBlock"), {}));
  expect(missing.errors).toContain("rust-js: the classes of `./site.module.css`, which `STYLES` reads, can't be said: there's no file");
  const plain = compile("css-module-plain-css", styles(getter("code_block", "codeBlock"), { css: ".codeBlock {}\n", linkName: "./site.css#default" }));
  expect(plain.errors).toContain(
    'rust-js: `STYLES` is a `Styles`, a CSS Module\'s classes, so it\'s the module\'s default import, a file of the app\'s, `#[link_name = "./site.module.css#default"]`, not `./site.css#default`',
  );
  const named = compile("css-module-named", styles(getter("code_block", "codeBlock"), { css: ".codeBlock {}\n", linkName: "./site.module.css#codeBlock" }));
  expect(named.errors).toContain("so it's the module's default import");
  const method = `    #[cfg_attr(rust_js, rust_js::link_name = "codeBlock")]
    pub fn code_block(&self) -> String {
        unreachable!()
    }
`;
  const called = compile("css-module-called", styles(method, { css: ".codeBlock {}\n" }));
  expect(called.errors).toContain(
    'rust-js: a CSS Module\'s binding reads its classes: `#[rust_js::link_name = "get codeBlock"]`, a getter of `&self`, or `"get []"` of a class named as the program runs, not `codeBlock`',
  );
  const enumerated = compile("css-module-enum", { lib: "#[cfg_attr(rust_js, rust_js::css_module)]\npub enum Styles {}\n" });
  expect(enumerated.errors).toContain("rust-js: `#[rust_js::css_module]` is of a binding's struct of a CSS Module's classes");
});

test("a class read by a name the program has is the dynamic read, unchecked", () => {
  const dynamic = `    #[cfg_attr(rust_js, rust_js::link_name = "get []")]
    pub fn class(&self, name: &str) -> Option<String> {
        unreachable!()
    }
`;
  const more = "pub fn named(name: &str) -> Option<String> {\n    STYLES.class(name)\n}\n";
  const { js, errors } = compile("css-module-dynamic", styles(getter("code_block", "codeBlock") + dynamic, { css: ".codeBlock {}\n", more }));
  expect(errors).toBe("");
  expect(js).toContain("STYLES[name]");
});

// What Lightning CSS doesn't export where css-loader does is refused, never
// a key list that's short (#1617); and a file it can't read says where.
test("a CSS Module of ICSS, of @value, or that doesn't parse is refused, naming why", () => {
  const binding = getter("primary", "primary");
  for (const [name, css, why] of [
    ["css-module-export", ":export { primary: red }\n.a { color: red }\n", "its `:export` is ICSS, whose names Lightning CSS, Next's CSS parser, doesn't export where webpack's css-loader does (#1617)"],
    ["css-module-value", ".a { color: red }\n@value primary: red;\n", "its `@value`, at line 2, column 7: Lightning CSS, Next's CSS parser, refuses it where webpack's css-loader exports its name (#1617)"],
    ["css-module-unparsed", ".a { color: red }\n.b { } }\n.c { }\n", "the classes of `./site.module.css`, which `STYLES` reads, can't be said: Lightning CSS, Next's CSS parser, can't read it, at line 2"],
  ]) {
    const { errors, exitCode } = compile(name, styles(binding, { css }));
    expect([name, exitCode]).not.toEqual([name, 0]);
    expect(errors).toContain(why);
  }
});

// `rust-js bindings ./site.module.css` prints the binding the check reads,
// once: compiled against the file it's of, it passes, as rustfmt writes it,
// and a class no Rust name says is left to a note.
test("`rust-js bindings` prints the binding its check passes, formatted", () => {
  const dir = fixture("css-module-printed");
  writeFileSync(join(dir, "site.module.css"), ".codeBlock { color: red }\n.card-title:hover {}\n.type {}\n.\\31 0 {}\n.a-b {}\n.aB {}\n@keyframes spin {}\n");
  const bindings = (specifier: string) =>
    Bun.spawnSync(["node", join(root, "tooling/cli.js"), "bindings", specifier], { cwd: dir, stderr: "pipe", env: { ...process.env, RUST_JS_COMPILER: compiler } });
  const printed = bindings("./site.module.css");
  expect(printed.stderr.toString()).toBe("");
  const binding = printed.stdout.toString();
  expect(binding).toContain('    #[cfg_attr(rust_js, rust_js::link_name = "get type")]\n    pub fn r#type(&self) -> String {');
  expect(binding).toContain("// No getter is written of `10`, `a-b`, `aB`:");
  writeFileSync(join(dir, "lib.rs"), `${binding}\npub fn classes() -> String {\n    format!("{} {}", STYLES.code_block(), STYLES.r#type())\n}\n`);
  const compiled = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], { cwd: root, stderr: "pipe" });
  expect(compiled.stderr.toString()).toBe("");
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("`${STYLES.codeBlock} ${STYLES.type}`");
  const formatted = Bun.spawnSync(["rustfmt", "--edition", "2024", "--check", join(dir, "lib.rs")], { stderr: "pipe" });
  expect([formatted.exitCode, formatted.stdout.toString()]).toEqual([0, ""]);
  // A plain stylesheet, and a path not as a binding names it.
  writeFileSync(join(dir, "site.css"), ".plain {}\n");
  for (const specifier of ["./site.css", "site.module.css"]) {
    expect(bindings(specifier).stderr.toString()).toContain(
      `\`${specifier}\` isn't a CSS Module of the app's: name it as its binding does, from the crate root's JS, \`./site.module.css\``,
    );
  }
});
