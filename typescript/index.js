// TypeScript's own parser and printer, and a model of declarations between
// them (ADR 0206): `read` gives a .ts or .d.ts file's declarations as the
// model, and `print` writes the model as TypeScript, each by TypeScript 7's
// API. rust-js writes its .d.ts with it, and reads with it what a package
// declares, as react's generator does @types/react's.

import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { API } from "typescript/unstable/async";
import { NodeFlags, SyntaxKind, TokenFlags } from "typescript/unstable/ast";
import * as f from "typescript/unstable/ast/factory";

/**
 * @typedef {{ declarations: Declaration[], header?: string }} Module
 *
 * @typedef {{ kind: "import", from: string, names: string[], typeOnly: boolean }
 *   | { kind: "interface", name: string, exported: boolean, declare: boolean, typeParameters: string[], extends: Type[], members: Member[] }
 *   | { kind: "type", name: string, exported: boolean, declare: boolean, typeParameters: string[], type: Type }
 *   | { kind: "function", name: string, exported: boolean, declare: boolean, typeParameters: string[], params: Param[], returns: Type }
 *   | { kind: "const", name: string, exported: boolean, declare: boolean, type: Type }
 *   | { kind: "export-default", name: string }
 *   | { kind: "namespace", name: string, exported: boolean, declare: boolean, declarations: Declaration[] }
 *   | { kind: "other", text: string }} Declaration
 *
 * @typedef {{ kind: "property", name: string, optional: boolean, readonly: boolean, type: Type }
 *   | { kind: "method", name: string, optional: boolean, typeParameters: string[], params: Param[], returns: Type }
 *   | { kind: "index", parameter: string, key: Type, type: Type }
 *   | { kind: "other", text: string }} Member
 *
 * @typedef {{ name: string, optional: boolean, rest: boolean, type: Type }} Param
 *
 * @typedef {{ kind: "keyword", keyword: string }
 *   | { kind: "literal", value: string | number | boolean }
 *   | { kind: "reference", name: string, args: Type[] }
 *   | { kind: "union", types: Type[] }
 *   | { kind: "intersection", types: Type[] }
 *   | { kind: "array", element: Type, readonly: boolean }
 *   | { kind: "tuple", elements: Type[] }
 *   | { kind: "function", typeParameters: string[], params: Param[], returns: Type }
 *   | { kind: "object", members: Member[] }
 *   | { kind: "other", text: string }} Type
 */

const KEYWORDS = {
  [SyntaxKind.StringKeyword]: "string",
  [SyntaxKind.NumberKeyword]: "number",
  [SyntaxKind.BigIntKeyword]: "bigint",
  [SyntaxKind.BooleanKeyword]: "boolean",
  [SyntaxKind.SymbolKeyword]: "symbol",
  [SyntaxKind.ObjectKeyword]: "object",
  [SyntaxKind.AnyKeyword]: "any",
  [SyntaxKind.UnknownKeyword]: "unknown",
  [SyntaxKind.UndefinedKeyword]: "undefined",
  [SyntaxKind.VoidKeyword]: "void",
  [SyntaxKind.NeverKeyword]: "never",
};
const KEYWORD_KINDS = Object.fromEntries(Object.entries(KEYWORDS).map(([kind, name]) => [name, Number(kind)]));

/**
 * A session of TypeScript's, over `files`, which `read` reads; `print`
 * needs none. Close it when done: it's a process of TypeScript's.
 * @param {string[]} [files]
 */
