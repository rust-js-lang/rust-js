// A function's last parameter marked `#[rust_js::rest]`, a slice, is JS's
// rest parameter (ADR 0664): `function formatProdErrorMessage(code,
// ...args)`, which JS calls with any number of arguments, as React's build
// calls it, and the crate's own code with its slice spread. Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// React's `shared/formatProdErrorMessage.js`, as upstream writes it.
const upstream = `export default function formatProdErrorMessage(code) {
  let url = 'https://react.dev/errors/' + code;
  if (arguments.length > 1) {
    url += '?args[]=' + encodeURIComponent(arguments[1]);
    for (let i = 2; i < arguments.length; i++) {
      url += '&args[]=' + encodeURIComponent(arguments[i]);
    }
  }
  return (
    \`Minified React error #\${code}; visit \${url} for the full message or \` +
    'use the non-minified dev environment for full errors and additional ' +
    'helpful warnings.'
  );
}
`;

test("a function's rest slice is JS's rest parameter, and a call spreads the slice it's given", async () => {
  const dir = fixture("rest-parameters");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

js::export_default! {
    fn format_prod_error_message(
        code: f64,
        #[cfg_attr(rust_js, rust_js::rest)] args: &[Option<&Unknown>],
    ) -> String {
        let mut url = format!("https://react.dev/errors/{code}");
        for (i, arg) in args.iter().enumerate() {
            url += if i == 0 { "?args[]=" } else { "&args[]=" };
            url += &js::encode_uri_component(&js::string(*arg));
        }
        format!(
            "Minified React error #{code}; visit {url} for the full message or \\
             use the non-minified dev environment for full errors and additional \\
             helpful warnings."
        )
    }
}

pub fn total(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> f64 {
    xs.iter().sum()
}

// Named as any function is.
#[cfg_attr(rust_js, rust_js::name = "countAll")]
pub fn count(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> usize {
    xs.len()
}

pub struct Point {
    pub x: f64,
}

impl Point {
    pub fn moved(&self, #[cfg_attr(rust_js, rust_js::rest)] by: &[f64]) -> f64 {
        self.x + total(by)
    }
}

fn apply(f: impl Fn(&[f64]) -> f64, v: &[f64]) -> f64 {
    f(v)
}

pub fn sums(v: &[f64]) -> Vec<f64> {
    let pointer: fn(&[f64]) -> f64 = total;
    let named = total;
    vec![
        total(&[1.0, 2.0]),
        total(v),
        total(&v[1..]),
        total(&[]),
        Point { x: 10.0 }.moved(&[1.0, v[0]]),
        apply(total, v),
        pointer(v),
        named(&[v[1]]),
    ]
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("export default function formatProdErrorMessage(code, ...args) {");
  expect(out).toContain("export function total(...xs) {");
  expect(out).toContain("moved(point, ...by) {");
  expect(out).toContain("export function countAll(...xs) {");
  // Written out, its items; held, spread.
  expect(out).toContain("total(1, 2),");
  expect(out).toContain("total(...v),");
  expect(out).toContain("total(...$slice(v, 1)),");
  expect(out).toContain("total(),");
  expect(out).toContain("Point.moved({ x: 10 }, 1, $index(v, 0)),");
  // As a value, what's given it spread.
  expect(out).toContain("const pointer = (x) => total(...x);");
  expect(out).toContain("apply((x) => total(...x), v),");
  // A variable of it is it, called as it is.
  expect(out).toContain("total($index(v, 1)),");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export default function formatProdErrorMessage(code: number, ...args: (unknown | null | undefined)[]): string;");
  expect(declarations).toContain("export function total(...xs: number[]): number;");

  const lib = await import(join(dir, "lib.js"));
  writeFileSync(join(dir, "upstream.js"), upstream);
  const { default: original } = await import(join(dir, "upstream.js"));
  for (const args of [[], ["a b"], ["x", 2, undefined], [null, { toString: () => "&" }, true]]) {
    expect(lib.default(418, ...args)).toBe(original(418, ...args));
  }
  expect(lib.default(31, "object with keys {a, b}")).toBe(
    "Minified React error #31; visit https://react.dev/errors/31?args[]=object%20with%20keys%20%7Ba%2C%20b%7D for the full message or use the non-minified dev environment for full errors and additional helpful warnings.",
  );
  expect(lib.total()).toBe(-0);
  expect(lib.total(1, 2, 3)).toBe(6);
  expect(lib.countAll(1, 2, 3)).toBe(3);
  expect(lib.sums([4, 5])).toEqual([3, 9, 5, -0, 15, 9, 9, 5]);
});

// A library's function is called by another crate as its JS takes it: its
// manifest says its last parameter is a rest, which rustc's metadata of a
// parameter's attributes doesn't.
test("a library's rest function is called with the slice spread", async () => {
  const dir = fixture("rest-parameters-library");
  writeFileSync(join(dir, "dep.rs"), "pub fn total(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> f64 {\n    xs.iter().sum()\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`]);
  expect(readFileSync(join(dir, "dep", "lib.js"), "utf8")).toContain("export function total(...xs) {");
  writeFileSync(join(dir, "app.rs"), "pub fn sums(v: &[f64]) -> Vec<f64> {\n    let f: fn(&[f64]) -> f64 = dep::total;\n    vec![dep::total(&[1.0, 2.0]), dep::total(v), f(v)]\n}\n");
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir]);
  const out = readFileSync(join(dir, "app", "lib.js"), "utf8");
  expect(out).toContain("total(1, 2), total(...v)");
  const { sums } = await import(join(dir, "app", "lib.js"));
  expect(sums([4, 5])).toEqual([3, 9, 9]);
});

// Where it isn't JS's rest parameter, it's an error that says what to write.
for (const [name, source, message] of [
  ["not the last parameter", "pub fn f(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64], n: f64) -> f64 { xs.len() as f64 + n }",
    "`#[rust_js::rest]` is a function's last parameter, as JS's rest parameter is"],
  ["a Vec", "pub fn f(#[cfg_attr(rust_js, rust_js::rest)] xs: Vec<f64>) -> usize { xs.len() }",
    "`#[rust_js::rest]` is a slice's, `xs: &[T]`, not a `Vec<f64>`'s"],
  ["a slice of &mut", "pub fn f(#[cfg_attr(rust_js, rust_js::rest)] xs: &mut [f64]) { xs[0] = 1.0; }",
    "`#[rust_js::rest]` is a slice's, `xs: &[T]`, not a `&mut [f64]`'s"],
  ["an array", "pub fn f(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64; 2]) -> f64 { xs[0] }",
    "`#[rust_js::rest]` is a slice's, `xs: &[T]`, not a `&[f64; 2]`'s"],
  ["a closure's parameter", "pub fn f() -> usize { let g = |#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]| xs.len(); g(&[1.0]) }",
    "`#[rust_js::rest]` is a function's last parameter, not a closure's: a closure is given what its type says"],
  ["a trait method's parameter", "pub trait T { fn f(&self, xs: &[f64]) -> usize; }\npub struct S;\nimpl T for S { fn f(&self, #[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> usize { xs.len() } }",
    "`#[rust_js::rest]` is a function's last parameter, not a trait method's, which each impl's call gives alike"],
  ["a function given a dictionary", "pub fn f<T: Clone>(#[cfg_attr(rust_js, rust_js::rest)] xs: &[T]) -> Vec<T> { xs.to_vec() }\npub fn g() -> Vec<u32> { f(&[1, 2]) }",
    "a function whose last parameter is `#[rust_js::rest]` given a dictionary, a fact, a drop or its caller's location, which JS's rest parameter can't come before"],
  ["a pattern", "pub fn f(#[cfg_attr(rust_js, rust_js::rest)] &[..]: &[f64]) -> usize { 0 }",
    "`#[rust_js::rest]` is a parameter's name, `args: &[T]`, not a pattern's: JS's rest parameter is an array"],
  // What `js::import!` gives is called with the slice whole.
  ["imported", "pub fn total(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> f64 { xs.iter().sum() }\npub async fn later() -> f64 { js::import!(total).await(&[1.0]) }",
    "importing a function whose last parameter is `#[rust_js::rest]`"],
] as const) {
  test(`\`#[rust_js::rest]\` refused: ${name}`, () => {
    const dir = fixture(`rest-refused-${name.replaceAll(/\W+/g, "-")}`);
    writeFileSync(join(dir, "lib.rs"), source);
    const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
    expect(refused.exitCode).not.toBe(0);
    expect(refused.stderr.toString()).toContain(message);
  });
}

// A component is given its props, React's one object.
test("`#[rust_js::rest]` refused: a component's props", () => {
  const dir = fixture("rest-refused-component");
  writeFileSync(join(dir, "lib.rs"), "#![allow(non_snake_case)]\nuse react::{JSX, jsx};\n\npub fn Tags(#[cfg_attr(rust_js, rust_js::rest)] tags: &[&'static str]) -> JSX::Element {\n    jsx! { <p>{tags.len()}</p> }\n}\n");
  const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  expect(refused.exitCode).not.toBe(0);
  expect(refused.stderr.toString()).toContain("`#[rust_js::rest]` is a function's last parameter, not a component's, which React gives its props");
});
