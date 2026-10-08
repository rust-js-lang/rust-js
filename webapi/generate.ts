// Generate src/lib.rs, the `webapi` crate, from W3C's WebIDL (ADR 0024).
//
//   cd web && bun install && bun generate.ts
//
// Every rule about what gets in is in this file: which interfaces
// (INTERFACES), how WebIDL types become Rust (`rustType`), and the names.
// A member is generated only if rust-js supports all of its types; the rest
// are counted and skipped, and rerunning after rust-js grows picks them up.

import idl from "@webref/idl";
import { typescript } from "./coverage";
import webref from "@webref/idl/package.json" with { type: "json" };
import webrefEvents from "@webref/events";
import eventsPackage from "@webref/events/package.json" with { type: "json" };
import webrefElements from "@webref/elements";
import elementsPackage from "@webref/elements/package.json" with { type: "json" };

// The specs to read. Partial interfaces and mixins from these are merged in.
const SPECS = ["dom", "html", "hr-time", "uievents", "pointerevents", "cssom", "cssom-view", "geometry", "fetch", "encoding", "wasm-js-api", "wasm-web-api", "xhr", "streams", "touch-events", "FileAPI", "clipboard-apis", "css-animations", "css-transitions", "SVG", "svg-paths", "svg-animations", "filter-effects", "css-masking", "intersection-observer", "url"];

// The everyday DOM. Members that use any other interface are skipped.
const INTERFACES = [
  // dom
  "EventTarget", "Event", "Node", "CharacterData", "Text", "Comment", "Element", "Document",
  "DocumentFragment", "DOMTokenList", "NodeList", "HTMLCollection", "AbortController", "AbortSignal",
  // html
  "HTMLElement", "HTMLAnchorElement", "HTMLButtonElement", "HTMLDivElement", "HTMLFormElement",
  "HTMLHeadingElement", "HTMLImageElement", "HTMLInputElement", "HTMLLabelElement", "HTMLLIElement",
  "HTMLOListElement", "HTMLOptionElement", "HTMLOutputElement", "HTMLParagraphElement",
  "HTMLSelectElement", "HTMLSpanElement", "HTMLTextAreaElement", "HTMLUListElement",
  "HTMLTableElement", "HTMLTableSectionElement", "HTMLTableRowElement", "HTMLTableCellElement", "HTMLIFrameElement",
  "HTMLCanvasElement", "HTMLDetailsElement",
  // Every element of HTML, so each tag is of its own (ADR 0224): what its
  // attributes are, as @types/react has them, follows.
  "HTMLHtmlElement", "HTMLHeadElement", "HTMLTitleElement", "HTMLBaseElement", "HTMLLinkElement",
  "HTMLMetaElement", "HTMLStyleElement", "HTMLBodyElement", "HTMLHRElement", "HTMLPreElement",
  "HTMLQuoteElement", "HTMLMenuElement", "HTMLDListElement", "HTMLDataElement", "HTMLTimeElement",
  "HTMLBRElement", "HTMLModElement", "HTMLPictureElement", "HTMLSourceElement", "HTMLEmbedElement",
  "HTMLObjectElement", "HTMLVideoElement", "HTMLAudioElement", "HTMLTrackElement", "HTMLMediaElement",
  "HTMLMapElement", "HTMLAreaElement", "HTMLTableCaptionElement", "HTMLTableColElement", "HTMLDataListElement",
  "HTMLOptGroupElement", "HTMLProgressElement", "HTMLMeterElement", "HTMLFieldSetElement", "HTMLLegendElement",
  "HTMLSelectedContentElement", "HTMLDialogElement", "HTMLScriptElement", "HTMLTemplateElement", "HTMLSlotElement",
  "Window", "Location", "History", "Storage", "DataTransfer", "DragEvent", "ToggleEvent", "MessageEvent", "SubmitEvent",
  // hr-time: `window.performance`, the page's clock
  "Performance",
  // uievents
  "UIEvent", "FocusEvent", "MouseEvent", "KeyboardEvent", "InputEvent", "WheelEvent", "CompositionEvent",
  // What React's events wrap, @types/react's `nativeEvent` (ADR 0227)
  "TouchEvent", "AnimationEvent", "TransitionEvent", "ClipboardEvent",
  // pointerevents: a click is a `PointerEvent`
  "PointerEvent",
  // cssom
  "CSSStyleDeclaration", "CSSStyleProperties",
  // cssom-view, geometry: where things are on the page
  "DOMRectReadOnly", "DOMRect", "MediaQueryList", "MediaQueryListEvent",
  // intersection-observer: when an element comes into view, or goes out
  "IntersectionObserver", "IntersectionObserverEntry",
  // fetch: `window::fetch`, and what it gives back
  "Headers", "Request", "Response",
  // xhr, streams, touch-events: what React's forms, server rendering and touch events use
  "FormData", "ReadableStream", "Touch", "TouchList",
  // encoding: text to bytes and back
  "TextEncoder", "TextDecoder",
  // wasm-js-api: `WebAssembly.Module` and friends
  "Module", "Instance", "Memory",
  // FileAPI: raw data, a fetch's body or a download's
  "Blob", "File", "URL",
  // html, clipboard-apis: the browser, and what's copied
  "Navigator", "Clipboard", "ClipboardItem",
  // SVG, svg-animations, filter-effects, css-masking: each SVG element, and
  // what they extend, so each SVG tag is of its own (`svg_tags`).
  "SVGElement", "SVGGraphicsElement", "SVGGeometryElement", "SVGSVGElement", "SVGGElement", "SVGDefsElement",
  "SVGSymbolElement", "SVGUseElement", "SVGSwitchElement", "SVGTitleElement", "SVGDescElement", "SVGMetadataElement",
  "SVGStyleElement", "SVGScriptElement", "SVGPathElement", "SVGRectElement", "SVGCircleElement", "SVGEllipseElement",
  "SVGLineElement", "SVGPolylineElement", "SVGPolygonElement", "SVGTextContentElement", "SVGTextPositioningElement",
  "SVGTextElement", "SVGTSpanElement", "SVGTextPathElement", "SVGImageElement", "SVGForeignObjectElement",
  "SVGMarkerElement", "SVGGradientElement", "SVGLinearGradientElement", "SVGRadialGradientElement", "SVGStopElement",
  "SVGPatternElement", "SVGAElement", "SVGViewElement", "SVGAnimationElement", "SVGAnimateElement", "SVGSetElement",
  "SVGAnimateMotionElement", "SVGMPathElement", "SVGAnimateTransformElement", "SVGFilterElement", "SVGFEBlendElement",
  "SVGFEColorMatrixElement", "SVGFEComponentTransferElement", "SVGComponentTransferFunctionElement", "SVGFEFuncRElement",
  "SVGFEFuncGElement", "SVGFEFuncBElement", "SVGFEFuncAElement", "SVGFECompositeElement", "SVGFEConvolveMatrixElement",
  "SVGFEDiffuseLightingElement", "SVGFEDisplacementMapElement", "SVGFEDropShadowElement", "SVGFEFloodElement",
  "SVGFEGaussianBlurElement", "SVGFEImageElement", "SVGFEMergeElement", "SVGFEMergeNodeElement", "SVGFEMorphologyElement",
  "SVGFEOffsetElement", "SVGFESpecularLightingElement", "SVGFETileElement", "SVGFETurbulenceElement",
  "SVGFEDistantLightElement", "SVGFEPointLightElement", "SVGFESpotLightElement", "SVGClipPathElement", "SVGMaskElement",
];
// And every other class TypeScript's DOM has (`@types/web`), from whichever
// spec WebIDL has it in (ADR 0281).
const typescriptDom = await typescript();
const defined = new Set(Object.values(await idl.parseAll() as Record<string, { type: string; name: string; partial?: boolean }[]>)
  .flat().filter((d) => d.type === "interface" && !d.partial).map((d) => d.name));
INTERFACES.push(...[...typescriptDom.keys()].filter((name) => defined.has(name) && !INTERFACES.includes(name)).sort());
const known = new Set(INTERFACES);

