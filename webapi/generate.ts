// Generate src/lib.rs, the `webapi` crate, from W3C's WebIDL (ADR 0024).
//
//   bun webapi/generate.ts [out]   writes src/lib.rs, or `out`
//
// Every rule about what gets in is in this file: which interfaces
// (INTERFACES), how WebIDL types become Rust (`rustType`), and the names.
// A member is generated only if rust-js supports all of its types; the rest
// are counted and skipped, and rerunning after rust-js grows picks them up.

import { readFileSync } from "node:fs";

import idl from "@webref/idl";
import { typescript, typescriptInterfaces } from "./coverage";
import { identifier, needsJsName, plain, snake, snakeWords } from "../scripts/rust-names";

/** A record of each spec's, by its name, in order: what @webref lists in
 * the order its directory reads, which differs by filesystem, so the crate
 * would be another on another machine. */
const bySpec = <T>(record: Record<string, T>): Record<string, T> =>
  Object.fromEntries(Object.keys(record).sort().map((spec) => [spec, record[spec]]));
import webref from "@webref/idl/package.json" with { type: "json" };
import webrefEvents from "@webref/events";
import eventsPackage from "@webref/events/package.json" with { type: "json" };
import webrefElements from "@webref/elements";
import elementsPackage from "@webref/elements/package.json" with { type: "json" };

// The specs to read. Partial interfaces and mixins from these are merged in.
const SPECS = ["dom", "html", "hr-time", "uievents", "pointerevents", "cssom", "cssom-view", "geometry", "fetch", "encoding", "wasm-js-api", "wasm-web-api", "xhr", "streams", "touch-events", "FileAPI", "clipboard-apis", "css-animations", "css-transitions", "SVG", "svg-paths", "svg-animations", "filter-effects", "css-masking", "intersection-observer", "url", "console"];

// The everyday DOM. Members that use any other interface are skipped.
const INTERFACES = [
  // dom
  "EventTarget", "Event", "Node", "CharacterData", "Text", "Comment", "Element", "Document",
  "DocumentFragment", "DOMTokenList", "NodeList", "HTMLCollection", "AbortController", "AbortSignal",
  // A mixin as the union of what includes it, `Document | ShadowRoot` (`UNIONS`).
  "DocumentOrShadowRoot",
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
  // fetch: `window.fetch(..)`, and what it gives back
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
const defined = new Set(Object.values(bySpec(await idl.parseAll() as Record<string, { type: string; name: string; partial?: boolean }[]>))
  .flat().filter((d) => d.type === "interface" && !d.partial).map((d) => d.name));
INTERFACES.push(...[...typescriptDom.keys()].filter((name) => defined.has(name) && !INTERFACES.includes(name)).sort());
const known = new Set(INTERFACES);

// Members written by hand, in place of what's generated of their name. A
// message's sender is a union JS can't tell apart by `instanceof`: a
// frame's window is another realm's, no `Window` of this one's.
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
  // V8's `console.createTask(name)`, which no WebIDL nor TypeScript lib
  // declares, and Firefox, Safari and jsdom lack: a property that may be
  // `undefined`, as an optional property is, read as React reads it,
  // `console.createTask ? .. : ..`, and typed as its Flow types declare it
  // (ADR 0811).
  console: [
    {
      name: "create_task",
      jsName: "get console.createTask",
      params: [],
      result: "Option<&'static dyn Fn(&str) -> &'static ConsoleTask>",
      none: "undefined",
      doc: [
        "[Chrome](https://developer.chrome.com/docs/devtools/console/api#createtask): `console.createTask`,",
        "the function that makes a [`ConsoleTask`] of a name, whose `run` DevTools shows the stack of",
        "where it was made; `None`, `undefined`, where the host hasn't it, as Firefox, Safari and jsdom",
        "haven't. No TypeScript lib declares it: it's typed as React's Flow declares it,",
        "`createTask(label: string): ConsoleTask` (ADR 0811).",
      ],
    },
    // Chrome's `console.timeStamp(label, start, end, track, trackGroup,
    // color)`, its DevTools' extensibility API, which no WebIDL declares
    // and React's performance track logs each phase with, asked first,
    // `typeof console.timeStamp === 'function'`, as `js::is_function` asks
    // (ADR 0963). Firefox and Safari take its label alone.
    {
      name: "time_stamp",
      jsName: "console.timeStamp",
      params: [
        "label: &str",
        "start: Option<StrOrNumber<'_>>",
        "end: Option<StrOrNumber<'_>>",
        "track_name: Option<&str>",
        "track_group: Option<&str>",
        "color: Option<&str>",
      ],
      result: "()",
      doc: [
        "[Chrome](https://developer.chrome.com/docs/devtools/performance/extension#inject_your_data_with_consoletimestamp):",
        "`console.timeStamp(label, start, end, trackName, trackGroup, color)`, an entry DevTools'",
        "performance panel shows on track `track_name` of `track_group`, from `start` to `end`, each a",
        "time, `performance.now()`'s, or a label an earlier one was given; `color` one of DevTools'",
        "names, `\"primary\"`, `\"secondary\"`, `\"tertiary\"` and the rest. No WebIDL declares it;",
        "where the host hasn't it, `js::is_function(console::time_stamp)` is `false` (ADR 0963).",
      ],
    },
  ],
};

// `scheduler.yield(options)`, as Chrome first shipped it and React's
// SchedulerPostTask calls it, `scheduler.yield({signal})`: the spec's
// takes none now and inherits the task's, and a host ignores what it
// isn't given a parameter for (ADR 0963, amended).
EXTRA.Scheduler = [
  {
    name: "r#yield",
    jsName: "yield",
    params: ["this: &Scheduler", "options: Option<SchedulerPostTaskOptions<'_>>"],
    result: "Promise<()>",
    doc: [
      "[MDN](https://developer.mozilla.org/docs/Web/API/Scheduler/yield): a promise that settles when",
      "the task may go on, given its `options` as Chrome first took them and React's scheduler gives",
      "them, `scheduler.yield({ signal })`; the spec's takes none and inherits the task's, and a host",
      "ignores what it doesn't take (ADR 0963).",
    ],
  },
];

// Types written by hand, of what no WebIDL nor TypeScript lib declares,
// beside the namespace or interface whose member gives one: what
// `console.createTask` makes, as React's Flow declares it (ADR 0811).
const TYPES_BY_HAND: Record<string, string[]> = {
  console: [
    [
      "/// [`ConsoleTask`](https://developer.chrome.com/docs/devtools/console/api#createtask): what",
      "/// [`console::create_task`]'s function makes, a task whose `run` DevTools shows the stack of",
      "/// where it was made. No TypeScript lib declares it: it's typed as React's Flow declares it,",
      "/// `interface ConsoleTask { run<T>(f: () => T): T }` (ADR 0811).",
      '#[cfg_attr(rust_js, rust_js::types = "{ run<T>(f: () => T): T }")]',
      "pub struct ConsoleTask(PhantomData<JsObject>);",
      "",
      "impl ConsoleTask {",
      "    /// `task.run(f)`: `f` called, as part of the task, and what it gives.",
      '    #[cfg_attr(rust_js, rust_js::link_name = "run")]',
      "    // rust-js writes its JS: the body never runs, nor reads a parameter.",
      "    #[allow(unused_variables)]",
      "    pub fn run<T>(&self, f: impl FnOnce() -> T) -> T {",
      "        unreachable!()",
      "    }",
      "}",
    ].join("\n"),
  ],
};

// Namespaces: a module of functions, like `web_assembly::compile`.
const NAMESPACES = ["WebAssembly", "console", "CSS"];

// JS's own types that WebIDL uses, the js crate's: its buffers and typed
// arrays, each element a Rust number of its kind (ADR 0283).
const BUILTINS = new Set([
  "ArrayBuffer", "SharedArrayBuffer", "DataView", "Int8Array", "Uint8Array", "Uint8ClampedArray", "Int16Array", "Uint16Array",
  "Int32Array", "Uint32Array", "Float32Array", "Float64Array", "BigInt64Array", "BigUint64Array",
]);

// What throws for what a program can't prevent or check first, the page's
// permissions or its quota, as the spec says it does: a `Result`, its `Err`
// what it threw, as Rust's I/O is (ADRs 0035, 0442). One that throws only of
// a misuse a program can avoid, an invalid selector, panics, as Rust's
// indexing does. WebIDL doesn't say which throw: this is the list, by an
// interface and its member's JS form, and why.
const THROWS: Record<string, string> = {
  "Window get localStorage": "a `SecurityError` where the page may not keep data: storage turned off, a sandboxed frame, a `file:` or `data:` page",
  "Window get sessionStorage": "a `SecurityError` where the page may not keep data: storage turned off, a sandboxed frame, a `file:` or `data:` page",
  "Storage setItem": "a `QuotaExceededError` where the storage is full",
  "Document get cookie": "a `SecurityError` in a sandboxed document, or one of an opaque origin",
  "Document set cookie": "a `SecurityError` in a sandboxed document, or one of an opaque origin",
  "History pushState": "a `SecurityError` in a document no longer shown, or where the browser limits how often it's called",
  "History replaceState": "a `SecurityError` in a document no longer shown, or where the browser limits how often it's called",
  "HTMLCanvasElement toDataURL": "a `SecurityError` where an image of another origin, not shared by CORS, was drawn on it",
  "HTMLCanvasElement toBlob": "a `SecurityError` where an image of another origin, not shared by CORS, was drawn on it",
  "CanvasRenderingContext2D getImageData": "a `SecurityError` where an image of another origin, not shared by CORS, was drawn on it",
  "OffscreenCanvasRenderingContext2D getImageData": "a `SecurityError` where an image of another origin, not shared by CORS, was drawn on it",
  "CSSStyleSheet get cssRules": "a `SecurityError` of a stylesheet of another origin, not shared by CORS",
  "CSSStyleSheet get rules": "a `SecurityError` of a stylesheet of another origin, not shared by CORS",
  "CSSStyleSheet insertRule": "a `SecurityError` of a stylesheet of another origin, not shared by CORS",
  "CSSStyleSheet deleteRule": "a `SecurityError` of a stylesheet of another origin, not shared by CORS",
  "CSSStyleSheet addRule": "a `SecurityError` of a stylesheet of another origin, not shared by CORS",
  "CSSStyleSheet removeRule": "a `SecurityError` of a stylesheet of another origin, not shared by CORS",
  "IDBFactory open": "a `SecurityError` where the page may not keep data: storage turned off, a sandboxed frame, a `file:` or `data:` page",
  "IDBFactory deleteDatabase": "a `SecurityError` where the page may not keep data: storage turned off, a sandboxed frame, a `file:` or `data:` page",
};
const thrown = new Set<string>();

/** `f`, a member of `i`, that throws as `THROWS` says: its result a `Result`. */
function throwing(i: Interface, f: Fn): Fn {
  const key = `${i.name} ${f.jsName}`;
  const why = THROWS[key];
  if (why === undefined) return f;
  thrown.add(key);
  return { ...f, result: `Result<${f.result}, &'static JsError>`, doc: [...f.doc, `\`Err\`: ${why}.`] };
}

// A dictionary field's type, where WebIDL's is one Rust can't take: a
// `HeadersInit` is a sequence or a record, and `fetch` takes a `Headers` too.
// `PromiseRejectionEventInit`'s `promise` is WebIDL's `object`, TypeScript's
// `Promise<any>`: a promise of any value, `dyn IntoUnknown` (ADR 0654).
const FIELD_TYPES: Record<string, string> = { "RequestInit.headers": "&'a Headers", "PromiseRejectionEventInit.promise": "&'a dyn IntoUnknown" };

// The globals at the crate root: `document`, `window`, `performance`, which
// a worker has too, `navigator`, and `history`, the page's; and `console`,
// a namespace's value (ADR 0667).
const GLOBALS: [string, string][] = [
  ["document", "Document"], ["window", "Window"], ["performance", "Performance"], ["navigator", "Navigator"], ["history", "History"],
  ["console", "Console"],
];

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

const all = bySpec(await idl.parseAll() as Record<string, Def[]>);
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
        : m.type === "iterable" || m.type === "maplike" || m.type === "setlike"
          ? !!ts?.members.has("forEach")
          : m.special === "stringifier" && !m.name
          ? !!ts?.members.has("toString")
          : !!m.name && (d.type === "interface mixin" ? anyClassHas.has(m.name) : !!ts && (m.special === "static" ? ts.statics : ts.members).has(m.name));
    for (const m of d.members ?? []) if (!kept(m)) leftOut.add(m);
  }
}
const everywhere: Def[] = Object.values(all).flat();

