<script lang="ts">
  let { name, size = 20 }: { name: string; size?: number } = $props();
  const paths: Record<string, string> = {
    hourglass: 'M7 3h10M7 21h10M8 3c0 5 8 5 8 9s-8 4-8 9M16 3c0 5-8 5-8 9s8 4 8 9',
    keyboard: 'M4 6h16a1.5 1.5 0 0 1 1.5 1.5v9A1.5 1.5 0 0 1 20 18H4a1.5 1.5 0 0 1-1.5-1.5v-9A1.5 1.5 0 0 1 4 6zM6 10h1M10 10h1M14 10h1M7 14h10',
    sidebar: 'M5 4h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2zM9 4v16',
    magnet: 'M6 3v8a6 6 0 0 0 12 0V3M6 7h4M14 7h4M10 3v8a2 2 0 0 0 4 0V3',
    notes: 'M6 3h9l4 4v14H6zM9 11h7M9 15h7M9 7h4',
    play: 'M8 5v14l11-7z', pause: 'M8 5v14M16 5v14', folder: 'M3 8V5h6l2 3h10v11H3z',
    loop: 'M17 2l3 3-3 3M4 11V9a4 4 0 0 1 4-4h12M7 22l-3-3 3-3M20 13v2a4 4 0 0 1-4 4H4', plus: 'M12 5v14M5 12h14',
    minus: 'M5 12h14', reset: 'M5 8a8 8 0 1 1-1 8M5 3v5h5',
    back: 'M16 5 7 12l9 7M5 5v14', next: 'M8 5l9 7-9 7M19 5v14',
    trash: 'M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 10v7M14 10v7',
    edit: 'm15 4 5 5-10 10-6 1 1-6zM13 6l5 5', close: 'M6 6l12 12M18 6 6 18',
    help: 'M9 8a3 3 0 1 1 5 2c-2 1-2 2-2 3M12 17h.01M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0',
    fit: 'M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5', check: 'm5 12 4 4 10-10',
    lock: 'M8 10V7a4 4 0 0 1 8 0v3M5 10h14v11H5zM12 14v3',
    unlock: 'M8 10V7a4 4 0 0 1 7.5-2M5 10h14v11H5zM12 14v3'
  };
</script>
<svg width={size} height={size} viewBox="0 0 24 24" fill={name === 'play' ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={paths[name] ?? paths.plus}/></svg>
