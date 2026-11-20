<script lang="ts">
  // Circular progress: starts at 12 o'clock and fills clockwise. Without a
  // value it spins as an indeterminate indicator.
  let { value = null, size = 22 }: { value?: number | null; size?: number } = $props();
</script>

<span class="ring" class:indeterminate={value === null} aria-hidden="true"
  style:--fill={`${Math.round(Math.max(0, Math.min(1, value ?? .25)) * 360)}deg`} style:width={`${size}px`} style:height={`${size}px`}></span>

<style>
  .ring{flex:none;display:inline-block;border-radius:50%;background:conic-gradient(var(--blue) var(--fill),var(--progress-track) 0);-webkit-mask:radial-gradient(circle,transparent 55%,#000 57%);mask:radial-gradient(circle,transparent 55%,#000 57%)}
  .indeterminate{animation:spin 1s linear infinite}
  @keyframes spin{to{transform:rotate(360deg)}}
  @media(prefers-reduced-motion:reduce){.indeterminate{animation:none}}
</style>
