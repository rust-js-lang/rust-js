// Mutations of src/jsx_syntax/props.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flattened-names-refused",
    breaks: "a name a flattened struct has is refused as the props' own",
    file: "src/jsx_syntax/props.rs",
    find: "            Some(_) => \"NAME!(@slots [$($s)*] $flat [$($rest)* $name: $v,] $($tail)*)\",",
    replace: "            Some(_) => \"NAME { $name: $v, ..::core::panic!() }\",",
    tests: ["test/jsx-props.test.ts", "-t", "flattened props where they"],
  },
  {
    name: "required-prop-defaulted",
    breaks: "a required prop not given is its type's default, not said missing",
    file: "src/jsx_syntax/props.rs",
    find: "            \"(@slot required $name:literal []) => { ::core::compile_error!(concat!(\\\"missing prop `\\\", $name, \\\"` of `NAME`\\\")) }\"",
    replace: "            \"(@slot required $name:literal []) => { ::core::default::Default::default() }\"",
    tests: ["test/jsx-props.test.ts", "-t", "flattened props where they"],
  },
  {
    name: "omitted-prop-defaulted",
    breaks: "a prop with a default not given is written, `size=\\\"md\\\"`, not left out",
    file: "src/jsx_syntax/props.rs",
    find: "        arms.push(\"(@slot omitted $name:literal []) => { ::react::__omitted() }\".to_string());",
    replace: "        arms.push(\"(@slot omitted $name:literal []) => { ::core::default::Default::default() }\".to_string());",
    tests: ["test/jsx-props.test.ts", "-t", "flattened props where they"],
  },
  {
    name: "flattened-field-given-whole-refused",
    breaks: "a component's own flattened props passed on whole, `props={props}`, are taken for a name of theirs",
    file: "src/jsx_syntax/props.rs",
    find: "                \"(@slots [$($s:tt)*] [] [$($rest:tt)*] r#{field}: $v:expr, $($tail:tt)*) => {{ NAME!(@slots [$($s)*] [$v] [$($rest)*] $($tail)*) }}\"",
    replace: "                \"(@slots [$($s:tt)*] [] [$($rest:tt)*] r#{field}__never: $v:expr, $($tail:tt)*) => {{ NAME!(@slots [$($s)*] [$v] [$($rest)*] $($tail)*) }}\"",
    tests: ["test/jsx-props.test.ts", "-t", "flattened props where they"],
  },
  {
    name: "flattened-field-and-props-both",
    breaks: "a flattened struct given whole and names of it too drops the names",
    file: "src/jsx_syntax/props.rs",
    find: "                    \"(@slots [$($s:tt)*] [$whole:expr] [$($rest:tt)+]) => {{ ::core::compile_error!(\\\"give `{field}` or the props of it, not both\\\") }}\"",
    replace: "                    \"(@slots [$($s:tt)*] [$whole:expr] [$($rest:tt)+] @never) => {{ ::core::compile_error!(\\\"give `{field}` or the props of it, not both\\\") }}\"",
    tests: ["test/jsx-props.test.ts", "-t", "flattened props where they"],
  },
  {
    name: "companion-flattened-last",
    breaks: "a companion writes its flattened field last, out of the struct's order, so an `if` of next/link's `Link` is `let tmp; if ..`",
    file: "src/jsx_syntax/props.rs",
    find: "let (before, after) = (values[..*at].concat(), values[*at..].concat());",
    replace: "let (before, after) = (values.concat(), String::new());",
    tests: ["test/jsx-react-dev.test.ts", "-t", "react.dev's SandboxShell shows its expand button"],
  },
  {
    name: "props-lifetimes-kept",
    breaks: "a props struct's `'static` form keeps its lifetimes, `LabelProps<'a>`, so a `thread_local!` component's caller still borrows for good",
    file: "src/jsx_syntax/props.rs",
    find: "                statics.push(\"'static\".to_string());",
    replace: "                statics.push(name.clone());",
    tests: ["test/jsx-evaluation-order.test.ts", "-t", "thread_local component takes props"],
  },
  {
    name: "companion-written-default",
    breaks: "a struct with a Default written for it, React's attributes of any element, has no companion, so a flattened one isn't built",
    file: "src/jsx_syntax/props.rs",
    find: "    let derives_default = derives_default || written_default.contains(&name);\n",
    replace: "",
    tests: ["test/jsx-types.test.ts", "-t", "JSX attributes take the values @types/react types them as"],
  },
  {
    name: "children-param-defaulted",
    breaks: "children of a type parameter left out are its `Default`, a type rustc can't infer",
    file: "src/jsx_syntax/props.rs",
    find: "        } else if field_name == \"children\" && last.as_ref().is_some_and(|last| params.contains(last)) {",
    replace: "        } else if false && last.as_ref().is_some_and(|last| params.contains(last)) {",
    tests: ["test/jsx-types.test.ts", "-t", "children of a type parameter"],
  },
  {
    name: "any-node-children-moved",
    breaks: "children declared any node, `&'a dyn ReactNode`, are given as they are, not by reference",
    file: "src/jsx_syntax/props.rs",
    find: "        let empty = if field_name == \"children\" && any_node(&field.ty) {",
    replace: "        let empty = if false && field_name == \"children\" && any_node(&field.ty) {",
    tests: ["test/jsx-types.test.ts", "-t", "children declared any node"]
  },
  {
    name: "optional-any-node-children-moved",
    breaks: "children declared an `Option` of any node are given as they are, not `Some` by reference",
    file: "src/jsx_syntax/props.rs",
    find: "        } else if field_name == \"children\" && optional_any_node {",
    replace: "        } else if false && optional_any_node {",
    tests: ["test/jsx-types.test.ts", "-t", "children declared any node"]
  },
];
