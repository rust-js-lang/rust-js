// The order of standard output's and standard error's writes, which the
// corpus, keeping the two streams apart, can't tell: each case here run
// natively and as JS under Node with stderr written into stdout, as a
// terminal shows them, the bytes the same (#1490, ADR 1130). Rust's stdout
// is a `LineWriter`, holding a line till its newline; its stderr is
// unbuffered, each piece written as it's formatted. And as a browser runs
// it, where there's no stream, each stream's lines given its console in
// the order their newlines were written natively.
import { expect, test } from "bun:test";
import { join } from "node:path";

import { writeFileSync } from "node:fs";

import { runSync } from "./child";
import { libraryCrate } from "./metamorphic";
import { compileJs, compileJsLibrary, mergedStreams, nativeFlags } from "./programs";
import { buildCompiler, fixture, root } from "./support";

const cases = [
  "printed_while_a_line_is_formatted.rs",
  "eprinted_while_formatting.rs",
  "stderr_between_a_writers_lines.rs",
  "stderr_inside_a_std_values_debug.rs",
  "line_printed_after_a_part_left_unended.rs",
  "stdout_held_past_stderr.rs",
];

for (const name of cases) {
  test(`${name}: stdout and stderr interleave as native's`, () => {
    buildCompiler();
    const dir = fixture(`stream-order-${name}`);
    const { native, js, nativeLines, browserLines } = mergedStreams(join(root, "test/corpus", name), dir);
    expect(js).toBe(native);
    expect(browserLines).toEqual(nativeLines);
  }, 300_000);
}

// A library that prints whole lines alone writes each as `console.log`
// (ADR 0087); its app's `print!` leaves a line unended, held by the
// runtime. Natively one `LineWriter` holds both, so the library's line ends
// the app's, and stderr, unbuffered, comes before what's held. While the
// runtime holds text, `console.log` is wrapped and the held text joined to
// the line: on Node, and on Bun, whose `console.log` writes the descriptor
// itself, where a write apart could come out after the line (#2320, #2347).
test("a library's whole lines inside a line its app holds interleave as native's, 2>&1, on Node and Bun", () => {
  buildCompiler();
  const dir = fixture("stream-order-two-crates");
  const library = 'pub fn line(n: u32) {\n    println!("lib {n}");\n}\n\npub fn warn() {\n    eprintln!("warned");\n}\n';
  const program = `fn main() {\n    print!("partial ");\n    ${libraryCrate}::line(1);\n    print!("a");\n    ${libraryCrate}::warn();\n    print!("b ");\n    ${libraryCrate}::line(2);\n    print!("left ");\n    eprint!("err ");\n    ${libraryCrate}::line(3);\n    print!("at exit");\n}\n`;
  writeFileSync(join(dir, "library.rs"), library);
  writeFileSync(join(dir, "program.rs"), program);
  const rlib = join(dir, `lib${libraryCrate}.rlib`);
  const sh = (cmd: string[]) => Bun.spawnSync(["sh", "-c", '"$0" "$@" 2>&1', ...cmd], { cwd: dir, timeout: 120_000 }).stdout.toString();
  expect(runSync(["rustc", ...nativeFlags("2024"), "--crate-type=rlib", `--crate-name=${libraryCrate}`, join(dir, "library.rs"), "-o", rlib], root, 120_000).stderr).toBe("");
  const binary = join(dir, "program");
  expect(runSync(["rustc", ...nativeFlags("2024"), "--extern", `${libraryCrate}=${rlib}`, join(dir, "program.rs"), "-o", binary], root, 120_000).stderr).toBe("");
  const native = sh([binary]);
  expect(native).toBe("partial lib 1\nwarned\nab lib 2\nerr left lib 3\nat exit");
  const lib = compileJsLibrary(join(dir, "library.rs"), dir, "2024", libraryCrate);
  if (!("rmeta" in lib)) throw new Error(lib.error);
  const compiled = compileJs(join(dir, "program.rs"), dir, "2024", false, {}, ["--dependency", lib.manifest], ["--extern", `${libraryCrate}=${lib.rmeta}`, "-L", dir]);
  if (!("js" in compiled)) throw new Error(compiled.error);
  for (const runtime of ["node", "bun"]) {
    const outcome = join(dir, `${runtime}.json`);
    expect([runtime, sh([runtime, join(root, "test/corpus-run.ts"), compiled.js, outcome, compiled.js, outcome])]).toEqual([runtime, native]);
  }
}, 300_000);