// Members written by hand, for results the generator can't type: a union
// with an interface it doesn't bind.
const EXTRA: Record<string, Fn[]> = {
  MessageEvent: [
    {
      name: "source",
      jsName: "get source",
      params: ["this: &MessageEvent"],
      result: "Option<&'static JsObject>",
      doc: [
        "[MDN](https://developer.mozilla.org/docs/Web/API/MessageEvent/source): what sent it, a window,",
        "a `MessagePort` or a `ServiceWorker`, as an object: `js::object::is` tells which.",
      ],
    },
  ],
  FormData: [
    {
      name: "get",
      jsName: "get",
      params: ["this: &FormData", "name: &str"],
      result: "Option<String>",
      doc: [
        "[MDN](https://developer.mozilla.org/docs/Web/API/FormData/get): a text field's value.",
        "A file field's value is a `File`, which this doesn't bind.",
      ],
    },
    {
      name: "get_all",
      jsName: "getAll",
      params: ["this: &FormData", "name: &str"],
      result: "Vec<String>",
      doc: ["[MDN](https://developer.mozilla.org/docs/Web/API/FormData/getAll): every text value of a field."],
    },
  ],
};

// Namespaces: a module of functions, like `web_assembly::compile`.
const NAMESPACES = ["WebAssembly"];

// JS's own types that WebIDL uses, declared by hand at the crate root.
const BUILTINS = new Set(["ArrayBuffer", "Uint8Array"]);

// A dictionary field's type, where WebIDL's is one Rust can't take: a
// `HeadersInit` is a sequence or a record, and `fetch` takes a `Headers` too.
const FIELD_TYPES: Record<string, string> = { "RequestInit.headers": "&'a Headers" };

// The globals at the crate root: `document`, `window`, `performance`, which
// a worker has too, `navigator`, and `history`, the page's.
const GLOBALS: [string, string][] = [["document", "Document"], ["window", "Window"], ["performance", "Performance"], ["navigator", "Navigator"], ["history", "History"]];

// ── Reading the IDL ─────────────────────────────────────────────────────

type IdlType = { idlType: string | IdlType[]; nullable: boolean; union: boolean; generic: string };
type Arg = { name: string; idlType: IdlType; optional: boolean; variadic: boolean };
type Member = {
  type: string;
  name?: string;
  special?: string;
  readonly?: boolean;
  idlType?: IdlType;
  arguments?: Arg[];
  extAttrs?: { name: string }[];
  required?: boolean;
};
type Def = {
  type: string;
  name: string;
  partial?: boolean;
  inheritance?: string;
  members?: Member[];
  target?: string;
  includes?: string;
  idlType?: IdlType;
  arguments?: Arg[];
  extAttrs?: { name: string; rhs?: { value: string } }[];
};

const all = await idl.parseAll() as Record<string, Def[]>;
const anyClassHas = new Set([...typescriptDom.values()].flatMap((c) => [...c.members]));
// Every spec's: what the specs above have, as they have it, and of the
// others' members those TypeScript's DOM has, not one it leaves out as
// still an experiment (ADR 0281).
// (webidl2's definitions are objects with getters: the members left out
// are marked, not the definitions copied.)
const read: Def[] = Object.values(all).flat();
const leftOut = new WeakSet<Member>();
for (const [spec, defs] of Object.entries(all)) {
  if (SPECS.includes(spec)) continue;
  for (const d of defs) {
    const ts = d.type === "interface" ? typescriptDom.get(d.name) : undefined;
    // A mixin's, as a class that includes it has them.
    const kept = (m: Member) =>
      m.type === "constructor"
        ? !!ts?.ctor
        : !!m.name && (d.type === "interface mixin" ? anyClassHas.has(m.name) : !!ts && (m.special === "static" ? ts.statics : ts.members).has(m.name));
    for (const m of d.members ?? []) if (!kept(m)) leftOut.add(m);
  }
}
const everywhere: Def[] = Object.values(all).flat();

// Names that are strings (enums) or other types (typedefs), from any spec.
const enums = new Set(everywhere.filter((d) => d.type === "enum").map((d) => d.name));
const typedefs = new Map(everywhere.filter((d) => d.type === "typedef").map((d) => [d.name, d.idlType!]));
const dictionaries = new Map(everywhere.filter((d) => d.type === "dictionary" && !d.partial).map((d) => [d.name, d]));
const callbacks = new Map(everywhere.filter((d) => d.type === "callback").map((d) => [d.name, d]));

type Interface = {
  name: string;
  parent?: string;
  members: { member: Member; from: string }[];
  constructible: boolean;
  /** `[LegacyNamespace=WebAssembly]`: JS calls it `WebAssembly.Module`. */
  legacyNamespace?: string;
  /** A `namespace`: functions, and no type. */
  isNamespace?: boolean;
  /** The global scope's: functions called bare, `fetch(url)` (ADR 0269). */
  isGlobal?: boolean;
};
const interfaces = new Map<string, Interface>();
const namespaces = new Map<string, Interface>();
const mixins = new Map<string, Member[]>();
const includes = new Map<string, string[]>();

for (const d of read) {
  if (d.type === "interface" && known.has(d.name)) {
    const i = interfaces.get(d.name) ?? { name: d.name, members: [], constructible: false };
    if (!d.partial) {
      i.parent = d.inheritance ?? undefined;
      // `[HTMLConstructor]` elements can't be made with `new`.
      i.constructible = !(d.extAttrs ?? []).some((a) => a.name === "HTMLConstructor");
      i.legacyNamespace = (d.extAttrs ?? []).find((a) => a.name === "LegacyNamespace")?.rhs?.value;
    }
    i.members.push(...(d.members ?? []).filter((m) => !leftOut.has(m)).map((member) => ({ member, from: d.name })));
    interfaces.set(d.name, i);
  } else if (d.type === "namespace" && NAMESPACES.includes(d.name)) {
    const n = namespaces.get(d.name) ?? { name: d.name, members: [], constructible: false, isNamespace: true };
    n.members.push(...(d.members ?? []).map((member) => ({ member, from: d.name })));
    namespaces.set(d.name, n);
  } else if (d.type === "interface mixin") {
    mixins.set(d.name, [...(mixins.get(d.name) ?? []), ...(d.members ?? []).filter((m) => !leftOut.has(m))]);
  } else if (d.type === "includes") {
    includes.set(d.target!, [...(includes.get(d.target!) ?? []), d.includes!]);
  }
}
for (const [target, names] of includes) {
  const i = interfaces.get(target);
  for (const name of names) i?.members.push(...(mixins.get(name) ?? []).map((member) => ({ member, from: target })));
}
// What every JS global scope has, a window's, a worker's or Node's: its
// functions, `fetch` and `queueMicrotask`, called bare, where a window's
// are `window.fetch` (ADR 0269). Not its timers, whose handler here is
// text to run, which the builtins crate's take as closures (ADR 0102).
const TIMERS = ["setTimeout", "setInterval", "clearTimeout", "clearInterval"];
const globalScope: Interface = {
  name: "WindowOrWorkerGlobalScope",
  members: (mixins.get("WindowOrWorkerGlobalScope") ?? [])
    .filter((member) => member.type === "operation" && !TIMERS.includes(member.name ?? ""))
    .map((member) => ({ member, from: "WindowOrWorkerGlobalScope" })),
  constructible: false,
  isNamespace: true,
  isGlobal: true,
};
const missing = INTERFACES.filter((name) => !interfaces.has(name));
if (missing.length) throw new Error(`not in ${SPECS.join(", ")}: ${missing.join(", ")}`);

// The dictionaries a function gives, through a promise, a union, a sequence
// or another dictionary: each a struct that owns what it holds, which one
// a function takes is too, by value, where it's both, not a struct that
// borrows.
const givenDictionaries = new Set<string>();
const gives = (t: IdlType | undefined): void => {
  if (!t) return;
  if (Array.isArray(t.idlType)) return t.idlType.forEach(gives);
  const name = t.idlType as string;
  const aliased = typedefs.get(name);
  if (aliased) return gives(aliased);
  const d = dictionaries.get(name);
  if (!d || givenDictionaries.has(name)) return;
  givenDictionaries.add(name);
  for (let p: Def | undefined = d; p; p = p.inheritance ? dictionaries.get(p.inheritance) : undefined) {
    for (const m of p.members ?? []) gives(m.idlType);
  }
};
for (const i of interfaces.values()) {
  for (const { member } of i.members) if (member.type === "attribute" || member.type === "operation") gives(member.idlType);
}