export async function open(files = []) {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-typescript-"));
  // A file of its own, so a project of no others isn't an empty one.
  const own = join(dir, "own.d.ts");
  writeFileSync(own, "export {};\n");
  const config = join(dir, "tsconfig.json");
  writeFileSync(
    config,
    JSON.stringify({
      compilerOptions: { noEmit: true, noLib: true, skipLibCheck: true, types: [] },
      files: [own, ...files.map((file) => resolve(file))],
    }),
  );
  const api = new API({ cwd: dir });
  const close = async () => {
    await api.close();
    rmSync(dir, { recursive: true, force: true });
  };
  let project;
  try {
    const snapshot = await api.updateSnapshot({ openProjects: [config] });
    project = snapshot.getProject(config);
  } catch (error) {
    await close();
    throw error;
  }
  return {
    /** The declarations `file`, one of the session's, makes. @returns {Promise<Module>} */
    async read(file) {
      const source = await project.program.getSourceFile(resolve(file));
      if (!source) throw new Error(`${file} isn't one of this session's files`);
      return { declarations: readStatements(source.statements, source.text) };
    },
    /** `module` as TypeScript: each declaration as TypeScript prints it. */
    async print(/** @type {Module} */ module) {
      const parts = [];
      let previous;
      for (const declaration of module.declarations) {
        const text = await project.emitter.printNode(statement(declaration));
        // Imports together, then a blank line before each declaration.
        const together = previous === "import" && declaration.kind === "import";
        parts.push(previous && !together ? `\n${text}` : text);
        previous = declaration.kind;
      }
      return `${module.header ? `${module.header}\n\n` : ""}${parts.join("\n")}\n`;
    },
    close,
  };
}

// ── Reading: TypeScript's syntax tree, as the model ───────────────────────

/** The text `node` spans. */
function sourceOf(node, text) {
  return text.slice(node.pos, node.end).trim();
}

function exported(node) {
  return !!node.modifiers?.some((m) => m.kind === SyntaxKind.ExportKeyword);
}

function declared(node) {
  return !!node.modifiers?.some((m) => m.kind === SyntaxKind.DeclareKeyword);
}

function typeParameters(node) {
  return (node.typeParameters ?? []).map((p) => p.name.text);
}

function nameOf(name, text) {
  return name.kind === SyntaxKind.Identifier || name.kind === SyntaxKind.StringLiteral
    ? name.text
    : sourceOf(name, text);
}

/** @returns {Declaration[]} */
function readStatements(statements, text) {
  const out = [];
  for (const node of statements) {
    switch (node.kind) {
      case SyntaxKind.ImportDeclaration: {
        const clause = node.importClause;
        const named = clause?.namedBindings;
        if (named?.kind !== SyntaxKind.NamedImports) {
          out.push({ kind: "other", text: sourceOf(node, text) });
          break;
        }
        out.push({
          kind: "import",
          from: node.moduleSpecifier.text,
          names: named.elements.map((e) => e.name.text),
          typeOnly: clause.phaseModifier === SyntaxKind.TypeKeyword || !!clause.isTypeOnly,
        });
        break;
      }
      case SyntaxKind.InterfaceDeclaration:
        out.push({
          kind: "interface",
          name: node.name.text,
          exported: exported(node),
          declare: declared(node),
          typeParameters: typeParameters(node),
          extends: (node.heritageClauses ?? []).flatMap((clause) => clause.types.map((t) => readType(t, text))),
          members: node.members.map((m) => readMember(m, text)),
        });
        break;
      case SyntaxKind.TypeAliasDeclaration:
        out.push({
          kind: "type",
          name: node.name.text,
          exported: exported(node),
          declare: declared(node),
          typeParameters: typeParameters(node),
          type: readType(node.type, text),
        });
        break;
      case SyntaxKind.FunctionDeclaration:
        out.push({
          kind: "function",
          name: node.name?.text ?? "default",
          exported: exported(node),
          declare: declared(node),
          typeParameters: typeParameters(node),
          params: node.parameters.map((p) => readParam(p, text)),
          returns: node.type ? readType(node.type, text) : { kind: "keyword", keyword: "void" },
        });
        break;
      case SyntaxKind.VariableStatement:
        for (const declaration of node.declarationList.declarations) {
          out.push({
            kind: "const",
            name: sourceOf(declaration.name, text),
            exported: exported(node),
            declare: declared(node),
            type: declaration.type ? readType(declaration.type, text) : { kind: "keyword", keyword: "any" },
          });
        }
        break;
      case SyntaxKind.ExportAssignment:
        out.push(
          node.isExportEquals
            ? { kind: "other", text: sourceOf(node, text) }
            : { kind: "export-default", name: sourceOf(node.expression, text) },
        );
        break;
      case SyntaxKind.ModuleDeclaration:
        if (node.body?.kind === SyntaxKind.ModuleBlock) {
          out.push({
            kind: "namespace",
            name: nameOf(node.name, text),
            exported: exported(node),
            declare: declared(node),
            declarations: readStatements(node.body.statements, text),
          });
          break;
        }
        out.push({ kind: "other", text: sourceOf(node, text) });
        break;
      default:
        out.push({ kind: "other", text: sourceOf(node, text) });
    }
  }
  return out;
}

