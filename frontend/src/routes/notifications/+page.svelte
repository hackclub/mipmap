<script lang="ts">
    import { onMount } from "svelte";
    import { replaceState } from "$app/navigation";
    import { get_json, json, request, send_json } from "$lib/api";
    import { Load, Task } from "$lib/load.svelte";
    import SiteHead from "$lib/SiteHead.svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";

    type Preferences = {
        enabled: boolean;
        announcements: boolean;
        challenges: boolean;
        fulfillment_rounds: boolean;
        snapshot_reminders: boolean;
        ship_updates: boolean;
        inactivity_reminders: boolean;
    };
    type Settings = Preferences & { slack_linked: boolean };

    const options: [Exclude<keyof Preferences, "enabled">, string][] = [
        ["announcements", "General Announcements"],
        ["challenges", "New Challenges"],
        ["fulfillment_rounds", "Upcoming Fulfillment Rounds"],
        ["snapshot_reminders", "Snapshot Reminders"],
        ["ship_updates", "Ship Status Updates"],
        ["inactivity_reminders", "Inactivity Reminders"],
    ];

    let fields = $state<Preferences>();
    let unsubscribed = $state(false);
    let saved = $state(false);
    const save = new Task();

    const settings = new Load(async () => {
        const unsubscribe = new URLSearchParams(location.search).has(
            "unsubscribe",
        );
        const { slack_linked, ...preferences } = unsubscribe
            ? await json<Settings>(
                  await request(
                      "/api/slack/unsubscribe",
                      send_json("POST", {}),
                  ),
              )
            : await get_json<Settings>("/api/slack/preferences");
        if (unsubscribe) {
            replaceState(location.pathname, {});
            unsubscribed = true;
        }
        fields = preferences;
        return slack_linked;
    });

    onMount(() => {
        settings.run();
    });

    function submit(e: SubmitEvent) {
        e.preventDefault();
        if (!fields || save.pending) return;
        saved = false;
        save.run(
            async () =>
                json(
                    await request(
                        "/api/slack/preferences",
                        send_json("POST", $state.snapshot(fields)),
                    ),
                ),
            () => {
                saved = true;
                unsubscribed = false;
            },
        );
    }
</script>

<SiteHead
    title="Slack DMs - Hack Club Mipmap"
    description="Choose which Slack DMs the Hack Club Mipmap bot sends you."
    path="/notifications"
/>
<svelte:head>
    <meta name="robots" content="noindex" />
</svelte:head>

<main>
    <header>
        <h1>Slack bot DMs</h1>
        <a href="/dashboard">Dashboard</a>
    </header>

    {#if settings.error}
        <LoadError error={settings.error} onretry={settings.run} />
    {:else if !fields}
        <p class="note">Loading…</p>
    {:else}
        {#if unsubscribed}
            <p class="status" role="status">
                You've unsubscribed, so Steve the Sprite won't DM you anymore.
                You can turn DMs back on here any time.
            </p>
        {/if}
        {#if settings.data === false}
            <p class="note">
                Your Hack Club account doesn't have a Slack account linked yet,
                so the bot can't DM you (do it in <a
                    href="https://auth.hackclub.com"
                    target="_blank"
                    rel="noopener">Hack Club Auth</a
                >)
            </p>
        {/if}
        <form onsubmit={submit} onchange={() => (saved = false)}>
            <div class="box">
                <label class="all">
                    <span>Allow DMs from Steve the Sprite (Mipmap bot)</span>
                    <input type="checkbox" bind:checked={fields.enabled} />
                </label>
                <ul>
                    {#each options as [key, label] (key)}
                        <li>
                            <label>
                                <span>{label}</span>
                                <input
                                    type="checkbox"
                                    bind:checked={fields[key]}
                                    disabled={!fields.enabled}
                                />
                            </label>
                        </li>
                    {/each}
                </ul>
            </div>
            <div class="actions">
                <MipmapButton type="submit" disabled={save.pending}
                    >Save</MipmapButton
                >
                <p class="note" aria-live="polite">
                    {save.error || (saved ? "Saved" : "")}
                </p>
            </div>
        </form>
    {/if}
</main>

<style>
    main {
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: 2rem;
        max-width: 44rem;
        margin: 0 auto;
        padding: 3rem 1rem;
        color: #ead7c2;
    }

    header {
        display: flex;
        justify-content: space-between;
        align-items: baseline;
        gap: 1rem;
    }

    h1 {
        margin: 0;
        font-size: clamp(1.75rem, 5vw, 2.75rem);
        font-weight: 700;
        line-height: 1.2;
    }

    p {
        margin: 0;
    }

    .note {
        color: #d6ab70;
    }

    .status {
        font-size: 1.25rem;
    }

    form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .box {
        padding: 0.5rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    ul {
        display: flex;
        flex-direction: column;
        margin: 0;
        padding: 0;
        list-style: none;
    }

    label {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 0.5rem 0.75rem;
        cursor: pointer;
    }

    .all {
        font-weight: 700;
        border-bottom: 2px solid #4f2f1f;
    }

    label:has(:disabled) {
        cursor: not-allowed;
        opacity: 0.6;
    }

    span {
        flex: 1;
    }

    input[type="checkbox"] {
        flex: none;
        width: 1.25em;
        height: 1.25em;
        margin: 0;
        appearance: none;
        background: #4f2f1f;
        cursor: pointer;
    }

    input[type="checkbox"]:disabled {
        cursor: not-allowed;
    }

    input[type="checkbox"]:checked {
        background: #d6ab70;
        box-shadow: inset 0 0 0 0.25em #4f2f1f;
    }

    input[type="checkbox"]:focus-visible {
        outline: 2px solid #ead7c2;
    }

    .actions {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 1rem;
    }

    @media (max-width: 30rem) {
        main {
            gap: 1.5rem;
            padding: 2rem 1rem;
        }

        h1 {
            font-size: clamp(1.5rem, 6.5vw, 1.75rem);
        }

        .actions {
            flex-direction: column;
            align-items: stretch;
            gap: 0.5rem;
            text-align: center;
        }
    }
</style>
