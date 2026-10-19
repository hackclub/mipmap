<script lang="ts">
    import FormDialog from "$lib/ui/FormDialog.svelte";
    import { get_json, json, request, send_json } from "$lib/api";
    import { Load } from "$lib/load.svelte";

    type Preferences = {
        enabled: boolean;
        announcements: boolean;
        challenges: boolean;
        fulfillment_rounds: boolean;
        snapshot_reminders: boolean;
        ship_updates: boolean;
        inactivity_reminders: boolean;
    };

    let { onclose }: { onclose: () => void } = $props();

    const options: [Exclude<keyof Preferences, "enabled">, string][] = [
        ["announcements", "General Announcements"],
        ["challenges", "New Challenges"],
        ["fulfillment_rounds", "Upcoming Fulfillment Rounds"],
        ["snapshot_reminders", "Snapshot Reminders"],
        ["ship_updates", "Ship Status Updates"],
        ["inactivity_reminders", "Inactivity Reminders"],
    ];

    let fields = $state<Preferences>();
    const load = new Load(async () => {
        const { slack_linked, ...preferences } = await get_json<
            Preferences & { slack_linked: boolean }
        >("/api/slack/preferences");
        fields = preferences;
        return slack_linked;
    });
    load.run();

    const save = async () => {
        if (!fields) return;
        await json(
            await request(
                "/api/slack/preferences",
                send_json("POST", $state.snapshot(fields)),
            ),
        );
    };
</script>

<FormDialog
    title="Slack bot DMs"
    submit="Save"
    cancel
    valid={!!fields}
    onsubmit={save}
    {onclose}
>
    {#if load.error}
        <p>{load.error}</p>
    {:else if !fields}
        <p>Loading…</p>
    {:else}
        {#if load.data === false}
            <p class="note">
                Your Hack Club account doesn't have a Slack account linked yet,
                so the bot can't DM you
            </p>
        {/if}
        <label class="all">
            <span>Enable all messages</span>
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
    {/if}
</FormDialog>

<style>
    p {
        margin: 0;
        font-size: 0.85em;
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
        font-size: 0.85em;
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
</style>