function optional(node) {
  return (node.postfixToken ?? node.questionToken)?.kind === SyntaxKind.QuestionToken;
}

/** @returns {Member} */
function readMember(node, text) {
  switch (node.kind) {
    case SyntaxKind.PropertySignature:
      return {
        kind: "property",
        name: nameOf(node.name, text),
        optional: optional(node),
        readonly: !!node.modifiers?.some((m) => m.kind === SyntaxKind.ReadonlyKeyword),
        type: node.type ? readType(node.type, text) : { kind: "keyword", keyword: "any" },
      };
    case SyntaxKind.MethodSignature:
      return {
        kind: "method",
        name: nameOf(node.name, text),
        optional: optional(node),
        typeParameters: typeParameters(node),
        params: node.parameters.map((p) => readParam(p, text)),
        returns: node.type ? readType(node.type, text) : { kind: "keyword", keyword: "any" },
      };
    case SyntaxKind.IndexSignature: {
      const [parameter] = node.parameters;
      return {
        kind: "index",
        parameter: sourceOf(parameter.name, text),
        key: readType(parameter.type, text),
        type: readType(node.type, text),
      };
    }
    default:
      return { kind: "other", text: sourceOf(node, text) };
  }
}

/** @returns {Param} */
function readParam(node, text) {
  return {
    name: sourceOf(node.name, text),
    optional: optional(node),
    rest: !!node.dotDotDotToken,
    type: node.type ? readType(node.type, text) : { kind: "keyword", keyword: "any" },
  };
}

/** @returns {Type} */
function readType(node, text) {
  if (KEYWORDS[node.kind]) return { kind: "keyword", keyword: KEYWORDS[node.kind] };
  switch (node.kind) {
    case SyntaxKind.LiteralType: {
      const literal = node.literal;
      switch (literal.kind) {
        case SyntaxKind.StringLiteral:
          return { kind: "literal", value: literal.text };
        case SyntaxKind.NumericLiteral:
          return { kind: "literal", value: Number(literal.text) };
        case SyntaxKind.TrueKeyword:
          return { kind: "literal", value: true };
        case SyntaxKind.FalseKeyword:
          return { kind: "literal", value: false };
        case SyntaxKind.NullKeyword:
          return { kind: "keyword", keyword: "null" };
        default:
          return { kind: "other", text: sourceOf(node, text) };
      }
    }
    case SyntaxKind.TypeReference:
      return {
        kind: "reference",
        name: sourceOf(node.typeName, text),
        args: (node.typeArguments ?? []).map((t) => readType(t, text)),
      };
    case SyntaxKind.ExpressionWithTypeArguments:
      return {
        kind: "reference",
        name: sourceOf(node.expression, text),
        args: (node.typeArguments ?? []).map((t) => readType(t, text)),
      };
    case SyntaxKind.UnionType:
      return { kind: "union", types: node.types.map((t) => readType(t, text)) };
    case SyntaxKind.IntersectionType:
      return { kind: "intersection", types: node.types.map((t) => readType(t, text)) };
    case SyntaxKind.ArrayType:
      return { kind: "array", element: readType(node.elementType, text), readonly: false };
    case SyntaxKind.TypeOperator:
      if (node.operator === SyntaxKind.ReadonlyKeyword && node.type.kind === SyntaxKind.ArrayType) {
        return { kind: "array", element: readType(node.type.elementType, text), readonly: true };
      }
      return { kind: "other", text: sourceOf(node, text) };
    case SyntaxKind.TupleType:
      return { kind: "tuple", elements: node.elements.map((t) => readType(t, text)) };
    case SyntaxKind.ParenthesizedType:
      return readType(node.type, text);
    case SyntaxKind.FunctionType:
      return {
        kind: "function",
        typeParameters: typeParameters(node),
        params: node.parameters.map((p) => readParam(p, text)),
        returns: readType(node.type, text),
      };
    case SyntaxKind.TypeLiteral:
      return { kind: "object", members: node.members.map((m) => readMember(m, text)) };
    default:
      return { kind: "other", text: sourceOf(node, text) };
  }
}

