<script lang="ts">
  // THROWAWAY: three transport-strip layouts; ?mixer-prototype&variant=A|B|C.
  import Icon from './Icon.svelte';
  const names = ['Bass', 'Drums', 'Guitar', 'Piano', 'Vocals', 'Other'];
  const variants = ['A', 'B', 'C'];
  const titles = ['Channel strips', 'Paired tracks', 'Selected-track controls'];
  let variant = $state(new URLSearchParams(location.search).get('variant') || 'A');
  let tracks = $state(names.map(name => ({ name, gain: 100, mute: false, solo: false })));
  const source = $derived(tracks.some(track => track.mute || track.solo || track.gain !== 100) ? 'Stems' : 'Original');
  let playing = $state(false), count = $state(false), loop = $state(0);
  let selected = $state(0), speed = $state(100), pitch = $state(0);
  let drag: { index: number; y: number; value: number } | null = null;
  function changeVariant(delta: number) {
    variant = variants[(variants.indexOf(variant) + delta + 3) % 3];
    const url = new URL(location.href); url.searchParams.set('variant', variant); history.replaceState(null, '', url);
  }
  function gain(index: number, value: number) { tracks[index].gain = Math.max(0, Math.min(100, Math.round(value))); }
  function knobKey(event: KeyboardEvent, index: number) {
    if (['ArrowUp','ArrowRight','ArrowDown','ArrowLeft','Home','End'].includes(event.key)) {
      event.preventDefault(); event.stopPropagation();
      gain(index, event.key === 'Home' ? 0 : event.key === 'End' ? 100 : tracks[index].gain + (['ArrowUp','ArrowRight'].includes(event.key) ? 1 : -1));
    }
  }
  function loopNext() { loop = (loop + 1) % 3; if (loop === 2) count = true; }
