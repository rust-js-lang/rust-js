// Mutations of src/lower/field_keys.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "field-symbol-read-by-name",
    breaks: "a field a symbol names is read and written by the symbol item's name, `reason.RESPONSE_SYMBOL`",
    file: "src/lower/field_keys.rs",
    find: "            Some((item, span)) => Expr::index(base, self.field_symbol(item, span)),",
    replace: "            Some((item, _)) => Expr::member(base, self.tcx.item_name(item).as_str()),",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a field a symbol names is that symbol's key"
    ]
  },
  {
    name: "field-symbol-global-as-item",
    breaks: "JS's own symbol, `js::symbol::async_iterator`, is read as the crate's item it isn't",
    file: "src/lower/field_keys.rs",
    find: "            true => self.binding_ref(&bindings::js_name(self.tcx, item), named),",
    replace: "            true => self.item_read(item, named),",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a closure a symbol names is its method"
    ]
  },
];