// Names that are strings (enums) or other types (typedefs), from any spec.
const enums = new Set(everywhere.filter((d) => d.type === "enum").map((d) => d.name));
const typedefs = new Map(everywhere.filter((d) => d.type === "typedef").map((d) => [d.name, d.idlType!]));
const dictionaries = new Map(everywhere.filter((d) => d.type === "dictionary" && !d.partial).map((d) => [d.name, d]));
// A dictionary's partial members, another spec's, `EffectTiming`'s `delay`
// and `duration` of web-animations-2, KeyframeAnimationOptions' `rangeStart`
// of scroll-animations: the specs above have them all, any other those
// TypeScript's DOM declares of its interface (ADR 0281, rd160).
const typescriptDictionaries = await typescriptInterfaces();
const partialMembers = new Map<string, Member[]>();
for (const [spec, defs] of Object.entries(all)) {
  for (const d of defs) {
    if (d.type !== "dictionary" || !d.partial) continue;
    const ts = typescriptDictionaries.get(d.name);
    const kept = (d.members ?? []).filter((m) => SPECS.includes(spec) || (!!m.name && !!ts?.has(m.name)));
    partialMembers.set(d.name, [...(partialMembers.get(d.name) ?? []), ...kept]);
  }
}
const callbacks = new Map(everywhere.filter((d) => d.type === "callback").map((d) => [d.name, d]));
const callbackInterfaces = new Set(everywhere.filter((d) => d.type === "callback interface").map((d) => d.name));
// The callbacks that are a class, which `new` calls, as TypeScript's
// construct signature has it, `new (...params: any[]): HTMLElement`: WebIDL
// writes one as a function.
const constructors = new Set(["CustomElementConstructor"]);
// The names a class is known by too, `[LegacyWindowAlias=SVGPoint] interface
// DOMPoint`, which SVG's IDL still uses: `create_svg_point()` gives a `DOMPoint`.
const windowAliases = new Map(
  everywhere.flatMap((d) =>
    (d.extAttrs ?? []).filter((a) => a.name === "LegacyWindowAlias").flatMap((a: any) => {
      const value = a.rhs?.value;
      const names: string[] = typeof value === "string" ? [value] : (value ?? []).map((v: any) => v.value);
      return names.map((alias): [string, string] => [alias, d.name]);
    }),
  ),
);

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
  /** Of a mixin's union (`UNIONS`), TypeScript's type, `Document |
   * ShadowRoot`, and the interfaces that are one. */
  ts?: string;
  includers?: string[];
  /** `[Global]`: a global object, a `Window`, whose `onerror` JS calls with
   * an error's message, where it was and what was thrown (rd192). */
  globalObject?: boolean;
  /** `[Exposed=(Window,Worker)]`: the global scopes whose global object
   * has its class, `window.ErrorEvent` (ADR 0891); `*` every one. */
  exposed?: string[];
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
      i.globalObject = (d.extAttrs ?? []).some((a) => a.name === "Global");
      // `[Exposed=*]` is of the type `*`, of no value.
      const exposed: any = (d.extAttrs ?? []).find((a) => a.name === "Exposed")?.rhs;
      i.exposed =
        exposed?.type === "*" ? ["*"]
        : typeof exposed?.value === "string" ? [exposed.value]
        : (exposed?.value ?? []).map((v: { value: string }) => v.value);
    }
    i.members.push(...(d.members ?? []).filter((m) => !leftOut.has(m)).map((member) => ({ member, from: d.name })));
    interfaces.set(d.name, i);
  } else if (d.type === "namespace" && NAMESPACES.includes(d.name)) {
    const n = namespaces.get(d.name) ?? { name: d.name, members: [], constructible: false, isNamespace: true };
    // Of another spec's partial, as its interfaces' are: CSS's `escape` is
    // cssom's, its `px` css-typed-om's, left out (rd162).
    n.members.push(...(d.members ?? []).filter((m) => !leftOut.has(m)).map((member) => ({ member, from: d.name })));
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
// A mixin that's a type of its own, the union of the interfaces that
// include it, as TypeScript's `Document | ShadowRoot` is, React DOM's
// `HoistableRoot` (rd193): a value of any of them, given each member they
// all have, their nearest common ancestor's by `Deref` and each mixin each
// includes, itself or by an ancestor, `root.querySelector(..)` of
// `ParentNode`'s. Each includer is one, `DocumentOrShadowRoot::of(document)`
// (ADR 1024).
const UNIONS: Record<string, string[]> = { DocumentOrShadowRoot: ["Document", "ShadowRoot"] };
const chainOf = (name: string): string[] => {
  const parent = interfaces.get(name)?.parent;
  return parent ? [name, ...chainOf(parent)] : [name];
};
const mixinsOf = (name: string): Set<string> => new Set(chainOf(name).flatMap((level) => includes.get(level) ?? []));
for (const [union, includers] of Object.entries(UNIONS)) {
  const chains = includers.map(chainOf);
  const parent = chains[0].find((level) => level !== includers[0] && chains.every((chain) => chain.includes(level)));
  const shared = [...mixinsOf(includers[0])].filter((mixin) => includers.every((i) => mixinsOf(i).has(mixin)));
  interfaces.set(union, {
    name: union,
    parent,
    members: shared.flatMap((mixin) => (mixins.get(mixin) ?? []).map((member) => ({ member, from: includers[0] }))),
    constructible: false,
    ts: includers.join(" | "),
    includers,
  });
}
// What every JS global scope has, a window's, a worker's or Node's: its
// functions, `fetch` and `queueMicrotask`, called bare, where a window's
// are `window.fetch` (ADR 0269). Not its timers, whose handler here is
// text to run, which the builtins crate's take as closures (ADR 0102).
const TIMERS = ["setTimeout", "setInterval", "clearTimeout", "clearInterval"];
/** A window's `onerror`'s parameters, TypeScript's `OnErrorEventHandlerNonNull`'s. */
const ERROR_HANDLER = "Fn(EventOrStr<'_>, Option<&str>, Option<f64>, Option<f64>, Option<&Unknown>)";
/** What an event handler property holds, by its type, given its event:
 * `onclick`'s function takes it; `onbeforeunload`'s gives the text that
 * asks whether to leave, or none; `onerror`'s, on a global object, a
 * window, is given an error's message, where it was, and what was thrown,
 * and gives whether the error is handled, `true` not reported, while an
 * element's, whose `onerror` JS calls with the event alone, as the spec
 * says, is an `EventHandler` (rd192). Its getter gives the function JS
 * holds, a `&'static dyn Fn` (ADR 0653); of `onclick`'s, TypeScript's
 * `any` result is nothing a caller reads, and of `onerror`'s it's any
 * value, an `Unknown`, as one a JS program set may give none. Its setter
 * takes the union its trait, `into`, is: a Rust function or closure, the
 * function the getter gave, or any JS function value, each as it is, so a
 * handler read back is compared with the function set by `std::ptr::eq`,
 * `node.onclick === noop` (ADR 0653, amended), and a promise's `resolve`
 * is one, `link.onload = resolve` (ADR 1021, amended). */
const HANDLERS: Record<string, { into: (event: string) => string; get: (event: string) => string }> = {
  EventHandler: { into: (event) => `IntoEventHandler<${event}>`, get: (event) => `Fn(&${event})` },
  OnBeforeUnloadEventHandler: {
    into: (event) => `IntoOnBeforeUnloadEventHandler<${event}>`,
    get: (event) => `Fn(&${event}) -> Option<String>`,
  },
  OnErrorEventHandler: {
    into: () => "IntoOnErrorEventHandler",
    get: () => `${ERROR_HANDLER} -> Option<&'static Unknown>`,
  },
};

const globalScope: Interface = {
  name: "WindowOrWorkerGlobalScope",
  members: (mixins.get("WindowOrWorkerGlobalScope") ?? [])
    .filter((member) => (member.type === "operation" || (member.type === "attribute" && !GLOBALS.some(([global]) => global === member.name))) && !TIMERS.includes(member.name ?? ""))
    .map((member) => ({ member, from: "WindowOrWorkerGlobalScope" })),
  constructible: false,
  isNamespace: true,
  isGlobal: true,
};
// And a window's own, called bare too, as TypeScript's DOM declares them
// globals, `confirm(message)`: what a page's code calls, where
// `window.confirm(message)` is the method still (ADR 0269).
// So are its attributes, as TypeScript declares them, `declare var
// devicePixelRatio: number`, read bare, `devicePixelRatio`, which a
// program tests as upstream React does, `typeof devicePixelRatio ===
// "number"`, where a window may not be (rd195); not the globals the crate
// names (`GLOBALS`), nor an event handler.
const windowScope: Interface = {
  name: "Window",
  members: (interfaces.get("Window")?.members ?? []).filter(
    ({ member }) =>
      (member.type === "operation" ||
        (member.type === "attribute" &&
          !GLOBALS.some(([global]) => global === member.name) &&
          !((member.idlType?.idlType as string) in HANDLERS))) &&
      !TIMERS.includes(member.name ?? "") &&
      !globalScope.members.some((own) => own.member === member),
  ),
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

/** An interface's name with its namespace, if it has one: `WebAssemblyModule`. */
const qualified = (name: string) => (interfaces.get(name)?.legacyNamespace ?? "") + name;

/** What JS calls an interface: `WebAssembly.Module`, `Element`. */
const jsName = (i: Interface) => (i.legacyNamespace ? `${i.legacyNamespace}.${i.name}` : i.name);

/** Types, as WebIDL and TypeScript's DOM lib name them: `HTMLInputElement`,
 * `DOMRect`; one of a namespace with it, `Module` → `WebAssemblyModule`. */
const typeName = (name: string) => qualified(name);

// ── Types ───────────────────────────────────────────────────────────────

/** The whole numbers each WebIDL integer is, as JS is given them, which
 * `#[rust_js::whole]` says of a binding's `f64` (ADR 0994, amended): a
 * `long long`'s nearest number, whole too. */
const INTEGERS: Record<string, string> = {
  byte: "-128..=127", octet: "0..=255", short: "-32768..=32767", "unsigned short": "0..=65535",
  long: "-2147483648..=2147483647", "unsigned long": "0..=4294967295",
  "long long": "-9223372036854775808..=9223372036854775807", "unsigned long long": "0..=18446744073709551615",
};

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
/** The values the DOM converts, whatever TypeScript's `string` or `number`
 * says, by WebIDL's `ToString` or `ToNumber`, which React DOM gives them as
 * they are, `node.setAttribute(name, value)` of a number or a `bool`,
 * `select.size = value` and `img.src = value` of a prop's value: any value,
 * `impl IntoUnknown`, as JS takes one (the user's decision, React port gap
 * d13a). Each `Interface.member`, its value the last parameter. */
const STRINGIFIED = new Set([
  "Element.setAttribute", "Element.setAttributeNS", "ProcessingInstruction.setAttribute",
  "CSSStyleDeclaration.setProperty", "CSSStyleProperties.set cssFloat", "HTMLInputElement.set type",
  "HTMLInputElement.set name", "HTMLSelectElement.set size", "HTMLImageElement.set src",
  "HTMLImageElement.set srcset", "CSSStyleProperties.set display", "CSSStyleProperties.set margin",
  "CSSStyleProperties.set marginTop", "CSSStyleProperties.set marginBottom",
]);

const CLONED = new Set(["postMessage.message", "pushState.data", "replaceState.data", "structuredClone.value"]);

// The console's logging operations, which run no code but the host's and
// what they're given, a value's `toString` or getter as they format it
// (ADR 0898): never what's elsewhere, as `console.trace` may, by
// `Error.prepareStackTrace`. WebIDL doesn't say which they are.
const RUNS_ONLY_GIVEN = new Set([
  "console.assert", "console.debug", "console.error", "console.group", "console.groupCollapsed",
  "console.groupEnd", "console.info", "console.log", "console.warn",
]);

/** The Rust type for a (non-union) WebIDL type, or why there isn't one. */
function rustType(t: IdlType, at: Position): string | { skip: string } {
  if (at === "result" && isUnion(t)) return resultUnion(t);
  if (t.union) return { skip: "union" };
  // A promise a function returns is `.await`ed in Rust (ADR 0029). One it
  // takes is passed as it is: `compile_streaming(window.fetch(..))`.
  if (t.generic === "Promise") {
    const inner = rustType((t.idlType as IdlType[])[0], "result");
    return typeof inner === "string" ? `Promise<${inner}>` : inner;
  }
  // A sequence a function takes is a slice, a JS array of its items as
  // they are (ADR 0219): `new Blob([text, "!"])` of `&[BlobPart]`.
  // An `ObservableArray`, `adoptedStyleSheets`, is a JS array to JS: given
  // as one, and read as one, its items as they are.
  // A string in one is a `String`, as one a function gives is, so an array
  // a program holds, `Vec<String>`, or a getter gave, `&'static [String]`,
  // is given as it is, not copied to `&str`s (rd206): `types:
  // Some(&types)`, `attribute_filter: Some(&["class".into()])`.
  if ((t.generic === "sequence" || t.generic === "FrozenArray" || t.generic === "ObservableArray") && at === "param") {
    const item = paramType((t.idlType as IdlType[])[0]);
    return item ? `&[${item === "&str" ? "String" : item}]` : { skip: t.generic };
  }
  // One a function gives is a `Vec`, a new array each time; a frozen
  // array, the same one, which JS won't change, a slice of it.
  if ((t.generic === "sequence" || t.generic === "FrozenArray" || t.generic === "ObservableArray") && at === "result") {
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
  const alias = windowAliases.get(name);
  if (alias) return rustType({ ...t, idlType: alias }, at);
  // A number is TypeScript's `number`, an `f64`, given and read: one a
  // function takes the host converts as WebIDL says, `ToUint32` of an
  // `unsigned long`'s, wrapping, and a required one is any number that's
  // an `f64`, `impl Into<f64>` (\`generic\`) (ADR 0994, amended).
  if (NUMBERS[name]) return name === "boolean" ? "bool" : "f64";
  if (STRINGS.has(name) || enums.has(name)) return at === "param" ? "&str" : "String";
  if (name === "EventListener" && at === "param") return "Box<dyn FnMut(&Event)>";
  // A callback a function takes is a closure, given what JS calls it with
  // as a function's parameters are, `None` of a `null` one:
  // `new IntersectionObserver(|entries, observer| ..)`. An event handler
  // attribute's, `onclick`, isn't: a listener is `add_event_listener`'s,
  // typed by its event.
  const callback = name.endsWith("EventHandlerNonNull") ? undefined : callbacks.get(name);
  if (callback && at === "param" && (callback.idlType as IdlType).generic === "Promise") return promisedCallback(callback);
  if (callback && at === "param") {
    const args = (callback.arguments ?? []).map((a) => {
      const type = paramType(a.idlType);
      return type && a.idlType.nullable ? `Option<${type}>` : type;
    });
    const result = rustType(callback.idlType!, "result");
    if (args.some((a) => !a || a === ANY) || typeof result !== "string") return { skip: "callback" };
    return `Box<dyn FnMut(${args.join(", ")})${result === "()" ? "" : ` -> ${result}`}>`;
  }
  // A function JS gives is a `dyn Fn` of its signature, `Function`'s, which
  // says none, `dyn Fn() -> Option<&'static Unknown>`, called of nothing
  // for any value, as `js::Kind::Function` is (ADRs 0653, 0700). What's
  // no function isn't one: a callback interface, `walker.filter`, a
  // function or an object of `acceptNode`; and a constructor,
  // `customElements.get(name)`'s, which `new` calls.
  if (at === "result" && name === "Function") return "&'static dyn Fn() -> Option<&'static Unknown>";
  if (at === "result" && callbacks.has(name) && !constructors.has(name)) {
    // An `any` it's given is a value of any shape, as JS has it.
    const args = (callbacks.get(name)!.arguments ?? []).map((a) => (a.idlType.idlType === "any" ? "Option<&Unknown>" : paramType(a.idlType)));
    const result = rustType(callbacks.get(name)!.idlType!, "result");
    if (args.some((a) => !a || a === ANY) || typeof result !== "string") return { skip: "callback" };
    return `&'static dyn Fn(${args.join(", ")})${result === "()" ? "" : ` -> ${result}`}`;
  }
  if (at === "result" && (callbacks.has(name) || callbackInterfaces.has(name))) return "&'static JsObject";
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

/** The dictionaries `name` extends, its parent first. */
function dictionaryAncestors(name: string): string[] {
  const parent = dictionaries.get(name)?.inheritance;
  return parent ? [parent, ...dictionaryAncestors(parent)] : [];
}

/** A dictionary's members, its parents' first: `AddEventListenerOptions` has `capture`. */
function dictionaryMembers(d: Def): Member[] {
  const parent = d.inheritance ? dictionaries.get(d.inheritance) : undefined;
  return [...(parent ? dictionaryMembers(parent) : []), ...((d.members ?? []) as Member[]), ...(partialMembers.get(d.name) ?? [])];
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
      const given = FIELD_TYPES[`${d.name}.${m.name}`] ?? paramType(m.idlType!);
      // A field of any value, `any`'s, can't be generic, as a parameter is:
      // it's a `dyn IntoUnknown`, as a rest parameter's item is, `error:
      // Some(&error)` (ADR 0654). Not yet one of any object, `object`'s,
      // whose `dyn Any` only a parameter gives as its value, nor a list of
      // `any`s.
      const chosen = given === ANY ? "&dyn IntoUnknown" : given;
      const object = chosen?.includes("dyn core::any::Any") && !chosen.startsWith("&[");
      if (!chosen || object || chosen.includes(ANY)) continue;
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

/**
 * What a getter of a union of a Trusted Type and a string gives: the string,
 * `(TrustedHTML or DOMString) innerHTML`'s `DOMString`. A Trusted Type is
 * what a setter is given, never what a getter gives (Trusted Types, the
 * spec), and TypeScript declares it so, `innerHTML: string`; its setter
 * takes the string too (`paramType`'s `alternatives`) (rd125). A union of
 * other unbound members, `(RTCRtpScriptTransform or SFrameTransform)`,
 * may give either, so it's no one member's.
 */
const TRUSTED = new Set(["TrustedHTML", "TrustedScript", "TrustedScriptURL"]);
function soleMember(t: IdlType): IdlType | undefined {
  if (!isUnion(t) || unionOf(t)) return undefined;
  const members = (u: IdlType): IdlType[] => {
    const aliased = !u.union && !u.generic && typedefs.get(u.idlType as string);
    if (aliased) return members(aliased);
    return u.union ? (u.idlType as IdlType[]).flatMap(members) : [u];
  };
  const [trusted, bound] = [members(t).filter((u) => TRUSTED.has(u.idlType as string)), members(t).filter((u) => !TRUSTED.has(u.idlType as string))];
  if (trusted.length === 0 || bound.length !== 1 || !STRINGS.has(bound[0].idlType as string)) return undefined;
  // (webidl2's fields are getters, so each is read, not spread.)
  const [u] = bound;
  return { generic: u.generic, union: u.union, idlType: u.idlType, nullable: u.nullable || t.nullable } as IdlType;
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
  if (promisedOf(rust)) return "function";
  if (rust === "bool") return "boolean";
  if (Object.values(NUMBERS).includes(rust)) return "number";
  if (rust.startsWith("Box<dyn FnMut")) return "function";
  if (rust.startsWith("&[")) return "array";
  if (rust.includes("dyn core::any::Any")) return null;
  if (isDictionary(rust)) return "object";
  return rust.replace(/^&/, "").replace(/<.*$/, "");
}

/** The callback of a result JS resolves a type is, `&dyn IntoViewTransitionUpdateCallback`'s. */
function promisedOf(rust: string): string | undefined {
  const name = rust.match(/^(?:Box<dyn |&(?:'a )?dyn )Into(\w+)>?$/)?.[1];
  return name && promised.has(name) ? name : undefined;
}

/** A variant's name: its type's, or of a primitive its kind's, `Str`. */
function variantName(rust: string): string {
  const kind = kindOf(rust);
  if (kind === "string") return "Str";
  if (kind === "boolean") return "Bool";
  if (kind === "number") return "Number";
  if (kind === "function") return promisedOf(rust) ?? "Listener";
  if (kind === "array") return "List";
  return rust.replace(/^&/, "").replace(/<.*$/, "");
}

/** What TypeScript calls a variant's type: `string`, `Node`, `HTMLElement`. */
function tsName(rust: string): string {
  const kind = kindOf(rust);
  if (kind === "string" || kind === "boolean" || kind === "number") return kind;
  if (kind === "function") return promisedOf(rust) ?? "EventListener";
  const name = variantName(rust);
  const idl = INTERFACES.find((n) => typeName(n) === name);
  return idl ? jsName(interfaces.get(idl)!) : name;
}

/** A WebIDL union as an untagged enum (ADR 0215): a variant per kind. */
type Union = { name: string; variants: { name: string; type: string; ts: string }[]; borrows: boolean; ts: string };
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
  const name = typedef ? typeName(typedef) : memberNames(t).join("Or");
  return { name, variants, borrows: variants.some((v) => v.type.includes("'a")), ts: tsOf(t).join(" | ") };
}

// The types TypeScript's DOM declares by name, `BodyInit`.
const tsAliases = new Set(
  [...readFileSync(new URL("./node_modules/@types/web/index.d.ts", import.meta.url), "utf8").matchAll(/^(?:type|interface) (\w+)\b/gm)].map((m) => m[1]),
);

/** How TypeScript writes a union: a typedef of one by its name, `BodyInit`,
 * where TypeScript's DOM has it and Rust takes each of its members, else
 * its members, `string | BufferSource`: `HeadersInit` would take a record
 * Rust doesn't. */
function tsOf(t: IdlType): string[] {
  if (t.union) return [...new Set((t.idlType as IdlType[]).flatMap(tsOf))];
  const name = t.idlType as string;
  const aliased = !t.generic && typedefs.get(name);
  if (aliased && isUnion(aliased)) {
    const taken = flatMembers(aliased).every((member) => typeof rustType(member, "param") === "string");
    return taken && tsAliases.has(name) ? [name] : tsOf(aliased);
  }
  const rust = rustType(t, "param");
  return typeof rust === "string" ? [tsName(rust)] : [];
}

/** A union's members as it's written, for its name: a typedef's its own,
 * `(DOMString or BufferSource)` `StrOrBufferSource`, as TypeScript's
 * `string | BufferSource` is; another's its variant's. */
function memberNames(t: IdlType): string[] {
  if (t.union) return [...new Set((t.idlType as IdlType[]).flatMap(memberNames))];
  const name = t.idlType as string;
  if (!t.generic && typedefs.has(name) && isUnion(t)) return alternatives(t).length ? [typeName(name)] : [];
  const rust = rustType(t, "param");
  return typeof rust === "string" ? [variantName(rust)] : [];
}

/** A union's members, through the typedefs of unions in it. */
function flatMembers(t: IdlType): IdlType[] {
  if (t.union) return (t.idlType as IdlType[]).flatMap(flatMembers);
  const aliased = !t.generic && typedefs.get(t.idlType as string);
  return aliased && isUnion(aliased) ? flatMembers(aliased) : [t];
}

/**
 * A union a function gives: the enum a parameter of it takes (ADR 0215),
 * of `'static` borrows, `StrOrArrayBuffer<'static>`, read as the member
 * JS gives is. Only where Rust takes each member and JS tells each apart:
 * a member left out, or two of one kind, would be misread. `undefined`
 * among them is `None`: `Promise<Option<&Response>>` of `Cache.match`.
 */
function resultUnion(t: IdlType): string | { skip: string } {
  const members = flatMembers(t);
  const defined = members.filter((m) => m.idlType !== "undefined");
  const option = (rust: string) => (defined.length < members.length ? `Option<${rust}>` : rust);
  if (defined.length === 1) {
    const one = rustType(defined[0], "result");
    return typeof one === "string" ? option(one) : one;
  }
  const union = unionOf(t);
  if (!union || union.variants.length !== defined.length || union.variants.some((v) => kindOf(v.type.replace(/'a /g, "")) === "object")) {
    return { skip: "union" };
  }
  named(union);
  return option(union.borrows ? `${union.name}<'static>` : union.name);
}

/**
 * The Rust type a parameter, a setter or a dictionary's field takes: a
 * union's enum (ADR 0215), or the one type of its that Rust can take.
 */
function paramType(t: IdlType): string | undefined {
  const union = isUnion(t) ? unionOf(t) : null;
  if (!union) return alternatives(t)[0];
  named(union);
  return `${union.name}${union.borrows ? "<'_>" : ""}`;
}

/**
 * The union of `union`'s name, one enum for every union named so, by its
 * members, `StrOrNumber`: two whose numbers differ, `(DOMString or unsigned
 * long)`'s `u32` and `(DOMString or DOMHighResTimeStamp)`'s `f64`, are one
 * of the wider, `f64`, which each takes, as TypeScript's `number` is either.
 * One union of the name, as the last read was, was narrower than its other
 * members: `PerformanceMeasureOptions.start` took no time (ADR 0963).
 */
function named(union: Union): void {
  const known = unions.get(union.name);
  const wider = (a: string, b: string) => (a === b ? a : "f64");
  const variants = union.variants.map((v) => {
    const other = known?.variants.find((k) => k.name === v.name);
    return other && kindOf(v.type) === "number" ? { ...v, type: wider(v.type, other.type) } : v;
  });
  unions.set(union.name, { ...union, variants });
}

// ── Generating ──────────────────────────────────────────────────────────

const skipped = new Map<string, number>();
const skip = (why: string) => skipped.set(why, (skipped.get(why) ?? 0) + 1);

/** A binding; `nullable`, its parameters whose `None` is `null` (ADR 0275);
 * `none`, what its `Option` result's `None` is to JS where it's only one
 * (`noneOf`); `whole`, the whole numbers its `f64` is, a WebIDL integer's
 * (`wholeOf`); `variadic`, its last a rest parameter's tuple (ADR 0640). */
type Fn = {
  name: string;
  jsName: string;
  params: string[];
  result: string;
  doc: string[];
  nullable?: string[];
  none?: "null" | "undefined";
  whole?: string;
  typeParams?: string[];
  variadic?: boolean;
  /** It runs no code but its own, not even what it's given,
   * `#[rust_js::runs_nothing]` (ADR 0867): an event handler's setter, which
   * keeps the function it's given (ADR 1006, amended). */
  runsNothing?: boolean;
};

/** The interfaces a rest parameter's arguments are, `...streams:
 * MediaStream[]`'s: each `Takes` a reference to it, and to each that
 * extends it (ADR 0640). */
const restInterfaces = new Set<string>();

/**
 * What a rest parameter of `type`, a parameter's Rust, is a tuple of, its
 * `Rest<T>`'s `T` (ADR 0640): `any`'s `dyn IntoUnknown`, a union's `dyn
 * IntoNodeOrStr`, text's `str`, an interface's `MediaStream`, each of
 * which `Takes` what its parameter takes.
 */
function restItem(type: string): string {
  if (type === ANY) return "dyn IntoUnknown";
  const union = unions.get(type.replace(/<'_>$/, ""));
  if (union) return `dyn Into${union.name}`;
  if (type === "&str") return "str";
  const of = type.match(/^&(\w+)$/)?.[1];
  const named = of && INTERFACES.find((n) => typeName(n) === of);
  if (named) {
    restInterfaces.add(named);
    return of;
  }
  throw new Error(`a rest parameter of \`${type}\`: no \`Takes\` says what it takes`);
}

// Operations that call each callback they're given once at most, a request
// for one later call or for one answer: each a `Box<dyn FnOnce(..)>`, as
// the js crate's `setTimeout` takes, so it may move what it holds out
// (ADR 0869). WebIDL doesn't say it; each spec's algorithm does.
const CALLED_ONCE = new Set([
  "Window.queueMicrotask",
  "WindowOrWorkerGlobalScope.queueMicrotask",
  "Window.requestAnimationFrame",
  "Window.requestIdleCallback",
  "HTMLVideoElement.requestVideoFrameCallback",
  "HTMLCanvasElement.toBlob",
  "DataTransferItem.getAsString",
  "Geolocation.getCurrentPosition",
  "BaseAudioContext.decodeAudioData",
  "Scheduler.postTask",
  "LockManager.request",
  "FileSystemDirectoryEntry.getFile",
  "FileSystemDirectoryEntry.getDirectory",
  "FileSystemDirectoryReader.readEntries",
  "FileSystemEntry.getParent",
  "FileSystemFileEntry.file",
  "Notification.requestPermission",
  "RTCPeerConnection.createOffer",
  "RTCPeerConnection.createAnswer",
  "RTCPeerConnection.setLocalDescription",
  "RTCPeerConnection.setRemoteDescription",
  "RTCPeerConnection.addIceCandidate",
]);

/**
 * A callback whose result is a promise, WebIDL's `Promise<any> ()` of a
 * `ViewTransitionUpdateCallback`, gives JS what JS resolves, as a promise's
 * `then` does: a promise, or any value, `()` `undefined`, as TypeScript's
 * `(): any` and `(): void | PromiseLike<void>` are (rd206). So it's a
 * function of any result, `Into<Name>`, which a closure or a function is
 * (`ADR 1026`). What JS keeps is JS's, as ADR 1021 says of a handler:
 * where Rust gives it, moved, a `Box`, which may own what it holds; where
 * Rust lends it, a union's `Option<&dyn ..>` (ADR 0443) or a field of a
 * dictionary in one, `&dyn`, `Copy`, `Fn`, which owns nothing Rust drops
 * when the borrow ends, while JS still holds it.
 */
type Promised = { args: string[]; lent: boolean; once: boolean; ts: string };
const promised = new Map<string, Promised>();
function promisedCallback(callback: Def): string | { skip: string } {
  const name = callback.name;
  let known = promised.get(name);
  if (!known) {
    const args = (callback.arguments ?? []).map((a) => {
      const type = paramType(a.idlType);
      return type && a.idlType.nullable ? `Option<${type}>` : type;
    });
    if (args.some((a) => !a || a === ANY)) return { skip: "callback" };
    const uses = callbackUses().get(name) ?? { lent: false, ops: new Set<string>() };
    const once = !uses.lent && uses.ops.size > 0 && [...uses.ops].every((op) => CALLED_ONCE.has(op));
    known = { args: args as string[], lent: uses.lent, once, ts: name };
    promised.set(name, known);
  }
  return known.lent ? `&dyn Into${name}` : `Box<dyn Into${name}>`;
}

/** The bound of a function `Into<name>` takes: its arguments, `'static`
 * borrows, as a handler's event is (ADR 1021), and any result `R`. */
function promisedBound(name: string): string {
  const { args, lent, once } = promised.get(name)!;
  const each = args.map((a) => a.replace(/&(?!')/g, "&'static ").replace(/<'_>/g, "<'static>"));
  return `${lent ? "Fn" : once ? "FnOnce" : "FnMut"}(${each.join(", ")}) -> R${lent ? " + Copy" : ""} + 'static`;
}

/** Where each callback is taken: the operations, `Interface.op`, and
 * whether a union holds it, or a dictionary one holds: lent. */
let uses: Map<string, { lent: boolean; ops: Set<string> }> | undefined;
function callbackUses(): Map<string, { lent: boolean; ops: Set<string> }> {
  if (uses) return uses;
  const found = new Map<string, { lent: boolean; ops: Set<string> }>();
  const use = (name: string) => found.get(name) ?? found.set(name, { lent: false, ops: new Set() }).get(name)!;
  const seen = new Set<string>();
  const walk = (t: IdlType | undefined, op: string | undefined, inUnion: boolean): void => {
    if (!t) return;
    if (Array.isArray(t.idlType)) return t.idlType.forEach((each) => walk(each, op, inUnion || !!t.union));
    const name = t.idlType as string;
    const aliased = typedefs.get(name);
    if (aliased) return walk(aliased, op, inUnion);
    if (callbacks.has(name)) {
      const u = use(name);
      if (inUnion) u.lent = true;
      if (op) u.ops.add(op);
    }
    const d = dictionaries.get(name);
    if (d && !seen.has(`${name}${inUnion}`)) {
      seen.add(`${name}${inUnion}`);
      for (const m of dictionaryMembers(d)) walk(m.idlType, undefined, inUnion);
    }
  };
  for (const d of everywhere) {
    for (const m of d.members ?? []) {
      if (m.type === "operation" || m.type === "constructor") for (const a of m.arguments ?? []) walk(a.idlType, `${d.name}.${m.name}`, false);
    }
  }
  uses = found;
  return found;
}

/** A parameter of an operation `CALLED_ONCE` names: its callbacks called once. */
const once = (param: string) => param.replaceAll("Box<dyn FnMut(", "Box<dyn FnOnce(");

// Operations whose typed form, by an event's or a tag's type (ADR 0223), has
// their name: the form that takes any string is `_named`.
const NAMED: Record<string, string> = {
  "EventTarget.addEventListener": "add_event_listener_named",
  "EventTarget.removeEventListener": "remove_event_listener_named",
  "Document.createElement": "create_element_named",
  "Document.createElementNS": "create_element_ns_named",
};

/** `onerror`'s first argument, TypeScript's `Event | string`. */
const ERROR_ARG: Union = {
  name: "EventOrStr",
  variants: [{ name: "Event", type: "&'a Event", ts: "Event" }, { name: "Str", type: "&'a str", ts: "string" }],
  borrows: true,
  ts: "Event | string",
};

/** What `for..of` gives of `i`: an iterable's value, or its key and value,
 * or an indexed getter's item, `getter Element? item(unsigned long index)`;
 * its own, or its parent's. */
function iterated(i: Interface): string | undefined {
  for (const { member: m } of i.members) {
    if (m.type === "iterable" || m.type === "maplike" || m.type === "setlike") {
      const types = (m as unknown as { idlType: IdlType[]; async?: boolean }).idlType;
      if ((m as unknown as { async?: boolean }).async) return undefined;
      const each = types.map((t) => rustType(t, "result"));
      if (each.some((t) => typeof t !== "string")) return undefined;
      return each.length === 2 ? `(${each.join(", ")})` : (each[0] as string);
    }
  }
  for (const { member: m } of i.members) {
    const index = m.arguments?.[0]?.idlType.idlType;
    if (m.type === "operation" && m.special === "getter" && m.arguments?.length === 1 && index === "unsigned long") {
      const item = rustType(m.idlType!, "result");
      return typeof item === "string" ? item : undefined;
    }
  }
  const parent = i.parent && interfaces.get(i.parent);
  return parent ? iterated(parent) : undefined;
}

/** What a named property's unnamed special is (#429): its getter,
 * `get(name)`, `this[name]`, of what the name holds, `None` where it holds
 * nothing, `undefined`; its setter, `set(name, value)`, `this[name] =
 * value`; its deleter, `delete(name)`, `delete this[name]`. */
function namedProperty(i: Interface, m: Member, self: string[]): Fn | { skip: string } {
  const doc = [`[MDN](${mdn(i.name)}): its named property, \`${m.special} (DOMString name)\`.`];
  if (m.special === "getter") {
    const item = rustType(m.idlType!, "result");
    if (typeof item !== "string") return item;
    // Of text alone, `DOMStringMap`'s: a document's and a window's are
    // what JS names on them, elements and windows, not what they hold.
    if (item !== "String") return { skip: "a named getter not of text" };
    return { name: "get", jsName: "get []", params: [...self, "name: &str"], result: `Option<${item}>`, doc, none: "undefined" };
  }
  if (m.special === "setter") {
    const value = m.arguments?.[1] && paramType(m.arguments[1].idlType);
    if (!value) return { skip: "a named setter's value" };
    return { name: "set", jsName: "set []", params: [...self, "name: &str", `value: ${value}`], result: "()", doc };
  }
  return { name: "delete", jsName: "delete []", params: [...self, "name: &str"], result: "()", doc };
}

/** A stringifier's `toString()`, `url.to_string()`, its text. */
const toString = (i: Interface): Fn => ({
  name: "to_string",
  jsName: "toString",
  params: [`this: &${typeName(i.name)}`],
  result: "String",
  doc: [`[MDN](${mdn(i.name, "toString")})`],
});

/** The root of `name`'s inheritance chain within INTERFACES. */
function root(name: string): string {
  const parent = interfaces.get(name)!.parent;
  return parent && known.has(parent) ? root(parent) : name;
}

function mdn(iface: string, member?: string) {
  const ns = interfaces.get(iface)?.legacyNamespace;
  // MDN files `CSS` with the Web API's pages, the other namespaces with JS's.
  const js = NAMESPACES.includes(iface) && iface !== "CSS";
  const page = ns ? `JavaScript/Reference/Global_Objects/${ns}/${iface}` : js ? `JavaScript/Reference/Global_Objects/${iface}` : `API/${iface}`;
  return `https://developer.mozilla.org/docs/Web/${page}${member ? `/${member}` : ""}`;
}

/** Each interface's constants, by name: their Rust. */
const constantsOf = new Map<string, Map<string, string>>();

/** `i`'s members, an overload whose arguments begin another's folded into
 * it, the other's later ones optional: `alert()` and `alert(message)` are
 * `alert(message?)`, as TypeScript declares it (ADR 0443). */
function foldedOverloads(i: Interface): Interface {
  // Alike where Rust takes them alike: one name, one Rust type.
  const key = (a: Arg) => JSON.stringify([a.name, paramType(a.idlType) ?? a.idlType, a.variadic]);
  const members = [...i.members];
  for (let a = 0; a < members.length; a++) {
    const shorter = members[a].member;
    if (shorter.type !== "operation" || !shorter.name) continue;
    for (let b = a + 1; b < members.length; b++) {
      const longer = members[b].member;
      const short = shorter.arguments ?? [];
      const long = longer.arguments ?? [];
      if (longer.type !== "operation" || longer.name !== shorter.name || longer.special !== shorter.special) continue;
      if (long.length <= short.length || !short.every((arg, j) => key(arg) === key(long[j]))) continue;
      if (JSON.stringify(longer.idlType) !== JSON.stringify(shorter.idlType)) continue;
      // webidl2's fields are getters, so each is read, not spread.
      const arg = (a: Arg, optional: boolean): Arg => ({ name: a.name, idlType: a.idlType, optional, variadic: a.variadic });
      const args = [...short.map((a) => arg(a, a.optional)), ...long.slice(short.length).map((a) => arg(a, true))];
      const { type, name, special, idlType, extAttrs } = shorter;
      members[a] = { ...members[a], member: { type, name, special, idlType, extAttrs, arguments: args } as Member };
      members.splice(b, 1);
      a--;
      break;
    }
  }
  return { ...i, members };
}

/** `i`'s members, an overload whose arguments past another's are each a
 * member of that one's options dictionary dropped, the dictionary's alone
 * kept: `postMessage(message, transfer)` beside `postMessage(message,
 * options?)`, whose `StructuredSerializeOptions` has `transfer`, is the
 * second, as one function has one form (ADRs 0443, 0594). */
function subsumedOverloads(i: Interface): Interface {
  const members = i.members.filter(({ member: older }) => {
    if (older.type !== "operation" || !older.name) return true;
    return !i.members.some(({ member: newer }) => {
      if (newer === older || newer.type !== "operation" || newer.name !== older.name || newer.special !== older.special) return false;
      const args = newer.arguments ?? [];
      const last = args.at(-1);
      const options = last?.optional && typeof last.idlType.idlType === "string" ? dictionaries.get(last.idlType.idlType) : undefined;
      if (!options) return false;
      const shared = args.slice(0, -1);
      const theirs = older.arguments ?? [];
      const names = new Set(dictionaryMembers(options).map((m) => m.name));
      return (
        theirs.length > shared.length &&
        shared.every((arg, k) => theirs[k]?.name === arg.name) &&
        theirs.slice(shared.length).every((arg) => names.has(arg.name))
      );
    });
  });
  return { ...i, members };
}

function functionsOf(declared: Interface): Fn[] {
  const i = foldedOverloads(subsumedOverloads(declared));
  const fns: Fn[] = [];
  // A namespace's functions are called on it: `WebAssembly.compile(bytes)`.
  const self = i.isNamespace ? [] : [`this: &${typeName(i.name)}`];
  const member = (name: string) => (i.isNamespace && !i.isGlobal ? `${i.name}.${name}` : name);
  // A result that may be `null` is an `Option`: `None` in Rust (ADR 0030).
  const nullable = (t: IdlType) => t.nullable || typedefs.get(t.idlType as string)?.nullable;
  const orNull = (rust: string, t: IdlType) => (nullable(t) ? `Option<${rust}>` : rust);
  // What an `Option` result's `None` is to JS, as WebIDL and TypeScript
  // type it: only `null` of a nullable type, `T | null`, which
  // `#[rust_js::nullable]` says; only `undefined` of a union with
  // `undefined`, `T | undefined`, `window.event`'s, which
  // `#[rust_js::optional]` says; either, said by neither, of any other,
  // `any`'s (ADR 0298, amended).
  const noneOf = (t: IdlType, result: string): Fn["none"] => {
    if (!result.startsWith("Option<")) return undefined;
    const members = isUnion(t) ? flatMembers(t) : [t];
    const nulls = nullable(t) || members.some((m) => m.nullable);
    const undefineds = members.some((m) => m.idlType === "undefined");
    return nulls && !undefineds ? "null" : undefineds && !nulls ? "undefined" : undefined;
  };

  // The whole numbers a result is, where it's an `f64` of a WebIDL
  // integer, `status`'s `unsigned short`, through its typedefs, `GLenum`'s
  // (ADR 0994, amended); of a nullable one, an `Option<f64>`, its `Some`'s,
  // `selectionStart`'s `unsigned long?`.
  const wholeOf = (t: IdlType, result: string): string | undefined => {
    if ((result !== "f64" && result !== "Option<f64>") || t.union || t.generic) return undefined;
    const name = t.idlType as string;
    const aliased = typedefs.get(name);
    return aliased ? wholeOf(aliased, result) : INTEGERS[name];
  };

  // Arguments up to the first optional one, each of the one type it takes:
  // a union's is its enum (ADR 0215). Of one that takes `null`, WebIDL's
  // `Node? child`, TypeScript's `Node | null`, which `nulls` names: an
  // `Option` of it, its `None` `null` (`#[rust_js::nullable(child)]`), as a
  // setter's is (ADR 0275, amended, rd151): `insert_before(node, None)`.
  // A union's and `any`'s take it already.
  const signatures = (args: Arg[]): { names: string[]; types: string[]; nulls: string[] } | { skip: string } => {
    const names: string[] = [];
    const types: string[] = [];
    const nulls: string[] = [];
    for (const a of args) {
      if (a.optional || a.variadic) break;
      const type = paramType(a.idlType);
      if (!type) {
        const why = rustType(a.idlType, "param");
        return { skip: typeof why === "string" ? "?" : why.skip };
      }
      names.push(snake(a.name));
      types.push(type);
      if (nullable(a.idlType) && !isUnion(a.idlType) && type !== ANY && !type.startsWith("Option<")) nulls.push(snake(a.name));
    }
    return { names, types, nulls };
  };

  // What `#[rust_js::nullable(..)]` names of a function's parameters.
  const nulled = (nulls: string[]): Pick<Fn, "nullable"> => (nulls.length > 0 ? { nullable: nulls } : {});

  // The one form: its required arguments, then each optional one an
  // `Option`, as TypeScript's `x?` (ADR 0443): `encode(this, input)`. The
  // first whose type isn't bound ends them, a caller leaving it out. Then
  // its rest parameter, `...data`, a tuple of what its type takes, the
  // binding variadic (ADR 0640).
  const form = (base: string, sig: { names: string[]; types: string[]; nulls: string[] }, args: Arg[]) => {
    const params = sig.types.map((type, j) => `${sig.names[j]}: ${sig.nulls.includes(sig.names[j]) ? `Option<${type}>` : type}`);
    for (const a of args.filter((a) => !a.variadic)) {
      if (!a.optional) continue;
      const type = paramType(a.idlType);
      if (!type) return { name: base, params, variadic: false };
      params.push(`${snake(a.name)}: Option<${type}>`);
    }
    const rest = args.find((a) => a.variadic);
    const type = rest && paramType(rest.idlType);
    if (!rest || !type) return { name: base, params, variadic: false };
    params.push(`${snake(rest.name)}: impl Rest<${restItem(type)}>`);
    return { name: base, params, variadic: true };
  };

  for (const [index, { member: m }] of i.members.entries()) {
    // A constant is a Rust one, its value written where it's read, as a
    // `const`'s is (ADR 0031): `node::ELEMENT_NODE`.
    if (m.type === "const") {
      const rust = rustType(m.idlType!, "result");
      const value = (m as unknown as { value?: { type: string; value: string } }).value;
      const own = constantsOf.get(i.name) ?? new Map<string, string>();
      if (typeof rust !== "string" || value?.type !== "number") skip("constant");
      else if (!own.has(m.name!)) {
        // A number's, as an `f64` writes it: `0x02`'s `2.0`.
        const n = Number(value.value);
        const literal = rust.startsWith("f") ? (Number.isInteger(n) ? `${n}.0` : `${n}`) : value.value;
        // As WebIDL names it, `FLOAT_MAT2x3` too.
        const allow = /[a-z]/.test(m.name!) ? "    #[allow(non_upper_case_globals)]\n" : "";
        own.set(m.name!, `    /// \`${jsName(i)}.${m.name}\`\n${allow}    pub const ${m.name}: ${rust} = ${literal};`);
      }
      constantsOf.set(i.name, own);
      continue;
    }
    // An iterable, a maplike or a setlike: `forEach` of each item, given
    // what JS gives it, an array's `(value, index, list)` or a map's
    // `(value, key, map)`, and a map's or a set's own methods; and its
    // `keys`, `values` and `entries`, JS iterators, each a
    // `Box<dyn Iterator>`, which rust-js steps, adapts and loops over as
    // the JS iterator it is (ADR 0140).
    if (m.type === "iterable" || m.type === "maplike" || m.type === "setlike") {
      const declared = m as unknown as { idlType: IdlType[]; readonly?: boolean; async?: boolean };
      if (declared.async || i.isNamespace) {
        skip("async iterable");
        continue;
      }
      const [key, value] = declared.idlType.length === 2 ? declared.idlType : [undefined, declared.idlType[0]];
      const own = `&${typeName(i.name)}`;
      const v = paramType(value);
      const k = m.type === "setlike" ? v : key ? paramType(key) : "f64";
      const result = rustType(value, "result");
      if (!v || !k || typeof result !== "string") {
        skip(m.type);
        continue;
      }
      const doc = (name: string) => [`[MDN](${mdn(i.name, name)})`];
      const add = (name: string, jsName: string, params: string[], result: string) =>
        fns.push({ name, jsName, params: [...self, ...params], result, doc: doc(jsName.replace(/^get /, "")) });
      add("for_each", "forEach", [`callback: Box<dyn FnMut(${v}, ${k}, ${own})>`], "()");
      const keyResult = m.type === "setlike" ? result : key ? rustType(key, "result") : "f64";
      if (typeof keyResult === "string") {
        const iterator = (item: string) => `Box<dyn Iterator<Item = ${item}>>`;
        add("keys", "keys", [], iterator(keyResult));
        add("values", "values", [], iterator(result));
        add("entries", "entries", [], iterator(`(${keyResult}, ${result})`));
      }
      if (m.type === "maplike") {
        add("get", "get", [`key: ${k}`], `Option<${result}>`);
        add("has", "has", [`key: ${k}`], "bool");
      }
      if (m.type === "setlike") add("has", "has", [`value: ${v}`], "bool");
      if (m.type !== "iterable") {
        add("size", "get size", [], "f64");
        if (!declared.readonly) {
          if (m.type === "maplike") add("set", "set", [`key: ${k}`, `value: ${v}`], "()");
          if (m.type === "setlike") add("add", "add", [`value: ${v}`], "()");
          add("delete", "delete", [`key: ${k}`], "bool");
          add("clear", "clear", [], "()");
        }
      }
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
      // `new`, its optional arguments `Option`s (ADR 0443).
      const v = form("new", sig, m.arguments ?? []);
      fns.push({ name: v.name, jsName: `new ${jsName(i)}`, params: v.params, result: `&'static ${typeName(i.name)}`, doc: [`[MDN](${mdn(i.name, i.name)})`], variadic: v.variadic, ...nulled(sig.nulls) });
    } else if (m.type === "attribute") {
      // A static attribute is a function of the class's module, read at
      // each call: `notification::permission()`, `Notification.permission`.
      if (m.special === "static") {
        const result = rustType(m.idlType!, "result");
        if (typeof result === "string") {
          const given = orNull(result, m.idlType!);
          fns.push({ name: snake(m.name!), jsName: `get ${jsName(i)}.${m.name}`, params: [], result: given, doc: [`[MDN](${mdn(i.name, m.name)})`], none: noneOf(m.idlType!, given), whole: wholeOf(m.idlType!, given) });
        } else {
          skip(result.skip);
        }
        continue;
      }
      // A global scope's, a function of its module read at each call, bare:
      // `window_global::device_pixel_ratio()`, `devicePixelRatio` (rd195).
      if (i.isNamespace && i.isGlobal) {
        const result = rustType(m.idlType!, "result");
        if (typeof result === "string") {
          const given = orNull(result, m.idlType!);
          fns.push({ name: snake(m.name!), jsName: `get ${m.name}`, params: [], result: given, doc: [`[MDN](${mdn(i.name, m.name)})`], none: noneOf(m.idlType!, given), whole: wholeOf(m.idlType!, given) });
        } else {
          skip(result.skip);
        }
        continue;
      }
      if (i.isNamespace) {
        skip("namespace attribute");
        continue;
      }
      const doc = [`[MDN](${mdn(i.name, m.name)})`];
      // An event handler property, `onclick`: what it holds, a function or
      // none, and a closure to set it to, given the event its name is on this
      // target, as `add_event_listener`'s is (ADR 0223), or `None`, `null`.
      // An element's `onerror` JS calls with the event alone; a global
      // object's, with what the error was (rd192).
      const kind = m.idlType!.idlType as string;
      const handler = HANDLERS[kind === "OnErrorEventHandler" && !i.globalObject ? "EventHandler" : kind];
      if (handler && m.name!.startsWith("on")) {
        const event = listens.get(i.name)?.get(m.name!.slice(2)) ?? "Event";
        const name = snake(m.name!);
        if (handler === HANDLERS.OnErrorEventHandler) unions.set(ERROR_ARG.name, ERROR_ARG);
        const held = `Option<&'static dyn ${handler.get(typeName(event))}>`;
        fns.push({ name, jsName: `get ${m.name}`, params: self, result: held, doc, none: noneOf(m.idlType!, held) });
        fns.push({
          name: `set_${name}`,
          jsName: `set ${m.name}`,
          params: [...self, `value: Option<&dyn ${handler.into(typeName(event))}>`],
          result: "()",
          doc,
          nullable: ["value"],
          runsNothing: true,
        });
        continue;
      }
      // `stringifier attribute USVString href`: `toString()` is it too.
      if (m.special === "stringifier") fns.push(toString(i));
      const sole = soleMember(m.idlType!);
      const result = rustType(sole ?? m.idlType!, "result");
      // A getter whose type isn't supported (a union, say) is skipped, but
      // its setter can still take the union's enum.
      if (typeof result === "string") {
        const given = orNull(result, m.idlType!);
        fns.push({ name: snake(m.name!), jsName: `get ${m.name}`, params: self, result: given, doc, none: noneOf(m.idlType!, given), whole: wholeOf(m.idlType!, given) });
      } else {
        skip(result.skip);
      }
      const forwards = (m.extAttrs ?? []).some((a) => a.name === "PutForwards" || a.name === "Replaceable");
      const value = paramType(m.idlType!);
      if (!m.readonly && !forwards && value) {
        // One that takes `null`, TypeScript's `T | null`, takes a value or
        // `None`, `null`: `set_text_content("x")` and `set_text_content(None)`
        // (ADR 0275, amended).
        const takesNull = nullable(m.idlType!) && !isUnion(m.idlType!) && !value.startsWith("&[");
        // A reference it takes is of a lifetime it names, as `impl Trait`
        // can't elide one: `impl Into<Option<&'a str>>`.
        const borrows = value.startsWith("&");
        const taken = borrows ? `&'a ${value.slice(1)}` : value;
        const param = takesNull ? `value: impl Into<Option<${taken}>>` : `value: ${value}`;
        const named = takesNull ? { nullable: ["value"], ...(borrows ? { typeParams: ["'a"] } : {}) } : {};
        fns.push({ name: `set_${snakeWords(m.name!)}`, jsName: `set ${m.name}`, params: [...self, param], result: "()", doc, ...named });
      }
    } else if (m.type === "operation") {
      // `stringifier;`: what JS's `toString()` makes of it.
      if (!m.name && m.special === "stringifier") {
        fns.push(toString(i));
        continue;
      }
      // A named property's `getter DOMString (DOMString name)`, its setter
      // and its deleter, `DOMStringMap`'s: `dataset.get("x")`, `this[name]`,
      // `None` of a name it lacks, `undefined`; `set` and `delete` (#429).
      if (!m.name && m.arguments?.[0]?.idlType.idlType === "DOMString" && m.special && ["getter", "setter", "deleter"].includes(m.special)) {
        const named = namedProperty(i, m, self);
        if ("skip" in named) skip(named.skip);
        else fns.push(named);
        continue;
      }
      if (!m.name) {
        skip("unnamed");
        continue;
      }
      // A window's timers are the js crate's, the one way (ADR 1020): each
      // is gone, `TIMERS_GONE`'s, an error naming it.
      if (i.name === "Window" && TIMERS.includes(m.name)) {
        skip("a timer, the js crate's");
        continue;
      }
      // A media query list's legacy listener methods take a listener as
      // `addEventListener` does, `LEGACY_LISTENERS`' (#439).
      if (i.name === "MediaQueryList" && ["addListener", "removeListener"].includes(m.name)) {
        skip("a legacy listener method, typed");
        continue;
      }
      // A static method is the class's, `URL.createObjectURL(blob)`, in its
      // module, beside an instance's method of its name, `Response.json`.
      const isStatic = m.special === "static";
      const result = rustType(m.idlType!, "result");
      const sig = signatures(m.arguments ?? []);
      if (typeof result !== "string" || "skip" in sig) {
        skip(typeof result !== "string" ? result.skip : (sig as { skip: string }).skip);
        continue;
      }
      const doc = [`[MDN](${mdn(i.name, m.name)})`];
      // A later overload is named, as web-sys does, after the required
      // arguments that set it apart from the first: by name where the first
      // has none there (`scroll_to_with_x_and_y`), by type where
      // the types differ (`instantiate_with_web_assembly_module`).
      const first = i.members.slice(0, index).find(({ member: o }) => o.type === "operation" && o.name === m.name && o.special !== "static");
      const firstRequired = (first?.member.arguments ?? []).filter((a) => !a.optional);
      const required = (m.arguments ?? []).filter((a) => !a.optional);
      const key = (a: Arg) => JSON.stringify(a.idlType.idlType);
      const lead = !first
        ? []
        : required.flatMap((a, j) =>
            // A word of a name, `type`, not a parameter's `r#type`.
            !firstRequired[j] ? [snakeWords(a.name)] : key(firstRequired[j]) !== key(a) ? [suffix(sig.types[j])] : [],
          );
      const name = NAMED[`${i.name}.${m.name}`] ?? snake(m.name);
      const base = lead.length > 0 ? identifier(`${plain(name)}_with_${lead.join("_and_")}`) : name;
      const v = form(base, sig, m.arguments ?? []);
      const jsName = isStatic ? `${i.name}.${m.name}` : member(m.name);
      const params = CALLED_ONCE.has(`${i.name}.${m.name}`) ? v.params.map(once) : v.params;
      const given = orNull(result, m.idlType!);
      fns.push({ name: v.name, jsName, params: isStatic ? params : [...self, ...params], result: given, doc, variadic: v.variadic, none: noneOf(m.idlType!, given), whole: wholeOf(m.idlType!, given), ...nulled(sig.nulls) });
    }
  }

  // A value the DOM makes text of takes any value (`STRINGIFIED`).
  for (const f of fns) {
    if (!STRINGIFIED.has(`${i.name}.${f.jsName}`)) continue;
    const at = f.params.findIndex((p) => p.startsWith("value: "));
    if (at >= 0) f.params[at] = "value: impl IntoUnknown";
  }

  // `for..of`'s `[Symbol.iterator]`, where TypeScript has it: a function of
  // the module, `html_collection::iter(list)`, `Iterator.from(list)`, a JS
  // iterator (ADR 0140) of an iterable's values or entries, or of an indexed
  // collection's items. A method would be called on it, `list.from()`.
  if (!i.isNamespace && typescriptDom.get(i.name)?.members.has("[Symbol.iterator]")) {
    const item = iterated(i);
    if (item) {
      fns.push({
        name: "iter",
        jsName: "Iterator.from",
        params: [`items: &${typeName(i.name)}`],
        result: `Box<dyn Iterator<Item = ${item}>>`,
        doc: [`[MDN](${mdn(i.name, "Symbol.iterator")})`],
      });
    } else {
      skip("[Symbol.iterator]");
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

  // The first of each name wins: later overloads and clashes are skipped,
  // of a method's in the type's `impl` and of a module's in its module.
  const seen = new Set<string>();
  return fns
    .filter((f) => {
      const key = `${f.params[0]?.startsWith("this:") ? "impl" : "mod"} ${f.name}`;
      if (seen.has(key)) {
        skip("overload or clash");
        return false;
      }
      seen.add(key);
      return true;
    })
    .map((f) => throwing(i, f));
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
for (const spec of Object.values(bySpec(await webrefElements.listAll() as Record<string, WebrefElements>))) {
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
for (const spec of Object.values(bySpec(await webrefElements.listAll() as Record<string, WebrefElements>))) {
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
const typed = (doc: string[], js: string, signature: string, visibility = "pub ") =>
  [...doc.map((d) => `    /// ${d}`), `    #[cfg_attr(rust_js, rust_js::link_name = ${JSON.stringify(js)})]`, "    // rust-js writes its JS: the body never runs, nor reads a parameter.", "    #[allow(unused_variables)]", `    ${visibility}fn ${signature} {`, `        unreachable!()`, `    }`].join("\n");
// A listener is TypeScript's `EventListenerOrEventListenerObject`, each
// member as it is (ADRs 0229, 1021): a closure, or a listener JS gave, a
// function or an object JS calls `handleEvent` of, kept by identity.
const listener = "impl IntoEventListener<<Self as Listen<E>>::Event>";
// Removal finds the listener added, so it takes a shared one, `listener(..)`'s,
// or one JS gave: a closure made for the call is a new function, which
// removes nothing.
const shared = "impl IntoAddedEventListener<<Self as Listen<E>>::Event>";
// A window's timers, gone (ADR 1020): a timer is the js crate's global,
// which takes a function as it is, where these took text alone. Each takes
// what it did, of a type that's never a `TimerGone`, so a call is the error
// that says what to write.
const gone = (doc: string, signature: string, read?: string) =>
  [
    `    /// Gone (ADR 1020): ${doc}`,
    ...(read ? [`    #[cfg_attr(rust_js, rust_js::link_name = "${read}")]`] : []),
    "    #[allow(unused_variables)]",
    `    pub fn ${signature} {`,
    `        unreachable!()`,
    `    }`,
  ].join("\n");
const TIMERS_GONE = [
  gone(
    "a timeout is `js::set_timeout(handler, ms)`'s. Read as a value alone,\n    /// `js::method((window, Window::set_timeout))`, `window.setTimeout`, the\n    /// window's own, as React's SchedulerPostTask holds it at load (rd314):\n    /// called, of a `Box<dyn FnOnce()>`, the handler that's a `TimerGone`.",
    "set_timeout<G: TimerGone<fn()>>(&self, handler: G, timeout: Option<f64>) -> &'static js::TimeoutId",
    "setTimeout",
  ),
  gone("a timeout is cleared by `js::clear_timeout(id)`.", "clear_timeout<G: TimerGone>(&self, id: G)"),
  gone("an interval is `js::set_interval(callback, ms)`'s.", "set_interval<G: TimerGone>(&self, handler: G, timeout: Option<f64>, arguments: impl Rest<dyn IntoUnknown>) -> f64"),
  gone("an interval is cleared by `js::clear_interval(id)`.", "clear_interval<G: TimerGone>(&self, id: G)"),
];
// A media query list's legacy `addListener(callback)` and
// `removeListener(callback)`, each a listener as `addEventListener`'s and
// `removeEventListener`'s is (ADR 1021, amended; #439): one added is kept,
// `listener(..)`'s or a named `fn`, so removal finds it, and a closure
// written in place for removal, a new function, is refused. `null`, which
// each ignores, isn't taken.
const LEGACY_LISTENERS = [
  typed([`[MDN](${mdn("MediaQueryList", "addListener")}): \`callback\` called of each change, as`, "`add_event_listener(Change, callback, None)` calls it."], "addListener", "add_listener(&self, callback: impl IntoEventListener<MediaQueryListEvent>)"),
  typed([`[MDN](${mdn("MediaQueryList", "removeListener")}): \`callback\`, added before, gone: the one`, "added, `listener(..)`'s or a named `fn`."], "removeListener", "remove_listener(&self, callback: impl IntoAddedEventListener<MediaQueryListEvent>)"),
];
const TYPED: Record<string, string[]> = {
  MediaQueryList: LEGACY_LISTENERS,
  Window: TIMERS_GONE,
  Document: [
    typed([`[MDN](${mdn("Document", "createElement")}): the element a tag is (ADR 0223),`, "`document.create_element(Button)` an `HTMLButtonElement`. Another name is", "`create_element_named`'s, an `Element`."], "createElement", `create_element<T: Tag>(&self, tag: T) -> &'static <T as Tag>::Element`),
    typed([`[MDN](${mdn("Document", "createElementNS")}): the SVG element a tag is,`, "`document.create_element_ns(namespaces::Svg, svg_tags::Circle)` an `SVGCircleElement`.", "Another is `create_element_ns_named`'s, an `Element`."], "createElementNS", `create_element_ns<T: SVGTag>(&self, namespace: namespaces::Svg, tag: T) -> &'static <T as SVGTag>::Element`),
  ],
};

const out: string[] = [];
const line = (s = "") => out.push(s);

line(`//! The web platform for rust-js: DOM bindings generated by \`webapi/generate.ts\``);
line(`//! from W3C's WebIDL (\`@webref/idl\` ${webref.version}; specs: ${SPECS.join(", ")}). Do not edit.`);
line(`//!`);
line(`//! Each interface is a type (\`Element\`), its members its methods (\`el.append(..)\`),`);
line(`//! and its constructors, statics and constants a module of its name (\`element\`). Inheritance`);
line(`//! is \`Deref\`, so an \`&HTMLButtonElement\` has \`Element\`'s and \`Node\`'s methods and goes`);
line(`//! wherever an \`&Element\` or \`&Node\` is expected. See ADRs 0024 and 0282.`);
line(`//! The JS language's own types, \`Promise\` and \`ArrayBuffer\` say, are the js crate's (ADR 0102).`);
line(`//! Each event's name and each tag is a type too (ADR 0223), from \`@webref/events\` ${eventsPackage.version}`);
line(`//! and \`@webref/elements\` ${elementsPackage.version}: \`events::Click\`, whose value is \`"click"\`.`);
line(`//! Import \`EventTargetExt\` for \`button.add_event_listener(Click, |e: &PointerEvent| ..)\` (ADR 0282).`);
line();
line(`use core::marker::PhantomData;`);
line(`use core::ops::{Deref, Index};`);
line(`use js::{${[...BUILTINS, "Defined", "IntoUnknown", "JsError", "JsObject", "Promise", "Rest", "StructuredClone", "Takes", "Unknown"].sort().join(", ")}};`);
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
  const names: string[] = [...(f.typeParams ?? [])];
  const params = f.params.map((p) => {
    // An optional one by reference, so a `None` needs no type (ADR 0443):
    // a union's `Option<&dyn IntoNodeOrStr>`, an `any`'s `Option<&dyn IntoUnknown>`.
    const named = p.slice(0, p.indexOf(": "));
    const optional = p.slice(named.length + 2).match(/^Option<(.*)>$/)?.[1];
    if (optional === ANY) return `${named}: Option<&dyn IntoUnknown>`;
    const optionalUnion = optional === undefined ? undefined : unions.get(optional.replace(/<'_>$/, ""));
    if (optionalUnion) return `${named}: Option<&dyn Into${optionalUnion.name}>`;
    const union = unionParam(p);
    if (union) return `${p.slice(0, p.indexOf(": "))}: impl Into${union.name}`;
    // A number it must be given is any that's an `f64`, `list.item(i)` of
    // a `u32` too; one it may be given, an `Option<f64>`, whose `None`
    // needs no type (ADR 0994, amended).
    if (p.endsWith(": f64")) return `${named}: impl Into<f64>`;
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
    ...(f.variadic ? ["    #[cfg_attr(rust_js, rust_js::variadic)]"] : []),
    ...(f.nullable ? [`    #[cfg_attr(rust_js, rust_js::nullable(${f.nullable.join(", ")}))]`] : []),
    // Its result's `None`, where JS gives only one (`noneOf`).
    ...(f.none === "null" ? ["    #[cfg_attr(rust_js, rust_js::nullable)]"] : []),
    ...(f.none === "undefined" ? ["    #[cfg_attr(rust_js, rust_js::optional)]"] : []),
    // The whole numbers it gives, a WebIDL integer's (`wholeOf`).
    ...(f.whole ? [`    #[cfg_attr(rust_js, rust_js::whole = ${JSON.stringify(f.whole)})]`] : []),
    ...(RUNS_ONLY_GIVEN.has(f.jsName) ? ["    #[cfg_attr(rust_js, rust_js::runs_only_given)]"] : []),
    ...(f.runsNothing ? ["    #[cfg_attr(rust_js, rust_js::runs_nothing)]"] : []),
    // An operation WebIDL says gives `undefined` gives it, which a closure
    // of its call's value may give back (ADR 0883); not a setter, whose
    // assignment's value is what it's given.
    // Not a setter's, nor a `delete`'s, which gives `true` (#429).
    ...(f.result === "()" && !f.jsName.startsWith("set ") && f.jsName !== "delete []" ? ["    #[cfg_attr(rust_js, rust_js::returns_undefined)]"] : []),
    "    // rust-js writes its JS: the body never runs, nor reads a parameter.",
    "    #[allow(unused_variables)]",
    `    pub fn ${f.name}${names.length > 0 ? `<${names.join(", ")}>` : ""}(${params.join(", ")})${result} {`,
    "        unreachable!()",
    "    }",
  ].join("\n");
}

/**
 * Each CSS property of a style, by its name, as TypeScript has them and
 * CSSOM says (`[CEReactions] attribute CSSOMString _camel_cased_attribute`,
 * which WebIDL can't list): `style.backgroundColor`, text to read and set.
 */
function cssProperties(have: Fn[]): Fn[] {
  const names = new Set(have.map((f) => f.name));
  const fns: Fn[] = [];
  const style = typescriptDom.get("CSSStyleProperties");
  // What its parent, CSSStyleDeclaration, has, which TypeScript lists
  // there of `CSSStyleDeclarationBase`, `length` and `cssText`: the
  // declaration's, which a style reaches through `Deref`, no property.
  const declared = new Set((interfaces.get("CSSStyleDeclaration")?.members ?? []).map(({ member }) => member.name));
  for (const js of [...(style?.members ?? [])].sort()) {
    // Its properties: not a method TypeScript lists there, `setProperty`, a declaration's.
    if (!/^[a-z]\w*$/i.test(js) || names.has(snake(js)) || style?.methods.has(js) || declared.has(js)) continue;
    // Its CSS name: `backgroundColor` is `background-color`, `webkitLineClamp` `-webkit-line-clamp`.
    const css = js.replace(/^webkit(?=[A-Z])/, "-webkit").replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`);
    const doc = [`[\`${css}\`](https://developer.mozilla.org/docs/Web/CSS/${css})`];
    fns.push({ name: snake(js), jsName: `get ${js}`, params: ["this: &CSSStyleProperties"], result: "String", doc });
    // One the DOM makes text of any value of, as React gives it (`STRINGIFIED`).
    const value = STRINGIFIED.has(`CSSStyleProperties.set ${js}`) ? "value: impl IntoUnknown" : "value: &str";
    fns.push({ name: `set_${snakeWords(js)}`, jsName: `set ${js}`, params: ["this: &CSSStyleProperties", value], result: "()", doc });
  }
  return fns;
}

/**
 * Each attribute of a parent class that TypeScript declares again on `i`,
 * never `null`, where WebIDL's may be: `Element`'s `ownerDocument:
 * Document` of `Node`'s `Document?`, which an element always has. A getter
 * of its own, of that type, which a method call finds before the
 * parent's, reached through `Deref`.
 */
function narrowedAttributes(i: Interface, have: Fn[]): Fn[] {
  const names = new Set(have.map((f) => f.name));
  const narrowed = typescriptDom.get(i.name)?.narrowed ?? new Set<string>();
  const fns: Fn[] = [];
  for (let p = i.parent && interfaces.get(i.parent); p; p = p.parent && interfaces.get(p.parent)) {
    for (const { member: m } of p.members) {
      if (m.type !== "attribute" || !m.idlType?.nullable || !narrowed.has(m.name!) || names.has(snake(m.name!))) continue;
      // Its type, of no `Option`, which `orNull` makes of a nullable one.
      const result = rustType(m.idlType, "result");
      if (typeof result !== "string") continue;
      names.add(snake(m.name!));
      fns.push({ name: snake(m.name!), jsName: `get ${m.name}`, params: [`this: &${typeName(i.name)}`], result, doc: [`[MDN](${mdn(i.name, m.name)})`] });
    }
  }
  return fns;
}

/** `pub mod <name> { .. }`, holding a type's or a namespace's functions. */
function module(name: string, all: Fn[], typed: string[] = [], constants: string[] = []) {
  if (all.length === 0 && constants.length === 0) return;
  count += all.length;
  // Each a function with `#[rust_js::link_name]`, the one form of a
  // binding of a function (ADR 0039): an `extern` block's are refused.
  typed = [...all.map(generic), ...typed];
  line();
  line(`pub mod ${name} {`);
  // The crate's types, where its functions name one: `supports(r#type: &str)`
  // names none, nor `supported_content_encodings() -> &'static [String]`,
  // whose `String` is the prelude's, nor `&dyn core::any::Any`, a path, nor
  // `debug<D>(data: D)`, whose `D` is its own.
  const named = [...all.flatMap((f) => [...f.params, f.result]), ...typed]
    .some((t) => /(?<!::)\b(?!(?:String|Vec|Option|Box)\b)[A-Z]\w+/.test(t.replace(/\/\/.*$|"[^"]*"/gm, "")));
  if (named) line(`    use super::*;`);
  for (const c of constants) {
    line();
    line(c);
  }
  for (const t of typed) {
    line();
    line(t);
  }
  line(`}`);
}

/** A `NodeList` is TypeScript's `NodeListOf<T>`, of the type its items
 * are: a member's is what TypeScript says it lists, `querySelectorAll`'s
 * `Element`s, else `Node`s; and its own members give and take a `T`. */
function listing(i: Interface, f: Fn): Fn {
  if (i.name === "NodeList") {
    const of = (t: string) => t.replace(/&('static )?Node\b/g, "&$1T").replace(/&NodeList\b/g, "&NodeList<T>");
    const moduleFn = !f.params[0]?.startsWith("this:");
    return { ...f, params: f.params.map(of), result: of(f.result), ...(moduleFn ? { typeParams: ["T"] } : {}) };
  }
  // A union's, as each it's of lists it.
  const item = typescriptDom.get(i.includers?.[0] ?? jsName(i))?.lists.get(f.jsName.replace(/^get /, ""));
  if (!item || !known.has(item) || !/\bNodeList\b/.test(f.result)) return f;
  return { ...f, result: f.result.replace(/\bNodeList\b/, `NodeList<${typeName(item)}>`) };
}

// Each class's constructor a window holds, its method (ADR 0891).
const windowConstructors: Fn[] = [];
// And the window's own members' names, which no constructor's may take.
const windowMembers = new Set<string>();
for (const name of INTERFACES) {
  const i = interfaces.get(name)!;
  const type = typeName(name);
  const listOf = name === "NodeList";
  line();
  line(i.ts ? `/// \`${i.ts}\`, a value of each: what they all have, as TypeScript's union of them has it.` : `/// [\`${jsName(i)}\`](${mdn(name)})`);
  // Its JS class, as `instanceof` names it, where its Rust name isn't (ADR 0214).
  if (jsName(i) !== type) line(`#[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(jsName(i))})]`);
  // What it is to TypeScript: its DOM lib's interface of the same name.
  line(`#[cfg_attr(rust_js, rust_js::types = ${JSON.stringify(listOf ? "NodeListOf<T>" : (i.ts ?? jsName(i)))})]`);
  line(listOf ? `pub struct ${type}<T = Node>(PhantomData<JsObject>, PhantomData<T>);` : `pub struct ${type}(PhantomData<JsObject>);`);
  // Equal only to itself, as JS's `===` has an object: `ptr::eq`, the
  // binding of `===`, `a == b` and `v.contains(&a)` its `includes(a)`
  // (ADR 1016).
  const generics = listOf ? "<T>" : "";
  line();
  line(`/// \`a === b\`: the same object.`);
  line(`impl${generics} PartialEq for ${type}${generics} {`);
  line(`    #[cfg_attr(rust_js, rust_js::link_name = "===")]`);
  line(`    fn eq(&self, other: &Self) -> bool {`);
  line(`        core::ptr::eq(self, other)`);
  line(`    }`);
  line(`}`);
  line();
  line(`impl${generics} Eq for ${type}${generics} {}`);
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
  // TypeScript's index signature, `[index: number]: HTMLOptionElement`: an
  // item read by `[]`, `options[i]`, as JS reads it, of the class
  // TypeScript says, a `NodeList`'s its `T` (rd68).
  const item = typescriptDom.get(jsName(i))?.indexed;
  if (item && known.has(item)) {
    const output = listOf ? "T" : typeName(item);
    line();
    line(`/// \`list[index]\`, TypeScript's index signature: the item at \`index\`, as JS reads it.`);
    line(`impl${listOf ? "<T>" : ""} Index<usize> for ${type}${listOf ? "<T>" : ""} {`);
    line(`    type Output = ${output};`);
    line();
    line(`    #[cfg_attr(rust_js, rust_js::link_name = "get []")]`);
    line(`    // rust-js writes its JS: the body never runs, nor reads a parameter.`);
    line(`    #[allow(unused_variables)]`);
    line(`    fn index(&self, index: usize) -> &${output} {`);
    line(`        unreachable!()`);
    line(`    }`);
    line(`}`);
  }
  // What's written by hand is bound in place of what's generated of its name.
  const extra = EXTRA[name] ?? [];
  const fns = [...functionsOf(i).filter((f) => !extra.some((e) => e.name === f.name)), ...extra].filter((f) =>
    name !== "EventTarget" || !["addEventListener", "removeEventListener"].includes(f.jsName),
  );
  if (name === "CSSStyleProperties") fns.push(...cssProperties(fns));
  fns.push(...narrowedAttributes(i, fns));
  // Each member is a method, `el.append(..)`, `e.client_x()`, and what a
  // type extends its parent's through `Deref`: one spelling per operation
  // (ADR 0282). Only constructors, statics, constants and casts, which no
  // value of it is given to, stay in its module.
  const methods = fns.filter((f) => f.params[0] === `this: &${type}` && f.jsName !== "this");
  if (name === "Window") for (const f of methods) windowMembers.add(f.name);
  // Its constructor is a window's too, of the class that window holds, as
  // TypeScript's `Window & typeof globalThis` has it: `new
  // window.ErrorEvent(..)`, another window's, an iframe's or jsdom's, apart
  // from the global one (ADR 0891).
  if (i.exposed?.some((scope) => scope === "Window" || scope === "*")) {
    for (const f of fns.filter((f) => f.jsName.startsWith("new "))) {
      const doc = [...f.doc, `\`new window.${jsName(i)}(..)\`: this window's class, which is [\`${snake(qualified(name))}::${f.name}\`]'s only where the window is the global object.`];
      windowConstructors.push({ ...f, name: f.name.replace(/^new/, `new_${snake(qualified(name))}`), params: ["this: &Window", ...f.params], doc });
    }
  }
  module(snake(qualified(name)),fns.filter((f) => !methods.includes(f)).map((f) => listing(i, f)), [], [...(constantsOf.get(name)?.values() ?? [])]);
  count += methods.length;
  // A mixin's union of a value of one of its interfaces, as it is, as a
  // union enum's `of` is (ADR 0229): `DocumentOrShadowRoot::of(document)`.
  const unionOf: Fn[] = [];
  if (i.includers) {
    unionOf.push({
      name: "of",
      jsName: "this",
      params: [`this: &impl IsA<${type}>`],
      result: `&${type}`,
      doc: [`A \`${i.includers.join("` or a `")}\`, or one that extends one, as a \`${i.ts}\`: the value itself.`],
    });
  }
  if (methods.length > 0 || TYPED[name]) {
    line();
    line(listOf ? `impl<T> ${type}<T> {` : `impl ${type} {`);
    for (const f of methods.map((f) => listing(i, f))) {
      line(generic({ ...f, params: ["&self", ...f.params.slice(1)] }));
      line();
    }
    for (const f of unionOf) {
      line(generic(f));
      line();
    }
    for (const t of TYPED[name] ?? []) {
      line(t);
      line();
    }
    line("}");
  }
}

// A window's constructors, each of the class it holds: `new
// window.ErrorEvent(..)`, of each class whose constructor the global
// scope's module has, `error_event::new` (ADR 0891).
{
  const clash = windowConstructors.find((f) => windowMembers.has(f.name));
  if (clash) throw new Error(`Window has a member of its own named ${clash.name}, the name of a constructor it holds`);
  count += windowConstructors.length;
  line();
  line("/// The constructors a window holds, each class's of that window, as TypeScript's");
  line("/// `Window & typeof globalThis` has them: `window.new_error_event(..)` is");
  line("/// `new window.ErrorEvent(..)` (ADR 0891).");
  line("impl Window {");
  for (const f of windowConstructors) {
    line(generic({ ...f, params: ["&self", ...f.params.slice(1)] }));
    line();
  }
  line("}");
}

for (const name of NAMESPACES) {
  // A namespace that's a global value too, TypeScript's `declare var
  // console: Console`: the value's type, of no members of its own, as the
  // namespace's are its module's functions, read as values as any
  // binding is (ADR 0667).
  const value = GLOBALS.find(([global]) => global === name)?.[1];
  if (value) {
    line();
    line(`/// The \`${name}\` namespace as a value, the \`${name}\` global's: its functions are the \`${snake(name)}\` module's.`);
    line(`#[cfg_attr(rust_js, rust_js::types = "${value}")]`);
    line(`pub struct ${value}(PhantomData<JsObject>);`);
  }
  for (const t of TYPES_BY_HAND[name] ?? []) {
    line();
    line(t);
  }
  line();
  line(`/// The [\`${name}\`](${mdn(name)}) namespace.`);
  // What's written by hand is bound in place of what's generated of its name.
  const extra = EXTRA[name] ?? [];
  module(snake(name), [...functionsOf(namespaces.get(name)!).filter((f) => !extra.some((e) => e.name === f.name)), ...extra]);
}

line();
line("/// What every JS global scope has, a window's, a worker's or Node's, called bare: `fetch(url)`.");
module("global", functionsOf(globalScope));

line();
line("/// What a window's global scope has besides, called bare: `confirm(message)`.");
// Each with its options, an `Option`, a trailing `None` no argument (ADR 0443).
const addOptions = ", options: Option<&dyn IntoAddEventListenerOptionsOrBool>";
const removeOptions = ", options: Option<&dyn IntoEventListenerOptionsOrBool>";
// A window's own listener functions, called bare too, as TypeScript's DOM
// declares them globals, `addEventListener('submit', ..)` of React DOM's
// Fizz runtime: the global scope's, where a page's `window` may be another
// (#431), each typed by a window's event map.
const windowListeners = [
  typed([`[MDN](${mdn("EventTarget", "addEventListener")}): the global scope's, the event type a window's.`], "addEventListener",
    `add_event_listener<E>(event: E, listener: impl IntoEventListener<<Window as Listen<E>>::Event>${addOptions}) where Window: Listen<E>`),
  typed([`[MDN](${mdn("EventTarget", "removeEventListener")}): the global scope's, the event type a window's.`], "removeEventListener",
    `remove_event_listener<E>(event: E, listener: impl IntoAddedEventListener<<Window as Listen<E>>::Event>${removeOptions}) where Window: Listen<E>`),
];
module("window_global", functionsOf(windowScope), windowListeners);

// Typed methods need the concrete receiver, not a Deref-erased EventTarget.
// One extension trait supplies them to every target without copying the map.
line();
line("/// Event listener methods. Import this trait to keep the receiver's event map (ADR 0282).");
line("pub trait EventTargetExt: IsA<EventTarget> {");
for (const [name, js, callback, extra] of [
  ["add_event_listener", "addEventListener", listener, addOptions],
  ["remove_event_listener", "removeEventListener", shared, removeOptions],
]) {
  line(typed([`[MDN](${mdn("EventTarget", js)}). The event type comes from this receiver.`], js,
    `${name}<E>(&self, event: E, listener: ${callback}${extra}) where Self: Listen<E>`, ""));
  line();
}
for (const [name, js, callback, extra] of [
  ["add", "addEventListener", "impl IntoEventListener<Event>", addOptions],
  ["remove", "removeEventListener", "impl IntoAddedEventListener<Event>", removeOptions],
]) {
  line(typed(["A custom name, with the base Event."], js,
    `${name}_event_listener_named(&self, name: &str, listener: ${callback}${extra})`, ""));
  line();
}
line("}");
line("impl<T: IsA<EventTarget>> EventTargetExt for T {}");
line();
line(typed(["A shared JavaScript callback, reusable by add_event_listener/remove_event_listener",
  "and an event handler property's setter, set_onclick(Some(&listener(..))).",
  "The function itself, never a wrapper. For shared mutable state, capture a Cell or RefCell.",
  "Specify the event when inference needs it: listener::<PointerEvent>(|e| ..).",
  "A function of its `this`, `function::with_this(..)`'s, is a listener as it is (rd209)."], "this",
  "listener<E>(this: impl Fn(&E) + 'static) -> &'static dyn Fn(&E)").replaceAll(/^    /gm, ""));
line();
line(`/// [\`EventListenerObject\`](https://developer.mozilla.org/docs/Web/API/EventTarget/addEventListener#the_event_listener_callback):
/// an object a listener is, which JS calls \`handleEvent\` of. No global
/// names it: an object, as a value of the union it's in is told by
/// \`typeof\` (ADR 1021).
#[cfg_attr(rust_js, rust_js::types = "EventListenerObject")]
pub struct EventListenerObject(PhantomData<JsObject>);

/// \`a === b\`: the same object.
impl PartialEq for EventListenerObject {
    #[cfg_attr(rust_js, rust_js::link_name = "===")]
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self, other)
    }
}

impl Eq for EventListenerObject {}

/// An object is never nullish.
unsafe impl Defined for EventListenerObject {}

impl EventListenerObject {
${typed(["[MDN](https://developer.mozilla.org/docs/Web/API/EventTarget/addEventListener#the_event_listener_callback): what JS calls of it, given the event."], "handleEvent", "handle_event(&self, object: &Event)")}
}

/// TypeScript's \`EventListenerOrEventListenerObject\`: a listener JS gave,
/// each variant's value the member itself (ADR 0215), \`typeof listener ===
/// "function"\` of a function's (ADR 1021). An event is a JS object, which a
/// listener may keep, so a function's is \`&'static\`.
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy)]
pub enum EventListenerOrEventListenerObject {
    EventListener(&'static dyn Fn(&'static Event)),
    EventListenerObject(&'static EventListenerObject),
}

/// \`a === b\`: the same function or object, as removal finds a listener.
impl PartialEq for EventListenerOrEventListenerObject {
    #[cfg_attr(rust_js, rust_js::link_name = "===")]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::EventListener(a), Self::EventListener(b)) => core::ptr::addr_eq(*a, *b),
            (Self::EventListenerObject(a), Self::EventListenerObject(b)) => core::ptr::eq(*a, *b),
            _ => false,
        }
    }
}

impl Eq for EventListenerOrEventListenerObject {}

/// What a listener parameter of \`add_event_listener\` takes, TypeScript's
/// \`EventListenerOrEventListenerObject\`, each member as it is (ADR 0229): a
/// closure of the event, \`listener(..)\`'s shared one, or a listener JS gave,
/// \`EventListenerOrEventListenerObject\` or an \`&EventListenerObject\`
/// (ADR 1021). Being a union, it gives rustc no closure's signature, so a
/// closure's parameter says its event, \`|e: &PointerEvent| ..\`; an event
/// is a JS object, which a listener may keep, so a function given
/// \`&'static\` events is one too, \`function::with_this\`'s.
#[diagnostic::on_unimplemented(message = "\`{Self}\` is not an \`EventListenerOrEventListenerObject\`", note = "a listener is a closure of the event, \`listener(..)\`'s, or one JS gave, an \`EventListenerOrEventListenerObject\`")]
#[cfg_attr(rust_js, rust_js::types = "EventListenerOrEventListenerObject")]
pub trait IntoEventListener<E>: listeners::Sealed<E> {}
impl<E: 'static, F: FnMut(&'static E) + 'static> IntoEventListener<E> for F {}
impl<E> IntoEventListener<E> for EventListenerOrEventListenerObject {}
impl<E> IntoEventListener<E> for &'static EventListenerObject {}

/// What a listener parameter of \`remove_event_listener\` takes: one added
/// before, as it was, \`listener(..)\`'s shared function, a named \`fn\` of
/// the event, or a listener JS gave (ADR 1021). A closure written in place
/// is a new function, which removes nothing: rust-js refuses it
/// (\`rust_js::given_before\`).
#[diagnostic::on_unimplemented(message = "\`{Self}\` is not a listener added before", note = "removal finds the listener added: a shared one, \`listener(..)\`'s, a named \`fn\`, or one JS gave, an \`EventListenerOrEventListenerObject\`")]
#[cfg_attr(rust_js, rust_js::types = "EventListenerOrEventListenerObject")]
#[cfg_attr(rust_js, rust_js::given_before)]
pub trait IntoAddedEventListener<E>: listeners::Sealed<E> {}
impl<E: 'static, F: Fn(&'static E) + Copy + 'static> IntoAddedEventListener<E> for F {}
impl<E> IntoAddedEventListener<E> for EventListenerOrEventListenerObject {}
impl<E> IntoAddedEventListener<E> for &'static EventListenerObject {}

mod listeners {
    use super::*;

    pub trait Sealed<E> {}
    impl<E: 'static, F: FnMut(&'static E) + 'static> Sealed<E> for F {}
    impl<E> Sealed<E> for EventListenerOrEventListenerObject {}
    impl<E> Sealed<E> for &'static EventListenerObject {}
}

/// What an event handler property's setter, \`set_onclick\`, takes, each member
/// as it is (ADR 0229), borrowed, \`Some(&value)\`, or \`None\`, \`null\` (ADR 1021,
/// amended): a Rust function or closure of the event, the function its getter
/// gave, or a JS function of any value, as a promise's \`reject\` is. Being a
/// union, it gives rustc no closure's signature, so a closure's parameter says
/// its event, \`|e: &PointerEvent| ..\`. JS keeps what it's given where Rust
/// lent it, so a closure is \`Copy\`, which owns nothing to drop: one that
/// owns what isn't is \`listener(..)\`'s, which JS owns.
#[diagnostic::on_unimplemented(message = "\`{Self}\` is not an event handler of a \`{E}\`", note = "an event handler is a function or closure of the event, \`Copy\`, \`listener(..)\`'s, or a JS function value")]
#[cfg_attr(rust_js, rust_js::types = "(event: E) => any")]
pub trait IntoEventHandler<E>: handlers::Event<E> {}
impl<E: 'static, F: Fn(&'static E) + Copy + 'static> IntoEventHandler<E> for F {}
impl<E> IntoEventHandler<E> for &'static dyn Fn(Option<&Unknown>) {}

/// What \`set_onbeforeunload\` takes, as \`IntoEventHandler\` is: a function of the
/// event that gives the text that asks whether to leave, or none, or a JS
/// function of any value.
#[diagnostic::on_unimplemented(message = "\`{Self}\` is not an \`onbeforeunload\` handler of a \`{E}\`", note = "an \`onbeforeunload\` handler is a function or closure of the event that gives an \`Option<String>\`, \`Copy\`, or a JS function value")]
#[cfg_attr(rust_js, rust_js::types = "OnBeforeUnloadEventHandlerNonNull")]
pub trait IntoOnBeforeUnloadEventHandler<E>: handlers::BeforeUnload<E> {}
impl<E: 'static, F: Fn(&'static E) -> Option<String> + Copy + 'static> IntoOnBeforeUnloadEventHandler<E> for F {}
impl<E> IntoOnBeforeUnloadEventHandler<E> for &'static dyn Fn(Option<&Unknown>) {}

/// What a window's \`set_onerror\` takes, as \`IntoEventHandler\` is: a function
/// of the error's message or event, its source, line and column, and what was
/// thrown, that gives whether it's handled, \`true\` not reported; the function
/// the getter gave; or a JS function of any value.
#[diagnostic::on_unimplemented(message = "\`{Self}\` is not a window's \`onerror\` handler", note = "a window's \`onerror\` is a function or closure of \`(EventOrStr, Option<&str>, Option<f64>, Option<f64>, Option<&Unknown>)\` that gives a \`bool\`, \`Copy\`, the function \`onerror()\` gave, or a JS function value")]
#[cfg_attr(rust_js, rust_js::types = "OnErrorEventHandlerNonNull")]
pub trait IntoOnErrorEventHandler: handlers::Error {}
impl<F: ${ERROR_HANDLER} -> bool + Copy + 'static> IntoOnErrorEventHandler for F {}
impl IntoOnErrorEventHandler for &'static dyn ${ERROR_HANDLER} -> Option<&'static Unknown> {}
impl IntoOnErrorEventHandler for &'static dyn Fn(Option<&Unknown>) {}

mod handlers {
    use super::*;

    pub trait Event<E> {}
    impl<E: 'static, F: Fn(&'static E) + Copy + 'static> Event<E> for F {}
    impl<E> Event<E> for &'static dyn Fn(Option<&Unknown>) {}

    pub trait BeforeUnload<E> {}
    impl<E: 'static, F: Fn(&'static E) -> Option<String> + Copy + 'static> BeforeUnload<E> for F {}
    impl<E> BeforeUnload<E> for &'static dyn Fn(Option<&Unknown>) {}

    pub trait Error {}
    impl<F: ${ERROR_HANDLER} -> bool + Copy + 'static> Error for F {}
    impl Error for &'static dyn ${ERROR_HANDLER} -> Option<&'static Unknown> {}
    impl Error for &'static dyn Fn(Option<&Unknown>) {}
}`);

// Each callback of a result JS resolves (\`promisedCallback\`): what it takes,
// a function or closure of any result, sealed by a trait of its own.
for (const [name, { lent, ts }] of [...promised].sort(([a], [b]) => a.localeCompare(b))) {
  line();
  line(`/// What a \`${ts}\` takes: a function or closure that gives what JS resolves, as a`);
  line(`/// promise's \`then\` does, a promise or any value, \`()\` \`undefined\` (ADR 1026). ${lent ? "Lent," : "Moved,"}`);
  line(lent ? `/// \`Some(&update)\`, it's \`Copy\`, which owns nothing to drop while JS holds it (ADR 1021).` : `/// \`Box::new(..)\`, it's JS's, which may own what it holds.`);
  line(`#[diagnostic::on_unimplemented(message = "\`{Self}\` is not a \`${ts}\`", note = "a \`${ts}\` is a function or closure of what JS gives it${lent ? ", \`Copy\`," : ""} that gives any value, a promise or not")]`);
  line(`#[cfg_attr(rust_js, rust_js::types = "${ts}")]`);
  line(`pub trait Into${name}: promised::Into${name} {}`);
  line(`impl<R: 'static, F: ${promisedBound(name)}> Into${name} for F {}`);
}
// The dictionaries results use, as plain structs: JS objects (ADR 0020).
for (const [name, fields] of usedDictionaries) {
  line();
  line(`/// The \`${name}\` dictionary: a JS object with these fields, a \`None\` one not there.`);
  if (fields.every((f) => f.optional)) line(`#[derive(Default)]`);
  line(`pub struct ${typeName(name)} {`);
  for (const f of fields) {
    if (needsJsName(f.rust, f.js)) line(`    #[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(f.js)})]`);
    line(`    pub ${f.rust}: ${f.optional ? `Option<${f.type}>` : f.type},`);
  }
  line(`}`);
}

/**
 * Whether a value of the Rust type `rust` a dictionary or a union a
 * function takes holds is `Copy`: a number, a `bool`, a borrow, or an
 * `Option`, a dictionary or a union of them, so the program keeps what it
 * gave and gives it again, React's FragmentInstance's listener options
 * (rd208). Not a callback JS is given, a `Box`.
 */
const copyable = (rust: string, seen = new Set<string>()): boolean => {
  const option = rust.match(/^Option<(.*)>$/);
  if (option) return copyable(option[1], seen);
  if (rust === "bool" || /^[fiu](8|16|32|64)$/.test(rust) || rust.startsWith("&")) return true;
  const named = rust.replace(/<.*$/, "");
  if (seen.has(named)) return true;
  seen.add(named);
  const dictionary = [...paramDictionaries].find(([n]) => typeName(n) === named);
  if (dictionary) return dictionary[1].fields.every((f) => copyable(f.type, seen));
  const union = unions.get(named);
  if (union) return union.variants.every((v) => copyable(v.type, seen));
  return false;
};

/** A dictionary's or a union's derives: `Clone` and `Copy` of one that's `copyable`, and `Default` of a dictionary of optional fields. */
const derives = (name: string, optional: boolean) => {
  const each = [...(copyable(name) ? ["Clone", "Copy"] : []), ...(optional ? ["Default"] : [])];
  if (each.length) line(`#[derive(${each.join(", ")})]`);
};

// The dictionaries functions take (ADR 0102): records of optional fields.
for (const [name, { fields, borrows }] of [...paramDictionaries].sort(([a], [b]) => a.localeCompare(b))) {
  line();
  line(`/// The [\`${name}\`](https://developer.mozilla.org/docs/Web/API/${name}) dictionary: a JS object of these fields, a \`None\` one not given.`);
  derives(typeName(name), fields.every((f) => f.optional));
  line(`pub struct ${typeName(name)}${borrows ? "<'a>" : ""} {`);
  for (const f of fields) {
    if (needsJsName(f.rust, f.js)) line(`    #[cfg_attr(rust_js, rust_js::name = ${JSON.stringify(f.js)})]`);
    line(`    pub ${f.rust}: ${f.optional ? `Option<${f.type}>` : f.type},`);
  }
  line(`}`);
  // A JS object, never nullish, so a value of any shape too (ADR 1023).
  line();
  line(`unsafe impl Defined for ${typeName(name)}${borrows ? "<'_>" : ""} {}`);
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
const promisedUnions = new Map<string, { generics: string; type: string }[]>();
/** Whether a union holds a callback of a result JS resolves, sealed by a trait of its own. */
const promisedUnionsOf = (union: Union) => union.variants.some((v) => promisedOf(v.type) !== undefined);
for (const union of [...unions.values()].sort((a, b) => a.name.localeCompare(b.name))) {
  const lifetime = union.borrows ? "<'a>" : "";
  const elided = (type: string) => type.replace(/&'a /g, "&").replace(/'a\b/g, "'_");
  const members: string[] = [];
  line();
  line(`/// \`${union.ts}\`: each variant's value is the member itself (ADR 0215).`);
  line(`#[cfg_attr(rust_js, rust_js::untagged)]`);
  derives(union.name, false);
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
    // A dictionary that extends a member is one, the object itself, as
    // TypeScript's is: an `AddEventListenerOptions` an
    // `EventListenerOptions | boolean` (ADR 1023). The parameter takes it;
    // the enum, of the member's own fields, doesn't.
    const dictionary = [...paramDictionaries.keys()].find((n) => typeName(n) === v.name);
    if (dictionary) {
      for (const [n, { borrows }] of paramDictionaries) {
        if (n !== dictionary && dictionaryAncestors(n).includes(dictionary)) members.push(`${typeName(n)}${borrows ? "<'_>" : ""}`);
      }
    }
  }
  const ts = union.ts;
  const own = `${union.name}${union.borrows ? "<'_>" : ""}`;
  line();
  line(`/// What a \`${ts}\` parameter takes: each member as it is, and the enum (ADR 0229);`);
  line(`/// an optional one, \`Option<&dyn Into${union.name}>\`, each borrowed, \`Some(&value)\` (ADR 0443).`);
  line(`#[diagnostic::on_unimplemented(message = "\`{Self}\` is not a \`${ts}\`", note = "an optional one, \`Option<&dyn Into${union.name}>\`, takes \`Some(&value)\`")]`);
  line(`#[cfg_attr(rust_js, rust_js::types = "${ts}")]`);
  // A union of a callback of a result JS resolves takes the function
  // itself, of any result, `Some(&|| ..)`: sealed by a trait of its own,
  // as no shared one may take every function beside each other member.
  const callback = union.variants.map((v) => promisedOf(v.type)).find((c) => c !== undefined);
  if (callback) {
    const impls = [{ generics: `<R: 'static, F: ${promisedBound(callback)}>`, type: "F" }];
    for (const member of [...members, own]) if (!promisedOf(member)) impls.push({ generics: "", type: member });
    promisedUnions.set(union.name, impls);
    line(`pub trait Into${union.name}: promised::Into${union.name} {}`);
    for (const { generics, type } of impls) line(`impl${generics} Into${union.name} for ${type} {}`);
  } else {
    line(`pub trait Into${union.name}: sealed::Sealed {}`);
    for (const member of [...members, own]) {
      line(`impl Into${union.name} for ${member} {}`);
      sealed.add(member);
    }
    // A union whose each member is one of this one's is one too, as
    // TypeScript's is: an `AddEventListenerOptions | boolean` an
    // `EventListenerOptions | boolean` (rd208), the value itself.
    const elidedMembers = new Set(members);
    for (const other of unions.values()) {
      if (other.name === union.name || promisedUnionsOf(other)) continue;
      const each = other.variants.map((v) => elided(v.type));
      if (each.length > 0 && each.every((m) => elidedMembers.has(m))) {
        const theirs = `${other.name}${other.borrows ? "<'_>" : ""}`;
        line(`impl Into${union.name} for ${theirs} {}`);
        sealed.add(theirs);
      }
    }
  }
  line(`/// A rest parameter of \`${ts}\`s takes each member (ADR 0640).`);
  line(`impl<A: Into${union.name}> Takes<A> for dyn Into${union.name} {}`);
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
// What a rest parameter of an interface takes: a reference to it, or to one
// that extends it (ADR 0640).
for (const named of [...restInterfaces].sort()) {
  line();
  line(`/// A rest parameter of \`${jsName(interfaces.get(named)!)}\`s takes each, and each that extends it (ADR 0640).`);
  for (const n of INTERFACES.filter((n) => n === named || ancestors(n).includes(named))) {
    line(`impl<'a> Takes<&'a ${typeName(n)}> for ${typeName(named)} {}`);
  }
}
line();
line(`mod promised {`);
line(`    use super::*;`);
for (const [name] of [...promised].sort(([a], [b]) => a.localeCompare(b))) {
  line();
  line(`    pub trait Into${name} {}`);
  line(`    impl<R: 'static, F: ${promisedBound(name)}> Into${name} for F {}`);
}
for (const [union, impls] of [...promisedUnions].sort(([a], [b]) => a.localeCompare(b))) {
  line();
  line(`    pub trait Into${union} {}`);
  for (const { generics, type } of impls) line(`    impl${generics} Into${union} for ${type} {}`);
}
line(`}`);
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
line(`/// What a window's timer took, which nothing is: they're gone, a timer is the`);
line(`/// js crate's, every JS runtime's, which takes a function as it is (ADR 1020).`);
line(`#[diagnostic::on_unimplemented(`);
line(`    message = "a window's timers are gone: a timer is the js crate's global, \`js::set_timeout(handler, ms)\`, \`js::clear_timeout(id)\`, \`js::set_interval(callback, ms)\` and \`js::clear_interval(id)\`, which take a function as it is (ADR 1020)",`);
line(`    label = "write the js crate's timer, \`js::set_timeout\`, \`js::clear_timeout\`, \`js::set_interval\` or \`js::clear_interval\`"`);
line(`)]`);
line(`pub trait TimerGone<Of: ?Sized = ()> {}`);
line();
line(`/// The handler of a window's \`setTimeout\` read as a value,`);
line(`/// \`js::method((window, Window::set_timeout))\`'s, a function (ADR 1020, amended, rd314).`);
line(`impl TimerGone<fn()> for Box<dyn FnOnce()> {}`);
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
  // And each mixin's union one of its interfaces is, by itself or an ancestor.
  for (const [union, includers] of Object.entries(UNIONS)) {
    if (name !== union && [name, ...chain(name)].some((a) => includers.includes(a))) line(`unsafe impl IsA<${typeName(union)}> for ${typeName(name)} {}`);
  }
}

// What WebIDL marks `[Serializable]`, a `Blob` say, the browser's structured
// clone copies (ADR 0225); and an object is never nullish.
line();
for (const name of INTERFACES) {
  const serializable = read.some((d) => d.type === "interface" && d.name === name && (d.extAttrs ?? []).some((a) => a.name === "Serializable"));
  if (serializable) line(`unsafe impl StructuredClone for ${typeName(name)} {}`);
  line(`unsafe impl Defined for ${typeName(name)} {}`);
}

// Each member `THROWS` names is one this crate binds: a name gone from
// WebIDL, or written wrong, is no `Result` that quietly isn't.
const unthrown = Object.keys(THROWS).filter((key) => !thrown.has(key));
if (unthrown.length > 0) throw new Error(`THROWS names no member this crate binds: ${unthrown.join(", ")}`);
await Bun.write(process.argv[2] ?? new URL("./src/lib.rs", import.meta.url), `${out.join("\n")}\n`);
const reasons = [...skipped].sort((a, b) => b[1] - a[1]).map(([why, n]) => `${why} ${n}`);
console.log(`src/lib.rs: ${INTERFACES.length} interfaces, ${NAMESPACES.length} namespaces, ${count} functions, ${unions.size} unions`);
console.log(`events: ${eventNames.length} names, ${impls} on ${listens.size} targets (${disagreeing} where specs disagree, as \`Event\`); tags: ${tagNames.length}`);
console.log(`skipped: ${(process.env.ALL ? reasons : reasons.slice(0, 12)).join(", ")}${reasons.length > 12 && !process.env.ALL ? ", ..." : ""}`);
