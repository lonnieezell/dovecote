<script lang="ts">
  import { onMount } from 'svelte';
  import { connectEngine } from './lib/engine';
  import type { EngineClient, EngineEvent } from './lib/protocol';
  import { THEMES, applyTheme, readTheme, saveTheme, type Theme } from './lib/theme';

  let engine: EngineClient | undefined = $state();
  let reply: EngineEvent | undefined = $state();
  let error: string | undefined = $state();
  let theme: Theme = $state(readTheme(localStorage));

  onMount(() => {
    let stop: (() => void) | undefined;
    let gone = false;
    (async () => {
      const client = await connectEngine();
      const off = await client.onEvent((event) => (reply = event));
      if (gone) off();
      else {
        stop = off;
        engine = client;
      }
    })().catch((e) => (error = String(e)));
    return () => {
      gone = true;
      stop?.();
    };
  });
</script>

<div class="window">
  <nav aria-label="Sidebar">
    <h2>Dovecote</h2>
    <label>
      Theme
      <select
        bind:value={theme}
        onchange={() => {
          applyTheme(theme, document.documentElement);
          saveTheme(theme, localStorage);
        }}
      >
        {#each THEMES as option}
          <option value={option}>{option}</option>
        {/each}
      </select>
    </label>
  </nav>
  <section aria-label="Message list"><h2>Inbox</h2></section>
  <main aria-label="Reading pane">
    <h2>Nothing selected</h2>
    <button disabled={!engine} onclick={() => engine?.send({ type: 'ping', message: 'hello' })}>
      Ping engine
    </button>
    {#if error}
      <p role="alert">Could not reach the engine: {error}</p>
    {/if}
    {#if reply}
      <p data-testid="engine-reply">Engine replied: {reply.message} (schema {reply.schemaVersion})</p>
    {/if}
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
  }
  .window {
    display: grid;
    grid-template-columns: 220px 340px 1fr;
    height: 100vh;
    min-width: 1000px;
    min-height: 680px;
  }
  .window > * {
    padding: 16px;
    border-right: 1px solid var(--border);
  }
  .window > :last-child {
    border-right: none;
  }
</style>
