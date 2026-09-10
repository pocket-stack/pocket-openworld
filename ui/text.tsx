import { createMemo, For } from "solid-js";
import { getOps } from "@pocketjs/framework/host";
import { Text, View } from "@pocketjs/framework/components";

// The native font provider measures the same baked Inter slot that Text paints.
export function Paragraph(props:{text:string; width:number; cls:string; slot?:number}) {
  const lines=createMemo(() => {
    const text=props.text;
    const ends=[...(getOps().wrapText?.(text,props.slot??0,Math.max(1,props.width))??[]),text.length];
    let start=0;
    return ends.map(end=>{const part=text.slice(start,end).trim();start=end;return part;});
  });
  return <View class="flex-col gap-[2]"><For each={lines()}>{line=><Text class={props.cls}>{line}</Text>}</For></View>;
}