// ── Names ───────────────────────────────────────────────────────────────

/** `HTMLInputElement` → `["HTML", "Input", "Element"]`, `innerHTML` → `["inner", "HTML"]`. */
// "HTML" then a one-letter word (`HTMLIFrameElement`) would read as "HTMLI":
// split it off first, so its module is `html_iframe_element`.
const words = (name: string) =>
  name.replace(/^HTML(?=[A-Z][A-Z])/, "Html").match(/[A-Z]+(?![a-z])|[A-Z]?[a-z0-9]+/g) ?? [name];

/** An interface's name with its namespace, if it has one: `WebAssemblyModule`. */
const qualified = (name: string) => (interfaces.get(name)?.legacyNamespace ?? "") + name;

/** What JS calls an interface: `WebAssembly.Module`, `Element`. */
const jsName = (i: Interface) => (i.legacyNamespace ? `${i.legacyNamespace}.${i.name}` : i.name);

/** Types, as WebIDL and TypeScript's DOM lib name them: `HTMLInputElement`,
 * `DOMRect`; one of a namespace with it, `Module` → `WebAssemblyModule`. */
const typeName = (name: string) => qualified(name);

const KEYWORDS = new Set(
  ("as async await box break const continue crate do dyn else enum extern false final fn for gen if impl in " +
    "let loop macro match mod move mut override priv pub ref return self Self static struct super trait true " +
    "try type typeof unsafe unsized use virtual where while yield abstract become").split(" "),
);

/** `getElementById` → `get_element_by_id`. */
const snakeWords = (name: string) => words(name).map((w) => w.toLowerCase()).join("_");

/** Functions, modules and parameters, with a `_` after a Rust keyword: `type` → `type_`. */
const snake = (name: string) => {
  const s = snakeWords(name);
  return KEYWORDS.has(s) ? `${s}_` : s;
};

// ── Types ───────────────────────────────────────────────────────────────

const STRINGS = new Set(["DOMString", "USVString", "CSSOMString", "ByteString"]);
const NUMBERS: Record<string, string> = {
  boolean: "bool", byte: "i8", octet: "u8", short: "i16", "unsigned short": "u16",
  long: "i32", "unsigned long": "u32", double: "f64", "unrestricted double": "f64",
  // A `long long` is a JS number, not a `BigInt` an `i64` would be, so
  // exact to 2^53 (ADR 0281); a `float`, one an `f32` holds.
  "long long": "f64", "unsigned long long": "f64", float: "f32", "unrestricted float": "f32",
};

type Position = "param" | "result";

/** What a parameter typed `any` is, until its function names its type parameter. */
const ANY = "<any>";

// The `any` parameters the browser copies with its structured clone, by
// operation and parameter: a value only of a type it copies as it is,
// `js::StructuredClone` (ADR 0225). WebIDL doesn't say which they are.
const CLONED = new Set(["postMessage.message", "pushState.data", "replaceState.data", "structuredClone.value"]);

/** The Rust type for a (non-union) WebIDL type, or why there isn't one. */
function rustType(t: IdlType, at: Position): string | { skip: string } {
  if (t.union) return { skip: "union" };
  // A promise a function returns is `.await`ed in Rust (ADR 0029). One it
  // takes is passed as it is: `compile_streaming(window::fetch(..))`.
  if (t.generic === "Promise") {
    const inner = rustType((t.idlType as IdlType[])[0], "result");
    return typeof inner === "string" ? `Promise<${inner}>` : inner;
  }
  // A sequence a function takes is a slice, a JS array of its items as
  // they are (ADR 0219): `new Blob([text, "!"])` of `&[BlobPart]`.
  if ((t.generic === "sequence" || t.generic === "FrozenArray") && at === "param") {
    const item = paramType((t.idlType as IdlType[])[0]);
    return item ? `&[${item}]` : { skip: t.generic };
  }
  // One a function gives is a `Vec`, a new array each time; a frozen
  // array, the same one, which JS won't change, a slice of it.
  if ((t.generic === "sequence" || t.generic === "FrozenArray") && at === "result") {
    const inner = (t.idlType as IdlType[])[0];
    const item = rustType(inner, "result");
    if (typeof item !== "string") return item;
    const each = inner.nullable ? `Option<${item}>` : item;
    return t.generic === "sequence" ? `Vec<${each}>` : `&'static [${each}]`;
  }
  if (t.generic) return { skip: t.generic };
  const name = t.idlType as string;
  const aliased = typedefs.get(name);
  // (webidl2 types are objects with getters: pass them on as they are.)
  if (aliased) return rustType(aliased, at);
  if (name === "undefined") return at === "result" ? "()" : { skip: "undefined parameter" };
  // A `WindowProxy` is a `Window`, as a script sees it: `frame.contentWindow`.
  if (name === "WindowProxy") return rustType({ ...t, idlType: "Window" }, at);
  if (NUMBERS[name]) return NUMBERS[name];
  if (STRINGS.has(name) || enums.has(name)) return at === "param" ? "&str" : "String";
  if (name === "EventListener" && at === "param") return "Box<dyn FnMut(&Event)>";
  // A callback a function takes is a closure, given what JS calls it with
  // as a function's parameters are, `None` of a `null` one:
  // `new IntersectionObserver(|entries, observer| ..)`. An event handler
  // attribute's, `onclick`, isn't: a listener is `add_event_listener`'s,
  // typed by its event.
  const callback = name.endsWith("EventHandlerNonNull") ? undefined : callbacks.get(name);
  if (callback && at === "param") {
    const args = (callback.arguments ?? []).map((a) => {
      const type = paramType(a.idlType);
      return type && a.idlType.nullable ? `Option<${type}>` : type;
    });
    const result = rustType(callback.idlType!, "result");
    if (args.some((a) => !a || a === ANY) || typeof result !== "string") return { skip: "callback" };
    return `Box<dyn FnMut(${args.join(", ")})${result === "()" ? "" : ` -> ${result}`}>`;
  }
  // A value of any shape a function gives is the js crate's `Unknown`, or
  // `None` of `undefined` and `null` (ADR 0225): `response.json()`.
  // One it takes is of any type, as JS has it: a generic `M`, `message: M`.
  if (name === "any") return at === "result" ? "Option<&'static Unknown>" : ANY;
  // Any JS object: a Rust value of any type in, an opaque object out.
  if (name === "object") return at === "param" ? "&dyn core::any::Any" : "&'static JsObject";
  const dictionary = dictionaries.get(name);
  if (dictionary && at === "result") return dictionaryType(dictionary);
  if (dictionary && at === "param") return paramDictionary(dictionary);
  if (known.has(name) || BUILTINS.has(name)) return at === "param" ? `&${typeName(name)}` : `&'static ${typeName(name)}`;
  return { skip: name };
}

/** A field of a dictionary: its Rust and JS names, and type. */
type Field = { rust: string; js: string; type: string; optional: boolean };

/** The fields of each dictionary a result uses. */
const usedDictionaries = new Map<string, Field[]>();

/**
 * A dictionary a function returns is a Rust struct, which rust-js makes a
 * plain JS object (ADR 0020): its fields are read as they are, an optional
 * one an `Option`, a field Rust can't take left out.
 */
function dictionaryType(d: Def): string | { skip: string } {
  if (usedDictionaries.has(d.name)) return typeName(d.name);
  if (paramDictionaries.has(d.name)) return { skip: `${d.name} as a parameter too` };
  // Its own name first, for a field of its own type.
  const fields: Field[] = [];
  usedDictionaries.set(d.name, fields);
  for (const m of dictionaryMembers(d)) {
    const rust = rustType(m.idlType!, "result");
    if (typeof rust !== "string") continue;
    fields.push({ rust: snake(m.name!), js: m.name!, type: m.idlType!.nullable ? `Option<${rust}>` : rust, optional: !m.required });
  }
  // One of no fields Rust can take is an empty struct: the function gives it,
  // and nothing of it is read.
  return typeName(d.name);
}


