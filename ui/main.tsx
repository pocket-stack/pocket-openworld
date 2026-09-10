import { createSignal, onCleanup, Show, For } from "solid-js";
import { mount } from "@pocketjs/framework/solid";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { connectOverlay } from "@pocketjs/framework/overlay-host";
import { XP_THEME as theme } from "@pocketjs/framework/themes/desktop";
import { Paragraph as WrappedText } from "./text";
import { GameHud, type HudState } from "./hud";

type Trial = "orchard" | "rain" | "screen" | "firebreak";
interface Sample { wet: number; temperature: number; status: string; }
interface State {
  open: boolean; hud: HudState | null;
  trial: Trial; rain: boolean; screenOpen: boolean; paused: boolean;
  running: boolean; progress: number; elapsed: number;
  activity: string; outcome: string; result: string;
  left: Sample; right: Sample; width: number; height: number;
}
type Command = { action: string; value?: string };
const initial: State = {
  open:false, hud:null,
  trial:"orchard", rain:false, screenOpen:false, paused:false, running:false,
  progress:0, elapsed:0, activity:"Choose an experiment to begin.", outcome:"idle", result:"",
  left:{wet:0,temperature:24,status:""}, right:{wet:0,temperature:24,status:""}, width:960,height:600,
};
const tabs: {id:Trial; label:string}[] = [
  {id:"rain",label:"Rain"}, {id:"screen",label:"Screen"},
  {id:"firebreak",label:"Firebreak"}, {id:"orchard",label:"Orchard"},
];

