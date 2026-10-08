<script lang="ts">
  let { name, size = 20 }: { name: string; size?: number } = $props();
  const paths: Record<string, string> = {
    play: 'M8 5v14l11-7z', pause: 'M8 5v14M16 5v14', folder: 'M3 8V5h6l2 3h10v11H3z',
    loop: 'M4 8h13l-3-3m3 3-3 3M20 16H7l3 3m-3-3 3-3', plus: 'M12 5v14M5 12h14',
    minus: 'M5 12h14', reset: 'M5 8a8 8 0 1 1-1 8M5 3v5h5',
    back: 'M16 5 7 12l9 7M5 5v14', next: 'M8 5l9 7-9 7M19 5v14',
    trash: 'M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 10v7M14 10v7',
    edit: 'm15 4 5 5-10 10-6 1 1-6zM13 6l5 5', close: 'M6 6l12 12M18 6 6 18',
    help: 'M9 8a3 3 0 1 1 5 2c-2 1-2 2-2 3M12 17h.01M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0',
    fit: 'M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5', check: 'm5 12 4 4 10-10'
  };
</script>
<svg width={size} height={size} viewBox="0 0 24 24" fill={name === 'play' ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={paths[name] ?? paths.plus}/></svg>
