// The rustc flags that find the crates serde/build.sh built (ADR 0077), of
// `cargo build --message-format=json`'s messages on stdin: each library's
// file where Cargo says it put it, and each directory Cargo put one in, which
// Cargo 1.100 makes one for each build step's, `build/<package>/<hash>/out`,
// where 1.99's were all in `deps` (ADR 0109, amended).
//
//   cargo build --message-format=json .. | bun serde/flags.ts <profile dir> <extension>
//
// `<profile dir>` is where the crates asked for were built, the target's
// `wasm32-unknown-unknown/debug` say, and `<extension>` the file of each
// asked for, `rmeta` or `rlib`.
import { dirname, resolve, sep } from "node:path";

const [profile, extension] = process.argv.slice(2);
if (!profile || !extension) throw new Error("usage: bun serde/flags.ts <profile dir> <extension>");
const within = resolve(profile) + sep;
const asked = ["serde", "serde_json", "hashbrown", "indexmap", "memchr"];
const files = new Map<string, string>();
const dirs = new Set<string>();
for (const line of (await Bun.stdin.text()).split("\n").filter(Boolean)) {
  const message = JSON.parse(line);
  if (message.reason !== "compiler-artifact") continue;
  for (const file of message.filenames as string[]) {
    dirs.add(dirname(resolve(file)));
    if (resolve(file).startsWith(within) && file.endsWith(`.${extension}`) && asked.includes(message.target.name)) {
      files.set(message.target.name, resolve(file));
    }
  }
}
const missing = asked.filter((name) => !files.has(name));
if (missing.length) throw new Error(`cargo built no ${extension} of ${missing.join(", ")} in ${profile}`);
console.log([...asked.map((name) => `--extern ${name}=${files.get(name)}`), ...[...dirs].sort().map((dir) => `-L dependency=${dir}`)].join(" "));
