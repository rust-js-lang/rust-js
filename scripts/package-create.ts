// Pack @rust-js/create (ADR 0105): its script, and as its templates the
// vite-react and Next.js examples' files as git has them, with create/vite's
// in place of the Vite example's own (ADR 0192). This never publishes to a
// registry.
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const [destination, ...extra] = Bun.argv.slice(2);
if (!destination || extra.length) throw new Error("Usage: bun scripts/package-create.ts <output.tgz>");
const output = resolve(destination);
const run = (args: string[], cwd = root) => {
  const result = Bun.spawnSync(args, { cwd, stdout: "pipe", stderr: "pipe" });
  if (result.exitCode !== 0) throw new Error(result.stderr.toString());
  return result.stdout.toString();
};
const version = (Bun.TOML.parse(readFileSync(join(root, "Cargo.toml"), "utf8")) as { package: { version: string } }).package.version;
const pkg = JSON.parse(readFileSync(join(root, "create/package.json"), "utf8"));
if (pkg.version !== version) throw new Error(`@rust-js/create ${pkg.version} does not match compiler ${version}`);
const staging = mkdtempSync(join(tmpdir(), "rust-js-create-"));
try {
  for (const file of ["index.js", "package.json", "README.md"]) copyFileSync(join(root, "create", file), join(staging, file));
  // Each template: an example's files as git has them, so nothing building
  // it made goes with it, with its overrides', `create/<template>`, in place
  // of the example's own. The crates the example has by path, from this
  // checkout, are an app's npm packages, each at its crate's version, named
  // in its Cargo.toml by version: its `postinstall`, rust-js-patch, tells
  // Cargo where they are. Each crate a crate it names uses is a package too.
  const templates = [
    { name: "vite", example: "examples/vite-react", crates: ["builtins", "webapi", "react"] },
    { name: "next", example: "examples/next", crates: ["builtins", "webapi", "react", "next", "node"] },
  ];
  const crateVersion = (dir: string) => (Bun.TOML.parse(readFileSync(join(root, dir, "Cargo.toml"), "utf8")) as { package: { version: string } }).package.version;
  for (const { name, example, crates } of templates) {
    const overridden = join(root, "create", name);
    const overrides = new Set(existsSync(overridden)
      ? readdirSync(overridden, { recursive: true, withFileTypes: true })
        .filter((entry) => entry.isFile())
        .map((entry) => relative(overridden, join(entry.parentPath, entry.name)))
      : []);
    const template = join(staging, "templates", name);
    for (const file of run(["git", "ls-files", example]).trim().split("\n")) {
      const path = relative(example, file);
      // npm leaves a package's `.gitignore` out: @rust-js/create names it back.
      const target = join(template, path === ".gitignore" ? "_gitignore" : path);
      mkdirSync(dirname(target), { recursive: true });
      copyFileSync(overrides.has(path) ? join(overridden, path) : join(root, file), target);
    }
    const cargoPath = join(template, "Cargo.toml");
    let cargo = readFileSync(cargoPath, "utf8");
    for (const dir of crates) cargo = cargo.replace(`path = "../../${dir}"`, `version = "~${crateVersion(dir)}"`);
    if (cargo.includes('path = "../')) throw new Error(`${example}'s Cargo.toml has a crate by path that isn't a package`);
    writeFileSync(cargoPath, cargo);
    const templatePath = join(template, "package.json");
    const manifest = JSON.parse(readFileSync(templatePath, "utf8"));
    for (const dir of crates) manifest.dependencies[`@rust-js/${dir}`] = crateVersion(dir);
    manifest.devDependencies["@rust-js/build"] = "workspace:*";
    manifest.scripts = { ...manifest.scripts, postinstall: "rust-js-patch" };
    writeFileSync(templatePath, JSON.stringify(manifest, null, 2) + "\n");
  }
  mkdirSync(dirname(output), { recursive: true });
  run([process.execPath, "pm", "pack", "--ignore-scripts", "--filename", output], staging);
  console.log(output);
} finally {
  rmSync(staging, { recursive: true, force: true });
}
