<script lang="ts">
  import type {MeasureMap,MapEdit} from './timing';
  import type {Keymap} from './keymap';
  import type {ButtonId} from './actions';
  let {songKey,map,keymap,run,available,onedit}:
    {songKey:string;map:MeasureMap|null;keymap:Keymap;run:(id:ButtonId)=>void;available:(id:ButtonId)=>boolean;onedit:(edit:MapEdit)=>Promise<void>}=$props();
  let origin=$state(1),identity='';
  $effect(()=>{const next=JSON.stringify([songKey,map?.revision]);if(next!==identity){identity=next;origin=map?.first_number??1;}});
  export async function command(){await onedit({type:'set_origin',first_number:origin});}
</script>
<details open class="batch-tools"><summary>Batch tools</summary>
  <label>First full measure number<input type="number" min="1" max="1000000" step="1" bind:value={origin} disabled={!map?.measures.length}/></label>
  <button class="text-button" disabled={!available('map_origin')} title={keymap.title('map_origin')} onclick={()=>run('map_origin')}>{keymap.buttonLabel('map_origin','Preview numbering')}</button>
  <p>TODO: Apply beat grouping to all measures with a matching time signature.</p>
</details>
<style>
  .batch-tools{padding:9px 0 17px;border-bottom:1px solid #e0e6f0}summary{font-size:13px;font-weight:600;cursor:pointer;color:#3d506f}label{display:flex;flex-direction:column;gap:5px;font-size:11px;margin-top:11px;color:#596b86}input{min-width:0;width:100%;padding:7px 8px;border:1px solid #d3ddeb;border-radius:4px;background:#fff;color:#334966;font:inherit;font-size:12px}.text-button{font-size:11px;padding:5px 0;margin-top:4px}p{color:#7a879d;line-height:1.5;font-size:11px;margin:9px 0}
</style>
