// Rust's Unicode tables, which the runtime asks in place of a JS engine's
// (#1888, ADR 1163): crates/rust-js-program/src/runtime/unicode_tables.js is what
// scripts/unicode.ts makes of the pinned toolchain's std, so a toolchain
// bump that moves Unicode fails here until they're made again; and the JS
// answers as native Rust does of every `char` of every plane, each question,
// `{:?}` and both case maps, hashed a plane a line. Here, not in the corpus,
// whose nets compile and run each case many times over: a million `char`s
// a run is seconds of JS (#1888).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { underLoad } from "../scripts/timing";
import { printed } from "./crates";
import { buildCompiler, compiler, fixture, generatorOutput, nativeBinary, root, run, writeWhole } from "./support";

beforeAll(buildCompiler, 600_000);

test("unicode_tables.js is what scripts/unicode.ts makes of the toolchain's std", () => {
  expect(generatorOutput("scripts/unicode.ts")).toBe(readFileSync(join(root, "crates/rust-js-program/src/runtime/unicode_tables.js"), "utf8"));
}, underLoad(120_000));

const everyChar = `fn mixed(hash: u32, text: &str) -> u32 {
    text.bytes().fold(hash, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(16_777_619))
}

pub fn main() {
    for plane in 0..17u32 {
        let mut hash = 2_166_136_261u32;
        let mut chars = 0;
        for code in plane << 16..(plane + 1) << 16 {
            let Some(c) = char::from_u32(code) else {
                continue;
            };
            chars += 1;
            let asked = [
                c.is_alphabetic(),
                c.is_numeric(),
                c.is_alphanumeric(),
                c.is_uppercase(),
                c.is_lowercase(),
                c.is_whitespace(),
                c.is_control(),
            ];
            let answers: String = asked.iter().map(|&yes| if yes { '1' } else { '0' }).collect();
            hash = mixed(hash, &answers);
            hash = mixed(hash, &format!("{c:?}"));
            hash = mixed(hash, &c.to_uppercase().collect::<String>());
            hash = mixed(hash, &c.to_lowercase().collect::<String>());
        }
        println!("plane {plane}: {chars} chars, {hash:08x}");
    }
}
`;

test("every char's Unicode answers are native Rust's", () => {
  const dir = fixture("unicode-every-char");
  const built = nativeBinary(everyChar.replace("pub fn main", "fn main"), dir, ["--edition=2024", "-O", "-Awarnings"]);
  if ("error" in built) throw new Error(built.error);
  const native = run([built.binary]);
  writeWhole(join(dir, "app.rs"), everyChar);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--", "--edition=2024", "--crate-type=lib"]);
  expect(printed(join(dir, "app.js"))).toBe(native);
}, underLoad(120_000));
