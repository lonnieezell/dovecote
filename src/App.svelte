<script lang="ts">
  import { onMount } from 'svelte';
  import AccountSettings from './lib/AccountSettings.svelte';
  import AddAccount from './lib/AddAccount.svelte';
  import { connectEngine } from './lib/engine';
  import type { Account, EngineClient, EngineEvent } from './lib/protocol';
  import { THEMES, applyTheme, readTheme, saveTheme, type Theme } from './lib/theme';

  let engine: EngineClient | undefined = $state();
  let reply: Extract<EngineEvent, { type: 'pong' }> | undefined = $state();
  let error: string | undefined = $state();
  let theme: Theme = $state(readTheme(localStorage));
  let accounts: Account[] = $state([]);
  // What the reading pane shows: the add-account form, one account's settings, or nothing.
  let showing: 'add' | number | undefined = $state();
  let shownAccount = $derived(accounts.find((a) => a.id === showing));

  function onEvent(event: EngineEvent) {
    if (event.type === 'pong') reply = event;
    else if (event.type === 'accounts') accounts = event.accounts;
  }

  onMount(() => {
    let stop: (() => void) | undefined;
    let gone = false;
    (async () => {
      const client = await connectEngine();
      const off = await client.onEvent(onEvent);
      if (gone) off();
      else {
        stop = off;
        engine = client;
        await client.send({ type: 'listAccounts' });
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
    <ul class="accounts">
      {#each accounts as account (account.id)}
        <li>
          <button onclick={() => (showing = account.id)}>
            <span class="dot" style:background={`var(--${account.settings.color})`}></span>
            {account.settings.displayName}
          </button>
        </li>
      {/each}
    </ul>
    <button disabled={!engine} onclick={() => (showing = 'add')}>Add account</button>
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
    {#if engine && showing === 'add'}
      <AddAccount {engine} />
    {:else if engine && shownAccount}
      {#key shownAccount.id}
        <AccountSettings {engine} account={shownAccount} />
      {/key}
    {:else}
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
  .accounts {
    list-style: none;
    padding: 0;
  }
  .accounts button {
    display: flex;
    align-items: center;
    gap: 8px;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    padding: 4px 0;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
</style>
