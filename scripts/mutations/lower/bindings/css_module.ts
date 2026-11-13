// Mutations of src/lower/bindings/css_module.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "css-module-class-unchecked",
    breaks:
      "a CSS Module's getter of a class its file doesn't export, `get codeBlok`, compiles, and reads `undefined` typed `String` (ADR 1133)",
    file: "src/lower/bindings/css_module.rs",
    find: "                JsForm::Get(class) if is_method(tcx, getter) && keys.contains(&class) => {}\n",
    replace: "                JsForm::Get(class) if is_method(tcx, getter) && (keys.contains(&class) || true) => {}\n",
    tests: ["test/css-modules.test.ts", "-t", "doesn't export|where no code reads it|a save of it"],
  },
  {
    name: "css-module-of-another-import-taken",
    breaks:
      "a CSS Module's `static` bound to a named export or a plain stylesheet is taken, checked against no CSS Module's keys (ADR 1133)",
    file: "src/lower/bindings/css_module.rs",
    find: '            Some(((from, export), "")) if export == "default" && is_module_css(&from) => from,\n',
    replace: '            Some(((from, _), _)) => from,\n',
    tests: ["test/css-modules.test.ts", "-t", "refused, saying what to write"],
  },
  {
    name: "css-module-call-taken",
    breaks: "a CSS Module's binding that calls a class, `codeBlock`, is taken, though a class is a string, not a function (ADR 1133)",
    file: "src/lower/bindings/css_module.rs",
    find: "                JsForm::GetIndex => {}\n",
    replace: "                JsForm::GetIndex | JsForm::Call(_) => {}\n",
    tests: ["test/css-modules.test.ts", "-t", "refused, saying what to write"],
  },
  {
    name: "css-module-of-an-enum-taken",
    breaks: "`#[rust_js::css_module]` of an enum, which binds no classes, is taken silently (ADR 1133)",
    file: "src/lower/bindings/css_module.rs",
    find: "        if marked(tcx, def) && tcx.def_kind(def) != DefKind::Struct {\n",
    replace: "        if marked(tcx, def) && tcx.def_kind(def) != DefKind::Struct && false {\n",
    tests: ["test/css-modules.test.ts", "-t", "refused, saying what to write"],
  },
  {
    name: "css-module-turbopack-only-unnamed",
    breaks: "a getter of a key Turbopack alone exports, `vtName`, is refused as one the file doesn't export, not naming which bundlers do (#1621)",
    file: "src/lower/bindings/css_module.rs",
    find: "                JsForm::Get(class) if is_method(tcx, getter) && exported.turbopack_only.contains(&class) => {\n",
    replace: "                JsForm::Get(class) if is_method(tcx, getter) && false && exported.turbopack_only.contains(&class) => {\n",
    tests: ["test/css-modules-parity.test.ts"],
  },
];