// ── Printing: the model, as TypeScript's syntax tree ───────────────────────

const id = (name) => f.createIdentifier(name);
const string = (value) => f.createStringLiteral(value, TokenFlags.None);
const IDENTIFIER = /^[A-Za-z_$][A-Za-z0-9_$]*$/;

function modifiers(declaration) {
  const tokens = [];
  if (declaration.exported) tokens.push(f.createToken(SyntaxKind.ExportKeyword));
  if (declaration.declare) tokens.push(f.createToken(SyntaxKind.DeclareKeyword));
  return tokens.length ? tokens : undefined;
}

function typeParameterNodes(names) {
  return names?.length ? names.map((name) => f.createTypeParameterDeclaration(undefined, id(name))) : undefined;
}

/** A model's name, as TypeScript has one: `React.ReactNode` as written. */
function entity(name) {
  return id(name);
}

/** @param {Declaration} declaration */
function statement(declaration) {
  switch (declaration.kind) {
    case "import":
      return f.createImportDeclaration(
        undefined,
        f.createImportClause(
          declaration.typeOnly ? SyntaxKind.TypeKeyword : undefined,
          undefined,
          f.createNamedImports(declaration.names.map((name) => f.createImportSpecifier(false, undefined, id(name)))),
        ),
        string(declaration.from),
      );
    case "interface":
      return f.createInterfaceDeclaration(
        modifiers(declaration),
        id(declaration.name),
        typeParameterNodes(declaration.typeParameters),
        declaration.extends.length
          ? [
              f.createHeritageClause(
                SyntaxKind.ExtendsKeyword,
                declaration.extends.map((t) =>
                  f.createExpressionWithTypeArguments(
                    entity(t.name),
                    t.args?.length ? t.args.map(typeNode) : undefined,
                  ),
                ),
              ),
            ]
          : undefined,
        declaration.members.map(member),
      );
    case "type":
      return f.createTypeAliasDeclaration(
        modifiers(declaration),
        id(declaration.name),
        typeParameterNodes(declaration.typeParameters),
        typeNode(declaration.type),
      );
    case "function":
      return f.createFunctionDeclaration(
        modifiers(declaration),
        undefined,
        id(declaration.name),
        typeParameterNodes(declaration.typeParameters),
        declaration.params.map(param),
        typeNode(declaration.returns),
      );
    case "const":
      return f.createVariableStatement(
        modifiers(declaration),
        f.createVariableDeclarationList(
          [f.createVariableDeclaration(id(declaration.name), undefined, typeNode(declaration.type))],
          NodeFlags.Const,
        ),
      );
    case "export-default":
      return f.createExportAssignment(undefined, false, undefined, id(declaration.name));
    case "namespace":
      return f.createModuleDeclaration(
        modifiers(declaration),
        SyntaxKind.NamespaceKeyword,
        id(declaration.name),
        f.createModuleBlock(declaration.declarations.map(statement)),
      );
    default:
      throw new Error(`a ${declaration.kind} declaration isn't one TypeScript is given`);
  }
}

