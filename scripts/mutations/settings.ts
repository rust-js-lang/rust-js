// Mutations of src/settings.rs (ADR 0093).
import type { Mutation } from "../mutations";

const formats = ["test/settings.test.ts", "-t", "formatter's options|formatter hasn't"];

export const mutations: Mutation[] = [
  {
    name: "settings-format-unknown-allowed",
    breaks: "an option oxfmt hasn't, a misspelling, is ignored, and the layout isn't the one asked for",
    file: "src/settings.rs",
    find: "#[serde(rename_all = \"camelCase\", deny_unknown_fields)]\npub struct Format {",
    replace: "#[serde(rename_all = \"camelCase\")]\npub struct Format {",
    tests: formats,
  },
  {
    name: "settings-format-snake-case",
    breaks: "the formatter's options are named as Rust's fields are, not as oxfmt names them",
    file: "src/settings.rs",
    find: "#[serde(rename_all = \"camelCase\", deny_unknown_fields)]\npub struct Format {",
    replace: "#[serde(deny_unknown_fields)]\npub struct Format {",
    tests: formats,
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
    name: "settings-check-built-only",
    breaks: "a check that doesn't say when it runs isn't run for a save in the Vite server",
    file: "src/settings.rs",
    find: "    #[default]\n    Always,\n    /// A build, not a save.\n    Build,",
    replace: "    Always,\n    /// A build, not a save.\n    #[default]\n    Build,",
    tests: ["test/settings.test.ts", "-t", "only in a build"],
  },
  {
    name: "settings-farthest-manifest",
    breaks: "a crate's settings are read from the outermost Cargo.toml above it, a workspace's, not its own",
    file: "src/settings.rs",
    find: "                .find(|dir| dir.join(\"Cargo.toml\").is_file())\n",
    replace: "                .filter(|dir| dir.join(\"Cargo.toml\").is_file())\n                .last()\n",
    tests: ["test/settings.test.ts", "-t", "check that fails"],
  },
];
