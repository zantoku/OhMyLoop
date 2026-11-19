<script lang="ts">
  // The shell's playback bar along the bottom of a song view. Screens fill it
  // with their transport, mode toggles and mixer.
  import type { Snippet } from 'svelte';
  let { children }: { children: Snippet } = $props();
</script>

<section class="playback-bar" aria-label="Playback">{@render children()}</section>

<style>
  .playback-bar{flex:none;display:flex;align-items:center;gap:26px;padding:10px 24px;background:var(--surface);border-top:1px solid var(--border)}
</style>
