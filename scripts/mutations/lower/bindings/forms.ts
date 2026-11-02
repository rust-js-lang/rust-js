// Mutations of src/lower/bindings/forms.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "instance-of-given-unknown",
    breaks:
      "a binding's `link_name = \"instanceof\"`, `js::instance_of`'s, is a call of a method named so, `value.instanceof(constructor)` (ADR 0878)",
    file: "src/lower/bindings/forms.rs",
    find: '        "instanceof" => return JsForm::InstanceOfGiven,\n',
    replace: "",
    tests: [
      "test/js-collections.test.ts",
      "-t",
      "instanceof of a constructor value",
    ],
  },
  {
    name: "listener-identity-cloned",
    breaks:
      "an identity binding clones its function instead of retaining the same listener",
    file: "src/lower/bindings/forms.rs",
    find: '        "this" => return JsForm::This,',
    replace: '        "this" => return JsForm::Call("clone".to_string()),',
    tests: ["test/webapi-events.test.ts", "-t", "emit direct JavaScript"],
  },
  {
    name: "get-index-unread",
    breaks:
      "`js::get(value, key)`'s `get []` is a method of that name, not `value[key]`",
    file: "src/lower/bindings/forms.rs",
    find: '        "get []" => return JsForm::GetIndex,\n',
    replace: "",
    tests: ["test/lowering-bindings.test.ts", "-t", "unknown JS value"],
  },
  {
    name: "truthy-form-unknown",
    breaks: "`js::truthy`'s `!!` is a method's name, `value.!!()`",
    file: "src/lower/bindings/forms.rs",
    find: '        "!!" => return JsForm::Truthy,\n',
    replace: "",
    tests: ["test/bindings.test.ts", "-t", "tested as JS tests it"],
  },
  {
    name: "object-binding-unread",
    breaks:
      "a binding of an object, `{default}`, is a call of a function of that name, not the object of its arguments",
    file: "src/lower/bindings/forms.rs",
    find: "    if let Some(keys) = name.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {",
    replace:
      "    if let Some(keys) = name.strip_prefix(\"{{\").and_then(|t| t.strip_suffix('}')) {",
    tests: [
      "test/jsx.test.ts",
      "-t",
      "named component is loaded as a module's default",
    ],
  },
  {
    name: "empty-object-binding-refused",
    breaks:
      "a `{}` binding of no arguments, `dict::new()`, has one empty key, and rust-js refuses it",
    file: "src/lower/bindings/forms.rs",
    find: "        let keys = keys.split(',').map(str::trim).filter(|k| !k.is_empty());",
    replace: "        let keys = keys.split(',').map(str::trim);",
    tests: [
      "test/node-bindings.test.ts",
      "-t",
      "querystring module is bound whole",
    ],
  },
  {
    name: "typeof-form-called",
    breaks:
      '`#[link_name = "typeof"]` is a call, `typeof(value)`, not the operator',
    file: "src/lower/bindings/forms.rs",
    find: '        "typeof" => return JsForm::TypeOf,\n',
    replace: "",
    tests: ["test/bindings.test.ts", "-t", "react.dev's Console"],
  },
  {
    name: "text-form-called",
    breaks: '`#[link_name = "\\"\\" +"]` is a call of `"" +`, not `"" + value`',
    file: "src/lower/bindings/forms.rs",
    find: '        "\\"\\" +" => return JsForm::Text,\n',
    replace: "",
    tests: ["test/bindings.test.ts", "-t", "react.dev's Console"],
  },
  {
    name: "is-function-form-called",
    breaks:
      '`js::is_function(binding)` is a call of no JS function, not `typeof X === "function"` (ADR 0942)',
    file: "src/lower/bindings/forms.rs",
    find: '        "typeof === \\"function\\"" => return JsForm::Asks(Asked::Function),\n',
    replace: "",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "is_function is whether",
    ],
  },
  {
    name: "is-number-form-called",
    breaks:
      '`js::is_number(binding)` is a call of no JS function, not `typeof devicePixelRatio === "number"` (rd195)',
    file: "src/lower/bindings/forms.rs",
    find: '        "typeof === \\"number\\"" => return JsForm::Asks(Asked::Number),\n',
    replace: "",
    tests: ["test/lowering-is-function.test.ts", "-t", "is_number is whether"],
  },
  {
    name: "is-number-asked-as-function",
    breaks:
      '`js::is_number(width)` is `typeof rustJsWidth === "function"`, false of a number (rd195)',
    file: "src/lower/bindings/forms.rs",
    find: '            Asked::Number => Some("number"),\n',
    replace: '            Asked::Number => Some("function"),\n',
    tests: ["test/lowering-is-function.test.ts", "-t", "is_number is whether"],
  },
  {
    name: "is-function-refusal-named-defined",
    breaks:
      "`js::is_function(|| 1)` is refused naming `js::is_defined`, which the program didn't write (ADR 0942)",
    file: "src/lower/bindings/forms.rs",
    find: '            Asked::Function => "js::is_function",\n',
    replace: '            Asked::Function => "js::is_defined",\n',
    tests: ["test/lowering-is-function.test.ts", "-t", "refused"],
  },
  {
    name: "static-form-a-method",
    breaks:
      "`Class::call_static` is a method of the class called `static`, `localPromise.static(..)` (rd235)",
    file: "src/lower/bindings/forms.rs",
    find: '        "static" => return JsForm::Static,\n',
    replace: "",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "a class held as a value",
    ],
  },
  {
    name: "asked-object-unrecognized",
    breaks:
      '`js::is_object(navigation)` is a call of a binding named `typeof === "object"`, not React DOM\'s `typeof navigation === "object"`',
    file: "src/lower/bindings/forms.rs",
    find: '        "typeof === \\"object\\"" => return JsForm::Asks(Asked::Object),\n',
    replace: "",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "js::is_object is whether",
    ],
  },
  {
    name: "unary-plus-a-call",
    breaks: "`js::to_number`'s `+` form is read as a call of a function named `+`, not JS's unary `+width` (rd331c)",
    file: "src/lower/bindings/forms.rs",
    find: "        \"+\" => return JsForm::Number,\n",
    replace: "",
    tests: ["test/js-any-values.test.ts", "-t", "to_number is JS's unary"],
  },
  {
    name: "binary-minus-a-call",
    breaks: "`js::subtract`'s `-` form is read as a call of a function named `-`, not JS's `bound.length - 1` (rd564)",
    file: "src/lower/bindings/forms.rs",
    find: "        \"-\" => return JsForm::Subtract,\n",
    replace: "",
    tests: ["test/js-any-values.test.ts", "-t", "subtract is JS's binary"],
  },
  {
    name: "method-throw-a-statement",
    breaks:
      "`AsyncIterator::throw`, a method of `&self`, is the `throw` statement's form, `iterator.throw(reason)` an error where React's Flight reply server calls it (rd584)",
    file: "src/lower/bindings/forms.rs",
    find: '        "throw" if !of_self(tcx, def_id) => return JsForm::Throw,\n',
    replace: '        "throw" => return JsForm::Throw,\n',
    tests: ["test/js-builtins.test.ts", "-t", "an async iterator's throw is its optional method"],
  },
  {
    name: "this-throw-a-member",
    breaks: "`js::throw(value)`, a function whose `this` is its parameter, is `value.throw()`, not `throw value` (rd584)",
    file: "src/lower/bindings/forms.rs",
    find: "    tcx.def_kind(def_id) == DefKind::AssocFn && tcx.associated_item(def_id).is_method()\n",
    replace: "    super::is_method(tcx, def_id)\n",
    tests: ["test/js-builtins.test.ts", "-t", "an async iterator's throw is its optional method"],
  },
  {
    name: "keyed-method-a-call",
    breaks:
      "`AsyncIterable::async_iterator`'s `[Symbol.asyncIterator]` is a method named `[Symbol.asyncIterator]`, refused, not `iterable[Symbol.asyncIterator]()` (rd585)",
    file: "src/lower/bindings/forms.rs",
    find: "        return JsForm::CallKeyed(key.to_string());\n",
    replace: "        let _ = key;\n",
    tests: ["test/js-builtins.test.ts", "-t", "an async iterable's"],
  },
  {
    name: "keyed-method-quoted",
    breaks:
      "a method's quoted name, `['matchMedia']`, is taken for a key, refused as no global's path, not as a method's property named, `matchMedia` (ADR 1033)",
    file: "src/lower/bindings/forms.rs",
    find: "        .filter(|key| !key.starts_with(['\\'', '\"']))\n",
    replace: "",
    tests: ["test/lowering-binding-paths.test.ts", "-t", "naming the quoted key"],
  },
];
