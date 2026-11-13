// Mutations of src/output/local_js.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "bound-file-unfollowed",
    breaks: "a bound JS file's own imports aren't read, so React's ReactFiberDevToolsHook.js importing ReactEventPriorities, which imports ReactFiberLane, is no cycle and `NoEventPriority = NoLane` throws where ReactFiberLane loads first",
    file: "src/output/local_js.rs",
    find: "            for specifier in self.loads(&file)? {\n",
    replace: "            for specifier in Vec::<String>::new() {\n",
    tests: ["test/lowering-constants.test.ts", "-t", "a bound JS file imports back"],
  },
  {
    name: "bound-module-unmatched",
    breaks: "a crate module a bound JS file imports, `./tags.js`, is a file not found, which may be any, so a module's load reads another's constant as its value though no cycle is there",
    file: "src/output/local_js.rs",
    find: "        .find_map(|s| stems.get(s))\n",
    replace: "        .find_map(|s| stems.get(s).filter(|_| false))\n",
    tests: ["test/lowering-constants.test.ts", "-t", "a bound JS file imports back"],
  },
  {
    name: "package-may-import-any",
    breaks: "a package a bound JS file imports, `node:path`, may import any of the crate's modules, so every module's load reads constants as their values",
    file: "src/output/local_js.rs",
    find: "            false => Imported::Package,\n",
    replace: "            false => Imported::Unknown,\n",
    tests: ["test/lowering-constants.test.ts", "-t", "a bound JS file imports back"],
  },
  {
    name: "missing-import-ignored",
    breaks: "a file a bound JS file imports that isn't there, which may be the crate's, imports nothing, and a cycle through it is unseen",
    file: "src/output/local_js.rs",
    find: "                    Imported::Unknown => return None,\n",
    replace: "                    Imported::Unknown => {}\n",
    tests: ["test/lowering-constants.test.ts", "-t", "a bound JS file imports back"],
  },
  {
    name: "missing-bound-file-imports-nothing",
    breaks: "a bound JS file that isn't where the crate's JS is imports nothing, though it may import any of the crate's modules",
    file: "src/output/local_js.rs",
    find: "        if loaded.is_empty() && pending.is_empty() {\n",
    replace: "        if false {\n",
    tests: ["test/lowering-constants.test.ts", "-t", "a bound JS file imports back"],
  },
  {
    name: "style-may-import-any",
    breaks: "a style a bound JS file imports, `./hook.css`, isn't read as JS, so it may import any of the crate's modules",
    file: "src/output/local_js.rs",
    find: "        if extension.as_deref().is_some_and(|e| ASSET_EXTENSIONS.contains(&e)) {\n",
    replace: "        if false {\n",
    tests: ["test/lowering-constants.test.ts", "-t", "a bound JS file imports back"],
  },
  {
    name: "css-module-file-unrecorded",
    breaks: "a CSS Module a binding is checked against isn't what the build read, so a save of it doesn't check the program again (ADR 1133)",
    file: "src/output/local_js.rs",
    find: "            self.loads.borrow_mut().insert(file.clone(), Some(Vec::new()));\n",
    replace: "",
    tests: ["test/css-modules.test.ts", "-t", "a save of it"],
  },
  {
    name: "css-module-in-source-unlooked",
    breaks: "a CSS Module beside the Rust, where the crate's JS goes in source, isn't found where the JS is written elsewhere (ADR 1133)",
    file: "src/output/local_js.rs",
    find: "        for root in [&self.output, &self.input] {\n            let file = specifier_file(parent_dir(&absolute(root)?), specifier)?;\n",
    replace: "        for root in [&self.output] {\n            let file = specifier_file(parent_dir(&absolute(root)?), specifier)?;\n",
    tests: ["test/css-modules.test.ts", "-t", "written elsewhere"],
  },
];
