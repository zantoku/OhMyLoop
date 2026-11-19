<script lang="ts">
  // A compact Settings button bound to an action: its label, tooltip and
  // availability come from the keymap model. `locked` dims it while a
  // download started in another section runs (Settings decision 6).
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { id, label, keymap, available, run, locked = false, variant = '', onclick }:
    { id: ButtonId; label: string; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void;
      locked?: boolean; variant?: '' | 'primary' | 'ghost' | 'danger'; onclick?: () => void } = $props();
</script>

<button class="compact-button {variant === 'danger' ? 'ghost danger' : variant}" disabled={locked || !available(id)} title={keymap.title(id)}
  onclick={() => (onclick ? onclick() : run(id))}>{keymap.buttonLabel(id, label)}</button>