/** The fields of each dictionary a function takes, and whether they borrow. */
const paramDictionaries = new Map<string, { fields: Field[]; borrows: boolean }>();

/** A dictionary's members, its parents' first: `AddEventListenerOptions` has `capture`. */
function dictionaryMembers(d: Def): Member[] {
  const parent = d.inheritance ? dictionaries.get(d.inheritance) : undefined;
  return [...(parent ? dictionaryMembers(parent) : []), ...((d.members ?? []) as Member[])];
}

/**
 * A dictionary a function takes is a Rust struct, as ReScript's is a record
 * of optional fields (ADR 0102): an optional member is an `Option`, `None`
 * unless given, which JS reads as not given, and with none required it has
 * `Default`: `RequestInit { method: Some("POST"), ..Default::default() }`.
 * What it borrows lives for `'a`. A member of a type Rust can't take is
 * left out; one of a union is its enum (ADR 0215).
 */
function paramDictionary(d: Def): string | { skip: string } {
  // One a function gives too is the result's struct, given by value.
  if (givenDictionaries.has(d.name) || usedDictionaries.has(d.name)) return dictionaryType(d);
  let known = paramDictionaries.get(d.name);
  if (!known) {
    const fields: Field[] = [];
    for (const m of dictionaryMembers(d)) {
      const chosen = FIELD_TYPES[`${d.name}.${m.name}`] ?? paramType(m.idlType!);
      // A field typed `any` is left out, as a struct field can't be generic.
      if (!chosen || chosen.includes("dyn core::any::Any") || chosen.includes(ANY)) continue;
      // A borrow in a field lives as long as the struct's: `&'a str`.
      const type = chosen.replace(/&(?!')/g, "&'a ").replace(/<'_>/g, "<'a>");
      fields.push({ rust: snake(m.name!), js: m.name!, type, optional: !m.required });
    }
    if (fields.length === 0) return { skip: d.name };
    known = { fields, borrows: fields.some((f) => f.type.includes("'a")) };
    paramDictionaries.set(d.name, known);
  }
  return `${typeName(d.name)}${known.borrows ? "<'_>" : ""}`;
}

/** A parameter type that's a dictionary: `RequestInit<'_>`. */
const isDictionary = (rust: string) =>
  [...paramDictionaries.keys(), ...usedDictionaries.keys()].some((n) => rust.replace(/<'_>$/, "") === typeName(n));

/** The Rust types a parameter can take: one per supported member of a union. */
function alternatives(t: IdlType): string[] {
  // A typedef of a union, like `RequestInfo`, is that union, and a union
  // inside a union (`ArrayBufferView` in `BufferSource`) is its members.
  const aliased = !t.union && !t.generic && typedefs.get(t.idlType as string);
  if (aliased) return alternatives(aliased);
  if (t.union) return (t.idlType as IdlType[]).flatMap(alternatives);
  const rust = rustType(t, "param");
  return typeof rust === "string" ? [rust] : [];
}

/** Is `t` a union, written out or through a typedef? */
function isUnion(t: IdlType): boolean {
  const aliased = !t.union && !t.generic && typedefs.get(t.idlType as string);
  return t.union || (!!aliased && isUnion(aliased));
}

/** `&str` → `str`, `&HTMLElement` → `html_element`: for `instantiate_with_web_assembly_module`. */
const suffix = (rust: string) => snake(rust.replace(/^&/, "").replace(/^Box<dyn FnMut.*$/, "listener"));

// ── Unions ──────────────────────────────────────────────────────────────

/**
 * What JS tells a value of a parameter type by, as an untagged enum's
 * variant is told (ADR 0214): a primitive's `typeof`, or a class's own name.
 * A dictionary is an object. `object`, any object, tells nothing apart.
 */
function kindOf(rust: string): string | null {
  if (rust === "&str") return "string";
  if (rust === "bool") return "boolean";
  if (Object.values(NUMBERS).includes(rust)) return "number";
  if (rust.startsWith("Box<dyn FnMut")) return "function";
  if (rust.startsWith("&[")) return "array";
  if (rust.includes("dyn core::any::Any")) return null;
  if (isDictionary(rust)) return "object";
  return rust.replace(/^&/, "").replace(/<.*$/, "");
}

/** A variant's name: its type's, or of a primitive its kind's, `Str`. */
function variantName(rust: string): string {
  const kind = kindOf(rust);
  if (kind === "string") return "Str";
  if (kind === "boolean") return "Bool";
  if (kind === "number") return "Number";
  if (kind === "function") return "Listener";
  if (kind === "array") return "List";
  return rust.replace(/^&/, "").replace(/<.*$/, "");
}

/** What TypeScript calls a variant's type: `string`, `Node`, `HTMLElement`. */
function tsName(rust: string): string {
  const kind = kindOf(rust);
  if (kind === "string" || kind === "boolean" || kind === "number") return kind;
  if (kind === "function") return "EventListener";
  const name = variantName(rust);
  const idl = INTERFACES.find((n) => typeName(n) === name);
  return idl ? jsName(interfaces.get(idl)!) : name;
}

/** A WebIDL union as an untagged enum (ADR 0215): a variant per kind. */
type Union = { name: string; variants: { name: string; type: string; ts: string }[]; borrows: boolean };
const unions = new Map<string, Union>();

/**
 * A union's enum: its members, of each kind JS tells apart the first, as
 * TypeScript's union of them. Named as its typedef is, `BodyInit`, or after
 * its members, `NodeOrStr`. None, of fewer than two members Rust can take.
 */
function unionOf(t: IdlType): Union | null {
  const kinds = new Set<string>();
  const variants: Union["variants"] = [];
  for (const alt of alternatives(t)) {
    const kind = kindOf(alt);
    if (!kind || kinds.has(kind)) continue;
    kinds.add(kind);
    // What it borrows lives as long as the enum: `&'a str`.
    const type = alt.replace(/&(?!')/g, "&'a ").replace(/<'_>/g, "<'a>");
    variants.push({ name: variantName(alt), type, ts: tsName(alt) });
  }
  if (variants.length < 2) return null;
  const typedef = !t.union && !t.generic && typedefs.has(t.idlType as string) ? (t.idlType as string) : undefined;
  const name = typedef ? typeName(typedef) : variants.map((v) => v.name).join("Or");
  return { name, variants, borrows: variants.some((v) => v.type.includes("'a")) };
}

/**
 * The Rust type a parameter, a setter or a dictionary's field takes: a
 * union's enum (ADR 0215), or the one type of its that Rust can take.
 */
function paramType(t: IdlType): string | undefined {
  const union = isUnion(t) ? unionOf(t) : null;
  if (!union) return alternatives(t)[0];
  unions.set(union.name, union);
  return `${union.name}${union.borrows ? "<'_>" : ""}`;
}

// ── Generating ──────────────────────────────────────────────────────────

const skipped = new Map<string, number>();
const skip = (why: string) => skipped.set(why, (skipped.get(why) ?? 0) + 1);

/** A binding; `nullable`, its parameters whose `None` is `null` (ADR 0275). */
type Fn = { name: string; jsName: string; params: string[]; result: string; doc: string[]; nullable?: string[] };

// Operations whose typed form, by an event's or a tag's type (ADR 0223), has
// their name: the form that takes any string is `_named`.
const NAMED: Record<string, string> = {
  "EventTarget.addEventListener": "add_event_listener_named",
  "EventTarget.removeEventListener": "remove_event_listener_named",
  "Document.createElement": "create_element_named",
  "Document.createElementNS": "create_element_ns_named",
};

/** The root of `name`'s inheritance chain within INTERFACES. */
function root(name: string): string {
  const parent = interfaces.get(name)!.parent;
  return parent && known.has(parent) ? root(parent) : name;
}

function mdn(iface: string, member?: string) {
  const ns = interfaces.get(iface)?.legacyNamespace;
  const page = ns ? `JavaScript/Reference/Global_Objects/${ns}/${iface}` : NAMESPACES.includes(iface) ? `JavaScript/Reference/Global_Objects/${iface}` : `API/${iface}`;
  return `https://developer.mozilla.org/docs/Web/${page}${member ? `/${member}` : ""}`;
}

/** Each interface's constants, by name: their Rust. */
const constantsOf = new Map<string, Map<string, string>>();

function functionsOf(i: Interface): Fn[] {
  const fns: Fn[] = [];
  // A namespace's functions are called on it: `WebAssembly.compile(bytes)`.
  const self = i.isNamespace ? [] : [`this: &${typeName(i.name)}`];
  const member = (name: string) => (i.isNamespace && !i.isGlobal ? `${i.name}.${name}` : name);
  // A result that may be `null` is an `Option`: `None` in Rust (ADR 0030).
  const nullable = (t: IdlType) => t.nullable || typedefs.get(t.idlType as string)?.nullable;
  const orNull = (rust: string, t: IdlType) => (nullable(t) ? `Option<${rust}>` : rust);

  // Arguments up to the first optional one, each of the one type it takes:
  // a union's is its enum (ADR 0215).
  const signatures = (args: Arg[]): { names: string[]; types: string[] } | { skip: string } => {
    const names: string[] = [];
    const types: string[] = [];
    for (const a of args) {
      if (a.optional) break;
      const type = paramType(a.idlType);
      if (!type) {
        const why = rustType(a.idlType, "param");
        return { skip: typeof why === "string" ? "?" : why.skip };
      }
      names.push(snake(a.name));
      types.push(type);
    }
    return { names, types };
  };

  // Each optional argument, in order, gives one more form, after the
  // required ones, named after it: `encode_with_input(this, input)`. Later
  // ones add `_and_<name>`. The first unsupported one ends them.
  const optionalForms = (base: string, lead: string[], sig: { names: string[]; types: string[] }, args: Arg[]) => {
    const forms: { name: string; params: string[] }[] = [];
    const params = sig.types.map((type, j) => `${sig.names[j]}: ${type}`);
    const words: string[] = [];
    for (const a of args.filter((a) => a.optional)) {
      const type = paramType(a.idlType);
      if (!type) break;
      words.push(snakeWords(a.name));
      params.push(`${snake(a.name)}: ${type}`);
      // A keyword's `_` isn't needed before more: `continue_with_key`.
      forms.push({ name: `${base.replace(/_$/, "")}_with_${[...lead, ...words].join("_and_")}`, params: [...params] });
    }
    return forms;
  };

  const requiredForm = (base: string, sig: { names: string[]; types: string[] }) => ({
    name: base,
    params: sig.types.map((type, j) => `${sig.names[j]}: ${type}`),
  });

  for (const [index, { member: m }] of i.members.entries()) {
    // A constant is a Rust one, its value written where it's read, as a
    // `const`'s is (ADR 0031): `node::ELEMENT_NODE`.
    if (m.type === "const") {
      const rust = rustType(m.idlType!, "result");
      const value = (m as unknown as { value?: { type: string; value: string } }).value;
      const own = constantsOf.get(i.name) ?? new Map<string, string>();
      if (typeof rust !== "string" || value?.type !== "number") skip("constant");
      else if (!own.has(m.name!)) {
        const literal = rust.startsWith("f") && !/[.xe]/i.test(value.value) ? `${value.value}.0` : value.value;
        // As WebIDL names it, `FLOAT_MAT2x3` too.
        const allow = /[a-z]/.test(m.name!) ? "    #[allow(non_upper_case_globals)]\n" : "";
        own.set(m.name!, `    /// \`${jsName(i)}.${m.name}\`\n${allow}    pub const ${m.name}: ${rust} = ${literal};`);
      }
      constantsOf.set(i.name, own);
      continue;
    }
    if (m.type === "constructor") {
      // `[HTMLConstructor]` is on an element's constructor now, where it was
      // on its interface: only a custom element's class calls it, and `new`
      // of it in a page throws "Illegal constructor".
      if (!i.constructible || (m.extAttrs ?? []).some((a) => a.name === "HTMLConstructor")) continue;
      const sig = signatures(m.arguments ?? []);
      if ("skip" in sig) {
        skip(sig.skip);
        continue;
      }
      // As ReScript names them: `new`, its optional arguments `new_with_<name>`.
      for (const v of [requiredForm("new", sig), ...optionalForms("new", [], sig, m.arguments ?? [])]) {
        fns.push({ name: v.name, jsName: `new ${jsName(i)}`, params: v.params, result: `&'static ${typeName(i.name)}`, doc: [`[MDN](${mdn(i.name, i.name)})`] });
      }
    } else if (m.type === "attribute") {
      if (m.special === "static" || i.isNamespace) {
        skip(m.special || "namespace attribute");
        continue;
      }
      const doc = [`[MDN](${mdn(i.name, m.name)})`];
      // An event handler property, `onclick`: what it holds, a function or
      // none, and a closure to set it to, given the event its name is on this
      // target, as `add_event_listener`'s is (ADR 0223), or `None`, `null`.
      if ((m.idlType!.idlType as string) === "EventHandler" && m.name!.startsWith("on")) {
        const event = listens.get(i.name)?.get(m.name!.slice(2)) ?? "Event";
        const name = snake(m.name!);
        fns.push({ name, jsName: `get ${m.name}`, params: self, result: "Option<&'static JsObject>", doc });
        fns.push({
          name: `set_${name}`,
          jsName: `set ${m.name}`,
          params: [...self, `value: Option<Box<dyn FnMut(&${typeName(event)})>>`],
          result: "()",
          doc,
          nullable: ["value"],
        });
        continue;
      }
      const result = rustType(m.idlType!, "result");
      // A getter whose type isn't supported (a union, say) is skipped, but
      // its setter can still take the union's enum.
      if (typeof result === "string") {
        fns.push({ name: snake(m.name!), jsName: `get ${m.name}`, params: self, result: orNull(result, m.idlType!), doc });
      } else {
        skip(result.skip);
      }
      const forwards = (m.extAttrs ?? []).some((a) => a.name === "PutForwards" || a.name === "Replaceable");
      const value = paramType(m.idlType!);
      if (!m.readonly && !forwards && value) {
        fns.push({ name: `set_${snakeWords(m.name!)}`, jsName: `set ${m.name}`, params: [...self, `value: ${value}`], result: "()", doc });
      }
    } else if (m.type === "operation") {
      if (!m.name) {
        skip("unnamed");
        continue;
      }
      // A static method is the class's, `URL.createObjectURL(blob)`: unless
      // an instance's method has its name, `Response.json`, which keeps it.
      const isStatic = m.special === "static";
      if (isStatic && i.members.some(({ member: o }) => o.type === "operation" && o.name === m.name && o.special !== "static")) {
        skip("static clash");
        continue;
      }
      const result = rustType(m.idlType!, "result");
      const sig = signatures(m.arguments ?? []);
      if (typeof result !== "string" || "skip" in sig) {
        skip(typeof result !== "string" ? result.skip : (sig as { skip: string }).skip);
        continue;
      }
      const doc = [`[MDN](${mdn(i.name, m.name)})`];
      // A later overload is named, as web-sys does, after the required
      // arguments that set it apart from the first: by name where the first
      // has none there (`set_range_text_with_start_and_end`), by type where
      // the types differ (`instantiate_with_web_assembly_module`).
      const first = i.members.slice(0, index).find(({ member: o }) => o.type === "operation" && o.name === m.name && o.special !== "static");
      const firstRequired = (first?.member.arguments ?? []).filter((a) => !a.optional);
      const required = (m.arguments ?? []).filter((a) => !a.optional);
      const key = (a: Arg) => JSON.stringify(a.idlType.idlType);
      const lead = !first
        ? []
        : required.flatMap((a, j) =>
            !firstRequired[j] ? [snake(a.name)] : key(firstRequired[j]) !== key(a) ? [suffix(sig.types[j])] : [],
          );
      const name = NAMED[`${i.name}.${m.name}`] ?? snake(m.name);
      const base = lead.length > 0 ? `${name.replace(/_$/, "")}_with_${lead.join("_and_")}` : name;
      for (const v of [requiredForm(base, sig), ...optionalForms(name, lead, sig, m.arguments ?? [])]) {
        const jsName = isStatic ? `${i.name}.${m.name}` : member(m.name);
        fns.push({ name: v.name, jsName, params: isStatic ? v.params : [...self, ...v.params], result: orNull(result, m.idlType!), doc });
      }
    }
  }

  // An unchecked cast from the root of the chain: `html_input_element::unchecked_from(e)`.
  if (!i.isNamespace && root(i.name) !== i.name) {
    fns.push({
      name: "unchecked_from",
      jsName: "this",
      params: [`this: &${typeName(root(i.name))}`],
      result: `&'static ${typeName(i.name)}`,
      doc: [`Treats \`this\` as \`${typeName(i.name)}\` without checking that it is one.`],
    });
  }

  // The first of each name wins: later overloads and clashes are skipped.
  const seen = new Set<string>();
  return fns.filter((f) => {
    if (seen.has(f.name)) {
      skip("overload or clash");
      return false;
    }
    seen.add(f.name);
    return true;
  });
}

// ── Events and tags (ADR 0223) ──────────────────────────────────────────

// Every interface WebIDL has, in any spec, by its parent: an event's or an
// element's interface this crate doesn't bind is the nearest one it does.
const parents = new Map(everywhere.filter((d) => d.type === "interface" && !d.partial).map((d) => [d.name, d.inheritance ?? undefined]));
const chain = (name: string): string[] => {
  const parent = interfaces.get(name)?.parent;
  return parent && known.has(parent) ? [parent, ...chain(parent)] : [];
};
const bound = (name: string): string => {
  for (let n: string | undefined = name; n; n = parents.get(n)) if (known.has(n)) return n;
  return "Event";
};
/** An event's or a tag's name as a type: `click` → `Click`, `h1` → `H1`. */
const nameType = (name: string) =>
  name.split(/[^A-Za-z0-9]+/).filter(Boolean).map((w) => w[0].toUpperCase() + w.slice(1)).join("");

// Where each event is fired, or bubbles to, by target and name: its interfaces.
type WebrefEvent = { type: string; interface: string; targets?: { target: string; bubblingPath?: string[] }[] };
const fired = new Map<string, Map<string, Set<string>>>();
for (const e of Object.values(await webrefEvents.listAll()) as WebrefEvent[]) {
  for (const t of e.targets ?? []) {
    for (const at of [t.target, ...(t.bubblingPath ?? [])]) {
      const names = fired.get(at) ?? new Map<string, Set<string>>();
      names.set(e.type, (names.get(e.type) ?? new Set<string>()).add(bound(e.interface)));
      fired.set(at, names);
    }
  }
}
// What each target this crate binds takes: each name its own interface, or
// the nearest ancestor, fires; its event the one interface they say, or
// `Event` where specs disagree.
let disagreeing = 0;
const listens = new Map<string, Map<string, string>>();
for (const name of INTERFACES) {
  if (name !== "EventTarget" && !chain(name).includes("EventTarget")) continue;
  const map = new Map<string, string>();
  for (const level of [name, ...chain(name)]) {
    for (const [event, given] of fired.get(level) ?? []) {
      if (map.has(event)) continue;
      if (given.size > 1) disagreeing++;
      map.set(event, given.size === 1 ? [...given][0] : "Event");
    }
  }
  if (map.size > 0) listens.set(name, map);
}
const eventNames = [...new Set([...listens.values()].flatMap((m) => [...m.keys()]))].sort();

// Each HTML element by its tag: the interface it is, or the nearest one
// bound. Not SVG's nor MathML's, nor an obsolete one.
type WebrefElements = { elements: { name: string; interface?: string; obsolete?: boolean }[] };
const tagElements = new Map<string, string>();
for (const spec of Object.values(await webrefElements.listAll()) as WebrefElements[]) {
  for (const el of spec.elements) {
    if (!el.interface || el.obsolete || tagElements.has(el.name)) continue;
    const lineage: string[] = [];
    for (let n: string | undefined = el.interface; n; n = parents.get(n)) lineage.push(n);
    if (lineage.includes("HTMLElement")) tagElements.set(el.name, bound(el.interface));
  }
}
const tagNames = [...tagElements.keys()].sort();
// And each SVG element by its tag, as TypeScript's `SVGElementTagNameMap`:
// made by `createElementNS` of SVG's namespace, where `createElement`
// would make an `HTMLUnknownElement`, so not `Tag`'s.
const svgTagElements = new Map<string, string>();
for (const spec of Object.values(await webrefElements.listAll()) as WebrefElements[]) {
  for (const el of spec.elements) {
    if (!el.interface || el.obsolete || svgTagElements.has(el.name)) continue;
    const lineage: string[] = [];
    for (let n: string | undefined = el.interface; n; n = parents.get(n)) lineage.push(n);
    if (lineage.includes("SVGElement") && known.has(el.interface)) svgTagElements.set(el.name, el.interface);
  }
}
const svgTagNames = [...svgTagElements.keys()].sort();
for (const names of [eventNames, tagNames, svgTagNames]) {
  const types = names.map(nameType);
  const clash = types.find((t, k) => types.indexOf(t) !== k);
  if (clash) throw new Error(`two names are ${clash}`);
}

// The typed forms of the `_named` operations: generic, so Rust functions
// whose JS is the operation's, not extern ones.
const typed = (doc: string[], js: string, signature: string) =>
  [...doc.map((d) => `    /// ${d}`), `    #[cfg_attr(rust_js, rust_js::link_name = ${JSON.stringify(js)})]`, "    // rust-js writes its JS: the body never runs, nor reads a parameter.", "    #[allow(unused_variables)]", `    pub fn ${signature} {`, `        unreachable!()`, `    }`].join("\n");
const listener = "Box<dyn FnMut(&<T as Listen<E>>::Event)>";
const TYPED: Record<string, string[]> = {
  EventTarget: [
    typed([`[MDN](${mdn("EventTarget", "addEventListener")}): \`listener\` for each event of a name here,`, "given the event the name is on this target (ADR 0223): a button's `Click` is a", "`PointerEvent`. One the data doesn't know is `add_event_listener_named`'s."], "addEventListener", `add_event_listener<T: Listen<E>, E>(this: &T, event: E, listener: ${listener})`),
    typed([`[MDN](${mdn("EventTarget", "addEventListener")})`], "addEventListener", `add_event_listener_with_options<T: Listen<E>, E>(this: &T, event: E, listener: ${listener}, options: AddEventListenerOptionsOrBool<'_>)`),
    typed([`[MDN](${mdn("EventTarget", "removeEventListener")})`], "removeEventListener", `remove_event_listener<T: Listen<E>, E>(this: &T, event: E, listener: ${listener})`),
    typed([`[MDN](${mdn("EventTarget", "removeEventListener")})`], "removeEventListener", `remove_event_listener_with_options<T: Listen<E>, E>(this: &T, event: E, listener: ${listener}, options: EventListenerOptionsOrBool)`),
  ],
  Document: [
    typed([`[MDN](${mdn("Document", "createElement")}): the element a tag is (ADR 0223),`, "`create_element(document, Button)` an `HTMLButtonElement`. Another name is", "`create_element_named`'s, an `Element`."], "createElement", `create_element<T: Tag>(this: &Document, tag: T) -> &'static <T as Tag>::Element`),
    typed([`[MDN](${mdn("Document", "createElementNS")}): the SVG element a tag is,`, "`create_element_ns(document, namespaces::Svg, svg_tags::Circle)` an `SVGCircleElement`.", "Another is `create_element_ns_named`'s, an `Element`."], "createElementNS", `create_element_ns<T: SVGTag>(this: &Document, namespace: namespaces::Svg, tag: T) -> &'static <T as SVGTag>::Element`),
  ],
};

const out: string[] = [];
const line = (s = "") => out.push(s);

line(`//! The web platform for rust-js: DOM bindings generated by \`webapi/generate.ts\``);
line(`//! from W3C's WebIDL (\`@webref/idl\` ${webref.version}; specs: ${SPECS.join(", ")}). Do not edit.`);
line(`//!`);
line(`//! Each interface is a type (\`Element\`) and a module of its members`);
line(`//! (\`element::append\`). Inheritance is \`Deref\`, so an \`&HTMLButtonElement\``);
line(`//! goes wherever an \`&Element\` or \`&Node\` is expected. See ADR 0024.`);
line(`//! The JS language's own types, \`Promise\` and \`ArrayBuffer\` say, are the js crate's (ADR 0102).`);
line(`//! Each event's name and each tag is a type too (ADR 0223), from \`@webref/events\` ${eventsPackage.version}`);
line(`//! and \`@webref/elements\` ${elementsPackage.version}: \`events::Click\`, whose value is \`"click"\`.`);
line();
line(`// Many Rust functions call the same JS name: a form per optional argument
// (\`new\`, \`new_with_body\`), and methods of the same name on different
// interfaces. rustc warns because in native code they would be one symbol.`);
line(`#![allow(clashing_extern_declarations)]`);
line(`// And some are named as libc's functions are, \`open\`, \`close\` and \`write\`:
// JS methods, which in native code rustc would take for those.`);
line(`#![allow(invalid_runtime_symbol_definitions)]`);
line();
line(`use core::marker::PhantomData;`);
line(`use core::ops::Deref;`);
line(`use js::{ArrayBuffer, Defined, JsObject, Promise, StructuredClone, Uint8Array, Unknown};`);
line();
line(`unsafe extern "Rust" {`);
GLOBALS.forEach(([name, type], k) => {
  if (k > 0) line();
  line(`    /// The \`${name}\` global.`);
  line(`    pub safe static ${name}: &'static ${type};`);
});
line(`}`);

let count = 0;

/** A parameter's union, `nodes: NodeOrStr<'_>`'s, if it's one. */
const unionParam = (p: string): Union | undefined => unions.get(p.slice(p.indexOf(": ") + 2).replace(/<'_>$/, ""));

/**
 * A function with a parameter typed `any`, or of a union, as a generic Rust
 * one, which an extern one can't be: each `any` of a type parameter of its
 * own, named after it, `message: M` (ADR 0225), and each union `impl` its
 * trait, `nodes: impl IntoNodeOrStr`, which passes a member on as it is
 * (ADR 0229).
 */
function generic(f: Fn): string {
  const names: string[] = [];
  const params = f.params.map((p) => {
    const union = unionParam(p);
    if (union) return `${p.slice(0, p.indexOf(": "))}: impl Into${union.name}`;
    if (!p.endsWith(`: ${ANY}`)) return p;
    const param = p.slice(0, -`: ${ANY}`.length);
    let type = param[0].toUpperCase();
    while (names.some((n) => n.split(":")[0] === type)) type += "1";
    names.push(CLONED.has(`${f.jsName}.${param}`) ? `${type}: StructuredClone` : type);
    return `${param}: ${type}`;
  });
  const result = f.result === "()" ? "" : ` -> ${f.result}`;
  return [
    ...f.doc.map((d) => `    /// ${d}`),
    `    #[cfg_attr(rust_js, rust_js::link_name = ${JSON.stringify(f.jsName)})]`,
    "    // rust-js writes its JS: the body never runs, nor reads a parameter.",
    "    #[allow(unused_variables)]",
    `    pub fn ${f.name}${names.length > 0 ? `<${names.join(", ")}>` : ""}(${params.join(", ")})${result} {`,
    "        unreachable!()",
    "    }",
  ].join("\n");
}

/** `pub mod <name> { .. }`, holding a type's or a namespace's functions. */
function module(name: string, all: Fn[], typed: string[] = [], constants: string[] = []) {
  if (all.length === 0 && constants.length === 0) return;
  count += all.length;
  const fns = all.filter((f) => !f.params.some((p) => p.endsWith(`: ${ANY}`) || unionParam(p)));
  typed = [...all.filter((f) => !fns.includes(f)).map(generic), ...typed];
  line();
  line(`pub mod ${name} {`);
  if (all.length > 0 || typed.length > 0) line(`    use super::*;`);
  for (const c of constants) {
    line();
    line(c);
  }
  if (fns.length > 0) {
  line();
  line(`    unsafe extern "Rust" {`);
  fns.forEach((f, k) => {
    if (k > 0) line();
    for (const d of f.doc) line(`        /// ${d}`);
    if (f.jsName !== f.name) line(`        #[link_name = ${JSON.stringify(f.jsName)}]`);
    if (f.nullable) line(`        #[cfg_attr(rust_js, rust_js::nullable(${f.nullable.join(", ")}))]`);
    const result = f.result === "()" ? "" : ` -> ${f.result}`;
    line(`        pub safe fn ${f.name}(${f.params.join(", ")})${result};`);
  });
  line(`    }`);
  }
  for (const t of typed) {
    line();
    line(t);
  }
  line(`}`);
}

for (const name of INTERFACES) {
  const i = interfaces.get(name)!;
  const type = typeName(name);
  line();
  line(`/// [\`${jsName(i)}\`](${mdn(name)})`);
  // Its JS class, as `instanceof` names it, where its Rust name isn't (ADR 0214).
  if (jsName(i) !== type) line(`#[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(jsName(i))})]`);
  // What it is to TypeScript: its DOM lib's interface of the same name.
  line(`#[cfg_attr(rust_js, rust_js::types = ${JSON.stringify(jsName(i))})]`);
  line(`pub struct ${type}(PhantomData<JsObject>);`);
  if (i.parent && known.has(i.parent)) {
    const parent = typeName(i.parent);
    line();
    line(`impl Deref for ${type} {`);
    line(`    type Target = ${parent};`);
    line();
    line(`    fn deref(&self) -> &${parent} {`);
    line(`        // Never runs: rust-js compiles this \`Deref\` to the object itself.`);
    line(`        unsafe { &*(self as *const Self as *const ${parent}) }`);
    line(`    }`);
    line(`}`);
  }
  const fns = [...functionsOf(i), ...(EXTRA[name] ?? [])];
  module(snake(qualified(name)), fns, TYPED[name] ?? [], [...(constantsOf.get(name)?.values() ?? [])]);
}

for (const name of NAMESPACES) {
  line();
  line(`/// The [\`${name}\`](${mdn(name)}) namespace.`);
  module(snake(name), functionsOf(namespaces.get(name)!));
}

line();
line("/// What every JS global scope has, a window's, a worker's or Node's, called bare: `fetch(url)`.");
module("global", functionsOf(globalScope));

// The dictionaries results use, as plain structs: JS objects (ADR 0020).
for (const [name, fields] of usedDictionaries) {
  line();
  line(`/// The \`${name}\` dictionary: a JS object with these fields, a \`None\` one not there.`);
  if (fields.every((f) => f.optional)) line(`#[derive(Default)]`);
  line(`pub struct ${typeName(name)} {`);
  for (const f of fields) {
    if (f.rust !== f.js) line(`    #[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(f.js)})]`);
    line(`    pub ${f.rust}: ${f.optional ? `Option<${f.type}>` : f.type},`);
  }
  line(`}`);
}

// The dictionaries functions take (ADR 0102): records of optional fields.
for (const [name, { fields, borrows }] of [...paramDictionaries].sort(([a], [b]) => a.localeCompare(b))) {
  line();
  line(`/// The [\`${name}\`](https://developer.mozilla.org/docs/Web/API/${name}) dictionary: a JS object of these fields, a \`None\` one not given.`);
  if (fields.every((f) => f.optional)) line(`#[derive(Default)]`);
  line(`pub struct ${typeName(name)}${borrows ? "<'a>" : ""} {`);
  for (const f of fields) {
    if (f.rust !== f.js) line(`    #[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(f.js)})]`);
    line(`    pub ${f.rust}: ${f.optional ? `Option<${f.type}>` : f.type},`);
  }
  line(`}`);
}

// The unions functions take (ADR 0215): untagged enums, each value the
// member itself, as TypeScript's union is (ADR 0214). Each member converts
// into it, and so does each interface that extends a class member but no
// other: an `&HTMLElement` into `NodeOrStr`'s `Node`.
const ancestors = (name: string): string[] => {
  const parent = interfaces.get(name)?.parent;
  return parent && known.has(parent) ? [parent, ...ancestors(parent)] : [];
};
// And each union's trait, `IntoNodeOrStr`, of what converts into it and
// the enum, which a function's parameter of it takes (ADR 0229): each the
// value itself. Sealed, by a trait each of them has once.
const sealed = new Set<string>();
for (const union of [...unions.values()].sort((a, b) => a.name.localeCompare(b.name))) {
  const lifetime = union.borrows ? "<'a>" : "";
  const elided = (type: string) => type.replace(/&'a /g, "&").replace(/'a\b/g, "'_");
  const members: string[] = [];
  line();
  line(`/// \`${union.variants.map((v) => v.ts).join(" | ")}\`: each variant's value is the member itself (ADR 0215).`);
  line(`#[cfg_attr(rust_js, rust_js::untagged)]`);
  line(`pub enum ${union.name}${lifetime} {`);
  for (const v of union.variants) line(`    ${v.name}(${v.type}),`);
  line(`}`);
  const classes = new Set(union.variants.map((v) => INTERFACES.find((n) => typeName(n) === v.name)).filter((n) => n !== undefined));
  for (const v of union.variants) {
    const own = INTERFACES.find((n) => typeName(n) === v.name);
    const extending = own ? INTERFACES.filter((n) => n !== own && ancestors(n).find((a) => classes.has(a)) === own) : [];
    for (const from of [v.type, ...extending.map((n) => `&'a ${typeName(n)}`)]) {
      members.push(elided(from));
      line();
      line(`impl${lifetime} From<${from}> for ${union.name}${lifetime} {`);
      line(`    fn from(value: ${from}) -> Self {`);
      line(`        ${union.name}::${v.name}(value)`);
      line(`    }`);
      line(`}`);
    }
  }
  const ts = union.variants.map((v) => v.ts).join(" | ");
  const own = `${union.name}${union.borrows ? "<'_>" : ""}`;
  line();
  line(`/// What a \`${ts}\` parameter takes: each member as it is, and the enum (ADR 0229).`);
  line(`#[diagnostic::on_unimplemented(message = "\`{Self}\` is not a \`${ts}\`")]`);
  line(`#[cfg_attr(rust_js, rust_js::types = "${ts}")]`);
  line(`pub trait Into${union.name}: sealed::Sealed {}`);
  for (const member of [...members, own]) {
    line(`impl Into${union.name} for ${member} {}`);
    sealed.add(member);
  }
  line();
  line(`impl${lifetime} ${union.name}${lifetime} {`);
  line(`    /// The member a parameter was given, as its enum, to \`match\`: the value itself.`);
  line(`    #[cfg_attr(rust_js, rust_js::link_name = "this")]`);
  line(`    #[allow(unused_variables)]`);
  line(`    pub fn of(this: impl Into${union.name}${union.borrows ? " + 'a" : ""}) -> ${union.name}${lifetime} {`);
  line(`        unreachable!()`);
  line(`    }`);
  line(`}`);
}
line();
line(`mod sealed {`);
line(`    use super::*;`);
line();
line(`    pub trait Sealed {}`);
for (const member of [...sealed].sort()) line(`    impl Sealed for ${member} {}`);
line(`}`);


// Each event's name and each tag, as a type whose value is its string, and
// what each is (ADR 0223); and each interface's ancestors.
line();
line(`/// What an event of a name is on a target, from \`@webref/events\` (ADR 0223):`);
line(`/// a button's \`Click\` is a \`PointerEvent\`, a document's \`Keydown\` a \`KeyboardEvent\`.`);
line(`pub trait Listen<E> {`);
line(`    type Event;`);
line(`}`);
line();
line(`/// The element a tag makes, from \`@webref/elements\` (ADR 0223): \`Button\`'s is an \`HTMLButtonElement\`.`);
line(`pub trait Tag {`);
line(`    type Element;`);
line(`}`);
line();
line(`/// The SVG element a tag makes, as TypeScript's \`SVGElementTagNameMap\`: \`Circle\`'s is an \`SVGCircleElement\`.`);
line(`pub trait SVGTag {`);
line(`    type Element;`);
line(`}`);
line();
line(`/// The namespaces \`create_element_ns\` makes an element of a tag in, each a type whose value is its URI.`);
line(`pub mod namespaces {`);
line(`    /// SVG's, \`"http://www.w3.org/2000/svg"\`.`);
line(`    #[cfg_attr(rust_js, rust_js::name = "http://www.w3.org/2000/svg")]`);
line(`    pub struct Svg;`);
line(`}`);
line();
line(`/// \`Self\` is a \`T\`, or extends one, as WebIDL says: an \`HTMLButtonElement\` is an \`Element\` (ADR 0223).`);
line(`/// Unsafe to implement: something that takes a \`T\` is given a \`Self\` unchecked.`);
line(`pub unsafe trait IsA<T> {}`);
line();
line(`/// Each event's name, a type whose value is its string: \`Click\` is \`"click"\` (ADR 0223).`);
line(`pub mod events {`);
eventNames.forEach((name, k) => {
  if (k > 0) line();
  line(`    /// \`"${name}"\``);
  line(`    #[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(name)})]`);
  line(`    pub struct ${nameType(name)};`);
});
line(`}`);
line();
line(`/// Each HTML element's tag, a type whose value is its name: \`Button\` is \`"button"\` (ADR 0223).`);
line(`pub mod tags {`);
tagNames.forEach((name, k) => {
  if (k > 0) line();
  line(`    /// \`<${name}>\``);
  line(`    #[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(name)})]`);
  line(`    pub struct ${nameType(name)};`);
});
line(`}`);
line();
line(`/// Each SVG element's tag, a type whose value is its name: \`Circle\` is \`"circle"\`.`);
line(`pub mod svg_tags {`);
svgTagNames.forEach((name, k) => {
  if (k > 0) line();
  line(`    /// \`<${name}>\``);
  line(`    #[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(name)})]`);
  line(`    pub struct ${nameType(name)};`);
});
line(`}`);
let impls = 0;
for (const [target, map] of listens) {
  line();
  for (const event of [...map.keys()].sort()) {
    line(`impl Listen<events::${nameType(event)}> for ${typeName(target)} { type Event = ${typeName(map.get(event)!)}; }`);
    impls++;
  }
}
line();
for (const tag of tagNames) line(`impl Tag for tags::${nameType(tag)} { type Element = ${typeName(tagElements.get(tag)!)}; }`);
line();
for (const tag of svgTagNames) line(`impl SVGTag for svg_tags::${nameType(tag)} { type Element = ${typeName(svgTagElements.get(tag)!)}; }`);
line();
for (const name of INTERFACES) {
  for (const a of [name, ...chain(name)]) line(`unsafe impl IsA<${typeName(a)}> for ${typeName(name)} {}`);
}

// What WebIDL marks `[Serializable]`, a `Blob` say, the browser's structured
// clone copies (ADR 0225); and an object is never nullish.
line();
for (const name of INTERFACES) {
  const serializable = read.some((d) => d.type === "interface" && d.name === name && (d.extAttrs ?? []).some((a) => a.name === "Serializable"));
  if (serializable) line(`unsafe impl StructuredClone for ${typeName(name)} {}`);
  line(`unsafe impl Defined for ${typeName(name)} {}`);
}

await Bun.write(new URL("./src/lib.rs", import.meta.url), `${out.join("\n")}\n`);
const reasons = [...skipped].sort((a, b) => b[1] - a[1]).map(([why, n]) => `${why} ${n}`);
console.log(`src/lib.rs: ${INTERFACES.length} interfaces, ${NAMESPACES.length} namespaces, ${count} functions, ${unions.size} unions`);
console.log(`events: ${eventNames.length} names, ${impls} on ${listens.size} targets (${disagreeing} where specs disagree, as \`Event\`); tags: ${tagNames.length}`);
console.log(`skipped: ${reasons.slice(0, 12).join(", ")}${reasons.length > 12 ? ", ..." : ""}`);
