<script lang="ts">
  import { ACCOUNT_COLORS, type Account, type EngineClient } from './protocol';

  let { engine, account }: { engine: EngineClient; account: Account } = $props();

  // A copy to edit; the parent remounts this form when a different account is picked.
  // svelte-ignore state_referenced_locally
  let settings = $state({ ...account.settings });
  let saving = $state(false);
  let message: string | undefined = $state();

  async function save(e: SubmitEvent) {
    e.preventDefault();
    saving = true;
    message = undefined;
    try {
      const answer = await engine.send({ type: 'saveAccountSettings', id: account.id, settings: $state.snapshot(settings) });
      const failure = answer.find((event) => event.type === 'failed');
      message = failure ? failure.message : 'Saved.';
    } catch (e) {
      message = `Could not reach the engine: ${e}`;
    } finally {
      saving = false;
    }
  }
</script>

<section aria-label="Account settings">
  <h2>{account.email}</h2>
  <form class="stacked-form" onsubmit={save}>
    <label>Name <input bind:value={settings.displayName} required /></label>
    <fieldset>
      <legend>Color</legend>
      {#each ACCOUNT_COLORS as color, i}
        <label class="swatch" style:--swatch={`var(--${color})`}>
          <input type="radio" name="color" value={color} bind:group={settings.color} aria-label={`Color ${i + 1}`} />
        </label>
      {/each}
    </fieldset>
    <label>Signature <textarea bind:value={settings.signature} rows="4"></textarea></label>
    <label>Send from <input type="email" bind:value={settings.sendFrom} required /></label>
    <button type="submit" disabled={saving}>Save settings</button>
    {#if message}
      <p role="status">{message}</p>
    {/if}
  </form>
</section>

<style>
  fieldset {
    display: flex;
    gap: 8px;
    border: none;
    padding: 0;
  }
  .swatch {
    display: block;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--swatch);
  }
  .swatch:has(input:checked) {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }
  .swatch input {
    opacity: 0;
    margin: 0;
    width: 100%;
    height: 100%;
  }
</style>
