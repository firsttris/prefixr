<!-- The runner picker, as every form that runs something in a prefix has it. -->
<script lang="ts">
  import * as m from "$lib/paraglide/messages";
  import { runners } from "$lib/stores/runners";

  let {
    value = $bindable(),
    disabled = false,
    invalid = false,
  }: {
    value: string;
    disabled?: boolean;
    invalid?: boolean;
  } = $props();
</script>

<select bind:value {disabled} aria-invalid={invalid}>
  <option value="" disabled selected>{m.gameForm_runnerChoose()}</option>
  {#each $runners as runner (runner.id)}
    <option value={runner.id}>{runner.name} ({runner.kind})</option>
  {/each}
</select>
