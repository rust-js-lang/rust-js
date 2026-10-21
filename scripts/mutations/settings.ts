// Mutations of src/settings.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "settings-format-table-allowed",
    breaks: "`[package.metadata.rust-js.format]`, gone, is read as nothing, and its options a second place the layout is said",
    file: "src/settings.rs",
    find: "        if settings.format.is_some() {",
    replace: "        if settings.format.is_some() && false {",
    tests: ["test/settings.test.ts", "-t", "not Cargo.toml's"],
  },
  {
    name: "settings-prettier-transform-allowed",
    breaks: "a transform that runs Prettier is run, a second Prettier after rust-js's own",
    file: "src/settings.rs",
    find: ".find(|command| runs_prettier(command))",
    replace: ".find(|_| false)",
    tests: ["test/settings.test.ts", "-t", "Prettier as a transform is refused"],
  },
  {
    name: "settings-prettier-by-path",
    breaks: "a transform runs Prettier only as `prettier`, not by its path, and `node_modules/.bin/prettier` is run",
    file: "src/settings.rs",
    find: ".is_some_and(|name| name == \"prettier\")",
    replace: ".is_some_and(|name| name == \"prettier\" && false)",
    tests: ["test/settings.test.ts", "-t", "Prettier as a transform is refused"],
  },
  {
    name: "settings-check-failure-a-warning",
    breaks: "a check that doesn't say what its failure is warns, and the build it failed is published",
    file: "src/settings.rs",
    find: "    /// The build's.\n    #[default]\n    Build,\n    /// Its output, as warnings.\n    Never,",
    replace: "    /// The build's.\n    Build,\n    /// Its output, as warnings.\n    #[default]\n    Never,",
    tests: ["test/settings.test.ts", "-t", "check that fails"],
  },
  {
    name: "settings-farthest-manifest",
    breaks: "a crate's settings are read from the outermost Cargo.toml above it, a workspace's, not its own",
    file: "src/settings.rs",
    find: "                .find(|dir| dir.join(\"Cargo.toml\").is_file())\n",
    replace: "                .filter(|dir| dir.join(\"Cargo.toml\").is_file())\n                .last()\n",
    tests: ["test/settings.test.ts", "-t", "check that fails"],
  },
  {
    name: "settings-declarations-key-kept",
    breaks: "a crate's `declarations = true`, a key that's gone, is taken quietly, as if it said something",
    file: "src/settings.rs",
    find: "        if settings.declarations.is_some() {\n",
    replace: "        if false && settings.declarations.is_some() {\n",
    tests: ["test/settings.test.ts", "-t", "no key"],
  },
  {
    name: "settings-no-manifest-no-dir",
    breaks: "a file compiled alone with no Cargo.toml above it has no directory, and TypeScript is run nowhere: `node` can't be run",
    file: "src/settings.rs",
    find: "            return Ok(Settings {\n                dir: crate::paths::parent_dir(&crate::paths::absolute(input)?).to_path_buf(),\n                ..Settings::default()\n            });",
    replace: "            return Ok(Settings::default());",
    tests: ["test/declarations.test.ts", "-t", "no Cargo.toml above it has its declarations"],
  },
];