function App() {
  const [state, setState] = createSignal<State>(initial);
  const [position, setPosition] = createSignal({x:16,y:20});
  const channel = connectOverlay<State, Command>(setState);
  onCleanup(() => channel.dispose());
  const send = (action:string, value?:string) => channel.send({action,value});
  const width = () => Math.min(352, state().width - 24);
  const x = () => Math.max(8, Math.min(position().x, state().width-width()-8));
  const y = () => Math.max(8, Math.min(position().y, state().height-568));
  let dragStart = {x:16,y:20};
  const caption = (node:NodeMirror) => {
    onCleanup(channel.control("window-caption", node));
    const dispose = channel.drag(node, p => {
      if (p.started) dragStart={x:x(),y:y()};
      setPosition({x:dragStart.x+p.dx,y:dragStart.y+p.dy});
    });
    onCleanup(dispose);
  };
  const register = (id:string) => (node:NodeMirror) => { onCleanup(channel.control(id,node)); };
  const control = (id:string, label:()=>string, action:string, value?:string, disabled:()=>boolean=()=>false) => (
    <View ref={register(id)} debugName={id} class={disabled()?theme.disabledButton:theme.button}
      focusable={!disabled()} onPress={() => {if (!disabled()) send(action,value);}}>
      <Text class={theme.buttonText}>{label()}</Text>
    </View>
  );
  function Paragraph(props:{text:string; cls?:string; slot?:number; inset?:number}) {
    return <WrappedText text={props.text} cls={props.cls??theme.text} slot={props.slot} width={width()-(props.inset??34)} />;
  }
  const experiment = () => state().trial !== "orchard";
  const primaryScenario = () => state().trial === "rain" ? "shelter-rain-moved" : state().trial === "screen" ? "shelter-screen" : "shelter-firebreak";
  const alternateLabel = () => state().trial === "rain" ? "Keep roof in place (8s)" : state().trial === "screen" ? "Open screen & spray (42s)" : "Run without water (30s)";
  const primaryLabel = () => state().running ? "Restart comparison" : state().trial === "rain" ? "Run rain & roof comparison (15s)" : state().trial === "screen" ? "Run heat comparison (20s)" : "Run wet firebreak (30s)";
  const description = () => ({
    rain:"Rain wets exposed wood. Slide the roof to swap shelter.",
    screen:"The same screen intercepts heat and water.",
    firebreak:"Wet the middle strip to protect the grass beyond it.",
    orchard:"Original Frieren, staff actions and reactive orchard.",
  })[state().trial];
  const resultColor = () => state().outcome === "passed" ? theme.success : state().outcome === "failed" ? theme.failure : theme.muted;

  return (
    <Show when={state().open} fallback={<Show when={state().hud}>{hud => <GameHud model={hud()} width={state().width} height={state().height} register={register} />}</Show>}>
    <View ref={register("window")} class={theme.window} style={{insetL:x(),insetT:y(),width:width(),height:560}} debugName="WorldControlWindow">
      <View class={theme.caption} ref={caption}>
        <View class="w-[16] h-[16] rounded-[2] border border-white bg-gradient-to-b from-[#a1dd60] to-[#34822e]" />
        <View class="flex-1"><Text class={theme.title}>Pocket Openworld</Text></View>
        <View ref={register("close")} class={theme.close} focusable onPress={() => send("close")}><Text class={theme.closeText}>X</Text></View>
      </View>
      <View class={theme.body}>
        <View class={theme.tabs}>
          <For each={tabs}>{tab => (
            <View ref={register(`trial-${tab.id}`)} class={theme.tab(state().trial===tab.id)} focusable onPress={() => send("trial",tab.id)}>
              <Text class={theme.tabText(state().trial===tab.id)}>{tab.label}</Text>
            </View>
          )}</For>
        </View>
        <View class="h-[33] flex-col justify-center"><Paragraph text={description()} /></View>
        <Show when={experiment()} fallback={
          <View class={theme.well}><Text class={theme.heading}>Explore with Frieren</Text><Paragraph text="Close this window to walk and aim. Press Tab whenever you need controls." inset={50} /></View>
        }>
          <View class="flex-col gap-[6]">
            <View ref={register("run-primary")} class={theme.primaryButton} focusable onPress={() => send("run",primaryScenario())}>
              <Text class={theme.buttonText}>{primaryLabel()}</Text>
            </View>
            {control("run-alternate", alternateLabel, "run-alternate")}
          </View>
        </Show>
        <View class={theme.group}>
          <Text class={theme.groupTitle}>Try it yourself</Text>
          <View class="flex-row gap-[7]">
            {control("ignite",()=>state().trial==="screen"?"Light heaters":"Ignite", "ignite")}
            {control("spray",()=>"Spray water", "spray")}
          </View>
          <View class="flex-row gap-[7]">
            <Show when={experiment()} fallback={control("chop",()=>"Swing staff","chop")}>
              {control("rain",()=>state().rain?"Stop rain":"Start rain", "rain")}
            </Show>
            <Show when={state().trial!=="orchard"} fallback={control("pickup",()=>"Pick up / drop","pickup")}>
              {control("panel",()=>state().trial==="rain"?(state().screenOpen?"Roof to left":"Roof to right"):(state().screenOpen?"Close screen":"Open screen"),"panel",undefined,()=>state().trial==="firebreak")}
            </Show>
          </View>
        </View>
        <View class={theme.well}>
          <Text class={theme.groupTitle}>Live observations</Text>
          <Show when={experiment()} fallback={<Text class={theme.muted}>WASD to move after closing the window.</Text>}>
            <View class="flex-row gap-[8]">
              <For each={["left","right"] as const}>{side => (
                <View class="flex-1 flex-col gap-[3]">
                  <Text class={theme.text}>{side==="left"?"Left / control":"Right / comparison"}</Text>
                  <Text class={theme.heading}>{`${Math.round(state()[side].wet)}% wet  ${Math.round(state()[side].temperature)} C`}</Text>
                  <Text class={theme.muted}>{state()[side].status}</Text>
                </View>
              )}</For>
            </View>
          </Show>
        </View>
        <View class="flex-col gap-[5] flex-1">
          <Paragraph cls={resultColor()} slot={state().outcome==="passed" || state().outcome==="failed"?7:0} text={state().result || state().activity} />
          <View class={theme.progressTrack}><View class={theme.progressFill} style={{width:Math.max(0,Math.min(1,state().progress))*(width()-28)}} /></View>
          <Text class={theme.muted}>{state().running?state().activity:"Results stay visible. Reset to repeat."}</Text>
        </View>
        <View class="flex-row gap-[7]">
          {control("reset",()=>"Reset","reset")}
          {control("pause",()=>state().paused?"Resume":"Pause","pause")}
          {control("return",()=>"Back to game","close")}
        </View>
      </View>
      <View class={theme.status}><Text class={theme.muted}>{`Tab: controls / game     ${Math.round(state().elapsed)}s   ${state().paused?"Paused":"Simulation running"}`}</Text></View>
    </View>
    </Show>
  );
}
mount(() => <App />);
