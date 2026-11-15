import {beforeAll,expect,test} from "bun:test";
import {readFileSync,writeFileSync} from "node:fs";
import {join} from "node:path";
import {buildReact,compiler,fixture,run,target} from "./support";

beforeAll(buildReact,600_000);

// #1867: the real scrollbar case, reduced without replacing native getters
// by plain struct fields (the latter already inlines correctly).
test("native getter arguments stay in nested literals with evaluation order intact", async () => {
  const dir = fixture("native-getter-aggregate-arguments");
  writeFileSync(join(dir,"lib.rs"), `use webapi::{Element, PointerEvent, window_global};
pub struct Point { pub x:f64, pub y:f64 }
pub struct Edges { pub top:f64, pub left:f64 }
pub struct Bounds { pub width:f64, pub borders:Edges }
pub fn consume(point:&Point,bounds:&Bounds)->f64 {
    point.x + point.y + bounds.width + bounds.borders.top + bounds.borders.left
}
pub fn nested(event:&PointerEvent,target:&Element)->f64 {
    let rect=target.get_bounding_client_rect();
    let style=window_global::get_computed_style(target,None);
    consume(&Point { x:event.client_x()-rect.left(), y:event.client_y()-rect.top() }, &Bounds {
        width:rect.width(),
        borders:Edges {
            top:js::or(js::parse_float(style.border_top_width()),0.0),
            left:js::or(js::parse_float(style.border_left_width()),0.0),
        },
    })
}
pub fn consume_array(point:&[f64;2],bounds:&Bounds)->f64 {
    point[0] + point[1] + bounds.width + bounds.borders.top + bounds.borders.left
}
pub fn nested_array(event:&PointerEvent,target:&Element)->f64 {
    let rect=target.get_bounding_client_rect();
    let style=window_global::get_computed_style(target,None);
    consume_array(&[event.client_x()-rect.left(),event.client_y()-rect.top()], &Bounds {
        width:rect.width(),
        borders:Edges {
            top:js::or(js::parse_float(style.border_top_width()),0.0),
            left:js::or(js::parse_float(style.border_left_width()),0.0),
        },
    })
}
pub struct DocArg { pub document:&'static webapi::Document }
pub fn take_doc(_document:&DocArg,value:f64)->f64 { value }
pub fn global_before(event:&PointerEvent)->f64 {
    take_doc(&DocArg { document:window_global::document() }, event.client_x())
}
`);
  run([compiler,join(dir,"lib.rs"),"-o",join(dir,"lib.js"),"--","--extern","js="+join(target,"libjs.rmeta"),"--extern","webapi="+join(target,"libwebapi.rmeta"),"-L",target]);
  const lib = await import(join(dir,"lib.js"));
  const original=Object.getOwnPropertyDescriptor(globalThis,"getComputedStyle");
  const names=["rect()","style()","clientX","left","clientY","top","width","borderTopWidth","borderLeftWidth"];
  try {
    for(const name of ["nested","nestedArray"]) for(const stop of [undefined,...names]) {
      const calls:string[]=[];const marker={};
      const read=(name:string,value:unknown)=>{calls.push(name);if(stop===name)throw marker;return value;};
      const fields=(values:Record<string,unknown>)=>Object.defineProperties({},Object.fromEntries(Object.entries(values).map(([name,value])=>[name,{get(){return read(name,value);}}])));
      const rect=fields({left:1,top:2,width:100});
      const style=fields({borderTopWidth:"3px",borderLeftWidth:"4px"});
      const target={getBoundingClientRect(){return read("rect()",rect);}};
      Object.defineProperty(globalThis,"getComputedStyle",{configurable:true,value(){return read("style()",style);}});
      const event=fields({clientX:11,clientY:22});
      let value:number|undefined;let thrown=false;
      try{value=lib[name](event,target);}catch(error){thrown=error===marker;}
      expect({value,thrown,calls}).toEqual(stop===undefined?{value:137,thrown:false,calls:names}:{value:undefined,thrown:true,calls:names.slice(0,names.indexOf(stop)+1)});
    }
  } finally {
    if(original)Object.defineProperty(globalThis,"getComputedStyle",original);
    else Reflect.deleteProperty(globalThis,"getComputedStyle");
  }
  // A bare global may throw even though it is represented as a variable.
  // Earlier literal arguments retain that read before a later host getter.
  const documentDescriptor=Object.getOwnPropertyDescriptor(globalThis,"document");
  try {
    const calls:string[]=[];
    const event=Object.defineProperty({},"clientX",{get(){calls.push("clientX");return 4;}});
    Reflect.deleteProperty(globalThis,"document");
    expect(()=>lib.globalBefore(event)).toThrow(ReferenceError);
    expect(calls).toEqual([]);
    Object.defineProperty(globalThis,"document",{configurable:true,get(){calls.push("document");return {};}});
    expect(lib.globalBefore(event)).toBe(4);
    expect(calls).toEqual(["document","clientX"]);
    calls.length=0;const marker={};
    Object.defineProperty(globalThis,"document",{configurable:true,get(){calls.push("document");throw marker;}});
    let same=false;
    try{lib.globalBefore(event);}catch(error){same=error===marker;}
    expect(same).toBe(true);
    expect(calls).toEqual(["document"]);
  } finally {
    if(documentDescriptor)Object.defineProperty(globalThis,"document",documentDescriptor);
    else Reflect.deleteProperty(globalThis,"document");
  }
  const output=readFileSync(join(dir,"lib.js"),"utf8");
  for(const name of ["nested","nestedArray"]) {
    const marker="export function "+name+"(";
    const start=output.indexOf(marker);
    expect(start).toBeGreaterThanOrEqual(0);
    const next=output.indexOf("\nexport function ",start+marker.length);
    const body=output.slice(start,next<0?undefined:next);
    expect(body.split("\n").filter(line=>line.trimStart().startsWith("const "))).toEqual([
      "  const rect = target.getBoundingClientRect();",
      "  const style = getComputedStyle(target);",
    ]);
    expect(body).toContain("event.clientX - rect.left");
    expect(body).toContain("top: parseFloat(style.borderTopWidth) || 0");
  }
});