</script>
{#snippet hourglass()}
  <svg viewBox="0 0 24 24" width="19" height="19" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><path d="M6 3h12M6 21h12M7 3v4l10 10v4M17 3v4L7 17v4"/></svg>
{/snippet}
<svelte:window onkeydown={e => { const target = e.target as HTMLElement; if (target.closest('input,textarea,select,[contenteditable], [role="slider"]')) return; if(e.key==='ArrowLeft'||e.key==='ArrowRight'){e.preventDefault();changeVariant(e.key==='ArrowLeft'?-1:1);} }} onpointermove={e=>{if(drag)gain(drag.index,drag.value+(drag.y-e.clientY));}} onpointerup={()=>{drag=null;}} onpointercancel={()=>{drag=null;}}/>
<div class="prototype-strip variant-{variant}">
  <div class="transport">
    <div class="playback">
      <button class="icon-button" title="Previous section"><Icon name="back" size={17}/></button>
      <div class="play-position">
        <button class="play-button" aria-label={playing?'Pause':'Play'} onclick={()=>playing=!playing}><Icon name={playing?'pause':'play'} size={25}/></button>
        <div class="clock"><strong>1:24</strong><span> / 4:56</span></div>
      </div>
      <button class="icon-button" title="Next section"><Icon name="next" size={17}/></button>
    </div>
    <div class="modes">
      <button class="mode" class:active={count} aria-label="Count-in" aria-pressed={count} title={count?'Count-in on':'Count-in off'} onclick={()=>{count=!count;if(!count&&loop===2)loop=1;}}>{@render hourglass()}</button>
      <button class="mode loop" aria-label={['Loop off','Loop on','Loop with count-in'][loop]} title={['Loop off → Loop','Loop → Loop with count-in','Loop with count-in → Off'][loop]} onclick={loopNext}><span class="mode-icon" class:lit={loop>0}><Icon name="loop" size={19}/></span><span class="mode-icon" class:lit={loop===2}>{@render hourglass()}</span></button>
    </div>
  </div>
  <div class="mix">
    <div class="tracks">
      {#each tracks as track, i}
        <div class="track" class:muted={track.mute} class:soloed={track.solo} class:chosen={variant==='C'&&selected===i}>
          <button class="track-name" onclick={()=>selected=i}>{track.name}</button>
          <div class="knob" role="slider" tabindex="0" aria-label={`${track.name} gain`} aria-valuemin={0} aria-valuemax={100} aria-valuenow={track.gain} aria-valuetext={`${track.gain}%`} title={`${track.name} ${track.gain}% · drag up/down, scroll, or use arrow keys`}
            style:--angle={`${-135+track.gain*2.7}deg`} style:--sweep={`${track.gain*2.7}deg`}
            onpointerdown={e=>{e.preventDefault();selected=i;e.currentTarget.focus();e.currentTarget.setPointerCapture(e.pointerId);drag={index:i,y:e.clientY,value:track.gain};}}
            onkeydown={e=>{knobKey(e,i);}}
            onwheel={e=>{e.preventDefault();gain(i,track.gain+(e.deltaY<0?2:-2));}}
            ondblclick={()=>{gain(i,100);}}>
            <div class="dial"><span></span></div>
          </div>
          <span class="value">{track.gain}%</span>
          {#if variant!=='C'}<div class="toggles"><button class:enabled={track.mute} aria-label={`Mute ${track.name}`} aria-pressed={track.mute} onclick={()=>track.mute=!track.mute}>M</button><button class:enabled={track.solo} aria-label={`Solo ${track.name}`} aria-pressed={track.solo} onclick={()=>track.solo=!track.solo}>S</button></div>{/if}
        </div>
      {/each}
    </div>
    {#if variant==='C'}<div class="inspector"><strong>{tracks[selected].name}</strong><div class="toggles"><button class:enabled={tracks[selected].mute} onclick={()=>tracks[selected].mute=!tracks[selected].mute}>Mute</button><button class:enabled={tracks[selected].solo} onclick={()=>tracks[selected].solo=!tracks[selected].solo}>Solo</button></div><small>{tracks[selected].gain}%</small></div>{/if}
  </div>
</div>
<div class="compact-settings"><label>Speed <strong>{speed}%</strong><input aria-label="Prototype speed" type="range" min="25" max="200" bind:value={speed}/><button onclick={()=>speed=100}>Reset</button></label><label>Pitch <strong>{pitch>0?'+':''}{pitch} st</strong><input aria-label="Prototype pitch" type="range" min="-12" max="12" bind:value={pitch}/><button onclick={()=>pitch=0}>Reset</button></label></div>
<details class="state"><summary>Prototype state · {source} · no audio or saving</summary><pre>{JSON.stringify({variant,source,playing,countIn:count,loop:['off','loop','loop-with-count-in'][loop],speed,pitch,tracks},null,2)}</pre></details>
<div class="switcher"><button aria-label="Previous variant" onclick={()=>changeVariant(-1)}>←</button><div><strong>{variant} · {titles[variants.indexOf(variant)]}</strong><small>Throwaway mixer mockup · drag knobs to adjust</small></div><button aria-label="Next variant" onclick={()=>changeVariant(1)}>→</button></div>
<style>
  .prototype-strip{display:flex;align-items:center;gap:16px;border-block:1px solid var(--border);padding:12px 0;flex-shrink:0;flex-wrap:wrap}.transport{display:flex;align-items:center;gap:12px}.playback{display:flex;align-items:center;gap:3px}.modes{display:flex;flex-direction:column;align-items:center;gap:3px}.play-button{width:43px;height:40px}.mode{display:flex;align-items:center;justify-content:center;height:32px;width:58px;border:1px solid var(--border-strong);border-radius:5px;background:transparent;color:var(--muted);padding:5px}.active{background:var(--amber-bg);color:var(--amber-text);border-color:var(--amber-border)}.mode.loop{gap:2px;padding:0}.mode-icon{display:flex;align-items:center;justify-content:center;width:25px;height:26px;border-radius:3px}.mode-icon.lit{background:var(--amber-bg);color:var(--amber-text)}.play-position{display:flex;flex-direction:column;align-items:center;gap:5px}.clock{white-space:nowrap;line-height:1.2}.clock strong{font-size:11px;font-weight:500}.clock span{font-size:10px;color:var(--muted)}.mix{display:flex;align-items:center;gap:12px;flex:1;border-left:1px solid var(--border);padding-left:16px;min-width:0}.tracks{display:flex;justify-content:center;gap:26px;flex:1}.track{display:flex;flex-direction:column;align-items:center;gap:3px;min-width:44px}.track-name{border:0;background:transparent;font-size:10px;padding:0;color:var(--text-secondary)}.knob{width:38px;height:38px;border-radius:50%;padding:4px;background:conic-gradient(from 225deg,var(--blue) 0deg,var(--blue) var(--sweep),var(--border) var(--sweep),var(--border) 270deg,transparent 270deg);cursor:ns-resize;touch-action:none}.dial{position:relative;border-radius:50%;background:var(--surface);height:100%;border:1px solid var(--border-strong);transform:rotate(var(--angle));box-shadow:0 1px 3px #0002}.dial span{position:absolute;top:3px;left:calc(50% - 1px);width:2px;height:8px;background:var(--blue);border-radius:1px}.value{font-size:9px;color:var(--muted)}.toggles{display:flex;gap:3px}.toggles button{font-size:9px;min-width:20px;height:19px;border:1px solid var(--border);border-radius:3px;background:var(--sidebar);color:var(--muted);padding:0 4px}.toggles .enabled{background:var(--amber-bg);border-color:var(--amber-border);color:var(--amber-text)}.muted .knob{opacity:.4}.soloed .track-name{color:var(--amber-text);font-weight:700}.compact-settings{display:flex;gap:24px;padding:12px 0;flex-shrink:0}.compact-settings label{display:flex;align-items:center;gap:8px;font-size:11px;flex:1}.compact-settings strong{min-width:38px;font-weight:500}.compact-settings input{width:80px;flex:1;min-width:40px}.compact-settings button{font-size:10px;background:transparent;border:0;color:var(--link)}.state{font-size:10px;color:var(--muted);max-height:110px;overflow:auto;margin-bottom:55px;flex-shrink:0}.state pre{font-size:10px}.switcher{position:fixed;z-index:100;bottom:16px;left:50%;transform:translateX(-50%);display:flex;align-items:center;gap:18px;background:var(--text);color:var(--surface);padding:9px 12px;border-radius:9px;box-shadow:0 3px 14px #0003;white-space:nowrap}.switcher button{background:transparent;border:0;color:inherit;font-size:20px}.switcher div{display:flex;flex-direction:column;text-align:center}.switcher strong{font-size:12px}.switcher small{font-size:10px;opacity:.7}.variant-B .tracks{display:grid;grid-template-columns:repeat(3,1fr);gap:8px 16px}.variant-B .track{display:grid;grid-template-columns:34px 38px 28px 44px;gap:5px}.variant-B .track-name{text-align:left}.variant-B .knob{width:32px;height:32px}.variant-C .track{min-width:37px}.variant-C .chosen .track-name{background:var(--selected);color:var(--link);border-radius:3px;padding:2px 4px}.variant-C .knob{width:30px;height:30px}.inspector{border-left:1px solid var(--border);padding-left:10px;display:flex;flex-direction:column;gap:5px;min-width:70px}.inspector strong{font-size:10px}.inspector small{font-size:9px;color:var(--muted)}
  @media(max-width:1100px){.mix{flex-basis:100%;border-left:0;border-top:1px solid var(--border);padding:10px 0 0}.transport{width:100%;justify-content:space-between}.tracks{justify-content:center}}@media(max-width:680px){.variant-B .tracks{grid-template-columns:repeat(2,1fr)}.variant-B .track{grid-template-columns:30px 32px}.variant-B .track .value{grid-column:1}.compact-settings{gap:8px;flex-wrap:wrap}.compact-settings label{flex-basis:100%}.mix{gap:6px}.tracks{gap:3px}.track{min-width:32px}.knob{width:31px;height:31px}.variant-C .mix{flex-wrap:wrap}.inspector{border-left:0;flex-direction:row;align-items:center}.switcher{gap:8px}.switcher small{font-size:9px}}
</style>
