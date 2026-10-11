<script lang="ts">
  import { applyTheme, readThemePreference, saveThemePreference, type ThemePreference } from './theme';

  const initialPreference = readThemePreference();
  applyTheme(initialPreference);
  let preference = $state<ThemePreference>(initialPreference);
  let saveFailed = $state(false);
  const nextTheme: Record<ThemePreference, ThemePreference> = { auto: 'dark', dark: 'light', light: 'auto' };

  function toggle() {
    preference = nextTheme[preference];
    saveFailed = !saveThemePreference(preference);
  }
</script>

<div class="theme-control">
  <button type="button"
    title={`${preference === 'auto' ? 'Following operating system appearance' : `Using ${preference} appearance`}. Switch to ${nextTheme[preference]}.`}
    onclick={toggle}>UI: {preference}</button>
  {#if saveFailed}<span class="theme-save-error" role="status">UI preference could not be saved.</span>{/if}
</div>

<style>
  .theme-control{display:flex;align-items:center;gap:5px;white-space:nowrap}
  button{border:0;border-radius:3px;background:transparent;color:inherit;font:inherit;padding:3px 5px;line-height:1.2;min-width:49px;text-align:left}
  button:hover{background:var(--hover);color:var(--text)}
  .theme-save-error{color:var(--error-text);white-space:normal}
</style>
