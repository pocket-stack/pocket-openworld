import { Show, For } from "solid-js";
import { getOps } from "@pocketjs/framework/host";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { Paragraph } from "./text";

interface Target {name:string;temperature:number;wet:number;fuel:number;burning:boolean;cook:number;char:number;}
interface Sample {label:string;wet:number;temperature:number;status:string;history:string;}
export type HudState =
  | {kind:"orchard";water:{spraying:boolean;progress:number};target:Target|null;message:string}
  | {kind:"lab";title:string;instruction:string;samples:Sample[];footer:string;labels:{label:string;x:number;y:number}[]};

const panel="absolute flex-col p-[12] gap-[6] rounded-[5] bg-[#091c18] opacity-90";
const text="text-xs text-[#ddead5]";
const muted="text-xs text-[#b0c9b9]";
const title="text-sm font-bold text-[#eddaa4]";
const value="text-xs text-[#b3e5ef]";

// These Views are display-only. Pointer ownership remains with the game until
// Tab opens the controls; HUD and window share the same guest, fonts and scale.
export function GameHud(props:{model:HudState; width:number; height:number; register:(id:string)=>(node:NodeMirror)=>void}) {
  const orchard=()=>props.model.kind==="orchard"?props.model:null;
  const lab=()=>props.model.kind==="lab"?props.model:null;
  const messageWidth=()=>Math.min(props.width-64,Math.max(200,getOps().measureText(orchard()?.message??"",0)+32));
  return <View ref={props.register("game-hud")} class="absolute left-0 top-0 w-full h-full">
    <Show when={orchard()}>{model => <>
      <View ref={props.register("hud-orchard")} class={panel} style={{insetL:20,insetT:20,width:366}}>
        <Text class={title}>Reactive Orchard</Text>
        <Text class={text}>Tab: open controls     WASD: move</Text>
        <Paragraph cls={text} width={342} text="Click experiments and actions in the control window" />
      </View>
      <View ref={props.register("hud-water")} class={panel} style={{insetL:20,insetT:117,width:224}}>
        <Text class={value}>{model().water.spraying?"Water spraying":"Water ready"}</Text>
        <View class="h-[7] bg-[#123541]"><View class="h-full bg-[#31a4cf]" style={{width:200*Math.max(0,Math.min(1,model().water.progress))}} /></View>
      </View>
      <Show when={model().target}>{target =>
        <View ref={props.register("hud-target")} class={panel} style={{insetR:20,insetT:20,width:274}}>
          <Paragraph cls={title} slot={8} width={250} text={target().name} />
          <Text class={text}>{`Temp ${target().temperature.toFixed(1)} C   Moist ${Math.round(target().wet)}%`}</Text>
          <Text class={text}>{`Fuel ${Math.round(target().fuel)}%   Burn ${target().burning?"yes":"no"}`}</Text>
          <Text class={muted}>{`Cook ${Math.round(target().cook)}%   Char ${Math.round(target().char)}%`}</Text>
        </View>
      }</Show>
      <Show when={model().message}>
        <View ref={props.register("hud-message")} class={panel} style={{insetL:(props.width-messageWidth())/2,insetB:36,width:messageWidth()}}>
          <Paragraph cls={text} width={messageWidth()-24} text={model().message} />
        </View>
      </Show>
    </>}</Show>
    <Show when={lab()}>{model => <>
      <View ref={props.register("hud-lab")} class={panel} style={{insetL:16,insetT:16,width:props.width-32}}>
        <Text class={title}>{model().title}</Text>
        <Text class={text}>Tab: open the experiment and action controls</Text>
        <Paragraph cls={muted} width={props.width-56} text={model().instruction} />
      </View>
      <For each={model().labels}>{label =>
        <View class="absolute w-[56] items-center p-[3] rounded-[3] bg-[#17352d] opacity-90" style={{insetL:label.x-28,insetT:label.y}}>
          <Text class={text}>{label.label}</Text>
        </View>
      }</For>
      <View ref={props.register("hud-samples")} class={panel} style={{insetL:16,insetB:16,width:props.width-32}}>
        <View class="flex-row gap-[20]">
          <For each={model().samples}>{sample => <View class="flex-1 flex-col gap-[6]">
            <Text class={title}>{sample.label}</Text>
            <Text class={value}>{`Wet ${Math.round(sample.wet)}%    Temp ${Math.round(sample.temperature)} C`}</Text>
            <Text class={text}>{sample.status}</Text>
            <Text class={muted}>{sample.history}</Text>
          </View>}</For>
        </View>
        <Paragraph cls={muted} width={props.width-56} text={model().footer} />
      </View>
    </>}</Show>
  </View>;
}
