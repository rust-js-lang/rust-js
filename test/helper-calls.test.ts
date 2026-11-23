// A helper call is read by the names its own JS declares its parameters by,
// `call.helper_call_of("$lent")?.given("kind")`, never by its shape, its
// arguments' count or a fixed index, which an argument a helper gains or
// a call gives breaks (#2334). Each file's reads by the callee's name,
// `helper_name()`, are at most test/helper-shapes-known.txt's: a ratchet,
// which migrating a file lowers. That each name a call is read by is one
// its helper declares is `js::helper_call`'s own test, which reads the
// runtime's parameters as the compiler does.
import { expect, test } from "bun:test";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const read = (file: string) => readFileSync(join(root, file), "utf8");
const files = (dir: string): string[] =>
  readdirSync(join(root, dir)).flatMap((name) => {
    const path = join(dir, name);
    return statSync(join(root, path)).isDirectory() ? files(path) : path.endsWith(".rs") ? [path] : [];
  });

const known: Record<string, number> = Object.fromEntries(
  read("test/helper-shapes-known.txt").split("\n").filter((line) => line && !line.startsWith("#"))
    .map((line) => line.split("\t")).map(([file, n]) => [file, Number(n)]),
);

test("each file reads helper calls by their callee's name at most as often as the ratchet says, and says exactly", () => {
  const counted = (text: string) => text.split("\n").filter((line) => line.includes("helper_name()")).length;
  const now = Object.fromEntries([...files("crates"), ...files("src")].map((file) => [file, counted(read(file))] as const).filter(([, n]) => n > 0));
  const grown = Object.entries(now).filter(([file, n]) => n > (known[file] ?? 0))
    .map(([file, n]) => `${file}: ${n} reads by \`helper_name()\`, the ratchet's ${known[file] ?? 0}: read its arguments by name, \`helper_call_of\``);
  expect(grown).toEqual([]);
  const shrunk = Object.entries(known).filter(([file, n]) => (now[file] ?? 0) < n)
    .map(([file, n]) => `${file}: ${now[file] ?? 0} reads by \`helper_name()\`, fewer than the ratchet's ${n}: lower it in test/helper-shapes-known.txt`);
  expect(shrunk).toEqual([]);
});