function propertyName(name) {
  return IDENTIFIER.test(name) ? id(name) : string(name);
}

/** @param {Member} m */
function member(m) {
  switch (m.kind) {
    case "property":
      return f.createPropertySignatureDeclaration(
        m.readonly ? [f.createToken(SyntaxKind.ReadonlyKeyword)] : undefined,
        propertyName(m.name),
        m.optional ? f.createToken(SyntaxKind.QuestionToken) : undefined,
        typeNode(m.type),
        undefined,
      );
    case "method":
      return f.createMethodSignatureDeclaration(
        undefined,
        propertyName(m.name),
        m.optional ? f.createToken(SyntaxKind.QuestionToken) : undefined,
        typeParameterNodes(m.typeParameters),
        m.params.map(param),
        typeNode(m.returns),
      );
    case "index":
      return f.createIndexSignatureDeclaration(
        undefined,
        [f.createParameterDeclaration(undefined, undefined, id(m.parameter), undefined, typeNode(m.key))],
        typeNode(m.type),
      );
    default:
      throw new Error(`a ${m.kind} member isn't one TypeScript is given`);
  }
}

/** @param {Param} p */
function param(p) {
  return f.createParameterDeclaration(
    undefined,
    p.rest ? f.createToken(SyntaxKind.DotDotDotToken) : undefined,
    id(p.name),
    p.optional ? f.createToken(SyntaxKind.QuestionToken) : undefined,
    typeNode(p.type),
  );
}

/** `t`, in parentheses where an array's element needs them: `(string | undefined)[]`. */
function element(t) {
  const node = typeNode(t);
  return ["union", "intersection", "function"].includes(t.kind) ? f.createParenthesizedTypeNode(node) : node;
}

/** @param {Type} t */
function typeNode(t) {
  switch (t.kind) {
    case "keyword":
      if (t.keyword === "null") return f.createLiteralTypeNode(f.createToken(SyntaxKind.NullKeyword));
      if (!(t.keyword in KEYWORD_KINDS)) throw new Error(`\`${t.keyword}\` isn't a keyword TypeScript has`);
      return f.createKeywordTypeNode(KEYWORD_KINDS[t.keyword]);
    case "literal":
      switch (typeof t.value) {
        case "string":
          return f.createLiteralTypeNode(string(t.value));
        case "number":
          return f.createLiteralTypeNode(f.createNumericLiteral(String(t.value), TokenFlags.None));
        default:
          return f.createLiteralTypeNode(f.createToken(t.value ? SyntaxKind.TrueKeyword : SyntaxKind.FalseKeyword));
      }
    case "reference":
      return f.createTypeReferenceNode(entity(t.name), t.args?.length ? t.args.map(typeNode) : undefined);
    case "union":
      return f.createUnionTypeNode(t.types.map(typeNode));
    case "intersection":
      return f.createIntersectionTypeNode(t.types.map(typeNode));
    case "array": {
      const array = f.createArrayTypeNode(element(t.element));
      return t.readonly ? f.createTypeOperatorNode(SyntaxKind.ReadonlyKeyword, array) : array;
    }
    case "tuple":
      return f.createTupleTypeNode(t.elements.map(typeNode));
    case "function":
      return f.createFunctionTypeNode(typeParameterNodes(t.typeParameters), t.params.map(param), typeNode(t.returns));
    case "object":
      return f.createTypeLiteralNode(t.members.map(member));
    case "other":
      // What the model doesn't take apart, as it was written.
      return f.createTypeReferenceNode(id(t.text), undefined);
    default:
      throw new Error(`a ${t.kind} type isn't one TypeScript is given`);
  }
}
