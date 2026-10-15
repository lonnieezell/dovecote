<script lang="ts">
  import AccountSettings from './AccountSettings.svelte';
  import type {
    Account,
    Command,
    EngineClient,
    EngineEvent,
    LoginCheck,
    ServerSettings,
    Suggestion,
  } from './protocol';

  let { engine }: { engine: EngineClient } = $props();

  const NO_SERVERS: ServerSettings = { imapHost: '', imapPort: 993, smtpHost: '', smtpPort: 465 };

  let email = $state('');
  let asked = $state(false);
  let suggestion: Suggestion | null = $state(null);
  let manual = $state(false);
  let servers: ServerSettings = $state({ ...NO_SERVERS });
  let password = $state('');
  let testing = $state(false);
  let checks: { imap: LoginCheck; smtp: LoginCheck } | undefined = $state();
  let failure: string | undefined = $state();
  let added: Account | undefined = $state();

  /** Everything after the email belongs to the address that was looked up, so a new address starts over. */
  function startOver() {
    asked = false;
    checks = undefined;
    failure = undefined;
    added = undefined;
  }

  /** Sends a command; if it cannot be sent at all, says so and answers with nothing. */
  async function send(command: Command): Promise<EngineEvent[]> {
    try {
      return await engine.send(command);
    } catch (e) {
      failure = `Could not reach the engine: ${e}`;
      return [];
    }
  }

  async function lookUp(e: SubmitEvent) {
    e.preventDefault();
    email = email.trim();
    startOver();
    for (const event of await send({ type: 'suggestSettings', email })) {
      if (event.type !== 'settingsSuggested') continue;
      asked = true;
      suggestion = event.suggestion;
      manual = !event.suggestion;
      servers = { ...(event.suggestion?.settings ?? NO_SERVERS) };
    }
  }

  async function testAndAdd(e: SubmitEvent) {
    e.preventDefault();
    testing = true;
    checks = undefined;
    failure = undefined;
    const answer = await send({ type: 'addAccount', email, password, settings: $state.snapshot(servers) });
    testing = false;
    for (const event of answer) {
      if (event.type === 'loginsTested') checks = { imap: event.imap, smtp: event.smtp };
      else if (event.type === 'failed') failure = event.message;
      else if (event.type === 'accountAdded') {
        added = event.account;
        password = '';
      }
    }
  }
</script>

<section aria-label="Add an account">
  <h2>Add an account</h2>
  <form class="stacked-form" onsubmit={lookUp}>
    <label>
      Email address <input type="email" bind:value={email} oninput={startOver} required autocomplete="off" />
    </label>
    <button type="submit">Continue</button>
  </form>

  {#if asked && !added}
    <form class="stacked-form" onsubmit={testAndAdd}>
      {#if manual}
        <p>Enter the server settings for this account.</p>
        <label>IMAP server <input bind:value={servers.imapHost} required /></label>
        <label>IMAP port <input type="number" bind:value={servers.imapPort} required min="1" max="65535" /></label>
        <label>SMTP server <input bind:value={servers.smtpHost} required /></label>
        <label>SMTP port <input type="number" bind:value={servers.smtpPort} required min="1" max="65535" /></label>
        <p class="hint">Both use SSL/TLS.</p>
      {:else if suggestion}
        <p>
          {suggestion.name} settings found: {servers.imapHost}:{servers.imapPort} for reading,
          {servers.smtpHost}:{servers.smtpPort} for sending.
        </p>
        <button type="button" class="link" onclick={() => (manual = true)}>Enter the servers by hand</button>
      {/if}

      <label>
        Password or app password
        <input type="password" bind:value={password} required autocomplete="off" />
      </label>
      {#if suggestion?.provider === 'gmail'}
        <p class="hint">Gmail needs an app password, not your normal password.</p>
      {/if}
      <button type="submit" disabled={testing}>{testing ? 'Testing…' : 'Test and add'}</button>
    </form>
  {/if}

  {#if checks}
    <ul class="checks">
      <li data-testid="imap-check" class:ok={checks.imap.ok}>Reading mail (IMAP): {checks.imap.message}</li>
      <li data-testid="smtp-check" class:ok={checks.smtp.ok}>Sending mail (SMTP): {checks.smtp.message}</li>
    </ul>
  {/if}
  {#if failure}
    <p role="alert">{failure}</p>
  {/if}
  {#if added}
    <AccountSettings {engine} account={added} />
  {/if}
</section>

<style>
  .hint {
    color: var(--text-secondary);
    margin: 0;
  }
  .link {
    justify-self: start;
  }
  .checks {
    padding-left: 18px;
    color: var(--accent-text);
  }
  .checks .ok {
    color: var(--teal);
  }
</style>
