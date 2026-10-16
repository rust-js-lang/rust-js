// Mutations of src/main.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "cargo-record-outside-plan",
    breaks: "what Cargo is told of a crate is written on its own, and a build that fails writing it has changed the JS",
    file: "src/main.rs",
    find: "            _ => Vec::new(),\n        };\n",
    replace: "            _ => Vec::new(),\n        };\n        for (path, bytes) in std::mem::take(&mut plan.extra) {\n            let _ = std::fs::write(path, bytes);\n        }\n",
    tests: ["test/cargo-workspace.test.ts", "-t", "can't record what it made"],
  },
  {
    name: "rust-js-tool-unregistered",
    breaks: "`#[rust_js::link_name]` names a tool rustc doesn't know, and a binding is an error",
    file: "src/main.rs",
    find: "            tools.insert(Ident::with_dummy_span(Symbol::intern(\"rust_js\")));\n",
    replace: "",
    tests: ["test/diagnostics.test.ts","-t","unstable feature rust-js"],
  },
  {
    name: "crate-id-miscomputed",
    breaks: "JSX's expansions are hashed with another crate's id than rustc's",
    file: "src/main.rs",
    find: "    StableCrateId::new(\n        name,\n",
    replace: "    StableCrateId::new(\n        Symbol::intern(\"elsewhere\"),\n",
    tests: ["test/cargo-react.test.ts", "-t", "a component of a Cargo workspace using the react crate"],
  },
  {
    name: "rustc-mode-without-tool",
    breaks: "`rust-js --rustc` is rustc without rust-js's tool, and the binding crates don't compile",
    file: "src/main.rs",
    find: "impl Callbacks for Syntax {\n    fn config(&mut self, config: &mut rustc_interface::interface::Config) {\n        register_tool(config);\n",
    replace: "impl Callbacks for Syntax {\n    fn config(&mut self, _config: &mut rustc_interface::interface::Config) {\n",
    tests: ["test/cargo-react.test.ts"],
  },
  {
    name: "cargo-app-untracked",
    breaks: "Cargo isn't told what `RUST_JS_APP` was, so the app compiled as a program is fresh when another crate uses it",
    file: "src/main.rs",
    find: "            compiler\n                .sess\n                .env_depinfo\n                .lock()\n                .insert((Symbol::intern(cargo::APP), value));\n",
    replace: "            let _ = value;\n",
    tests: ["test/cargo-workspace.test.ts", "-t", "app's crate is a program"],
  },
  {
    name: "main-stack-default",
    breaks: "a long function's MIR overflows rustc's default stack",
    file: "src/main.rs",
    find: "        let _ = rustc_interface::util::STACK_SIZE.set(256 * 1024 * 1024);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","a long function"],
  },
  {
    name: "stage-kept",
    breaks: "a build that ends before publishing, refused by its settings, leaves its staging directory beside the metadata",
    file: "src/main.rs",
    find: "        if let Some(dir) = &self.0 {\n            let _ = std::fs::remove_dir_all(dir);\n        }\n",
    replace: "        let _ = &self.0;\n",
    tests: ["test/crates.test.ts", "-t", "settings refuse|can't be published publishes nothing"],
  },
  {
    name: "panics-abort",
    breaks: "rustc takes wasm32's panics to abort, where rust-js's unwind: a future's size and `cfg(panic)` are another program's",
    file: "src/main.rs",
    find: "        config.opts.cg.panic = Some(rustc_target::spec::PanicStrategy::Unwind);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^future_size_unwinds.rs$"],
  },
];
