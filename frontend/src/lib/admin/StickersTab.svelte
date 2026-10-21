<script lang="ts">
    import { Load } from "$lib/load.svelte";
    import { short_time } from "$lib/format";
    import LoadError from "$lib/ui/LoadError.svelte";
    import { create_wave, fetch_stickers, fulfill_wave, wave_csv } from "./api";
    import type { Wave } from "./types";
    import ActionDialog from "./ActionDialog.svelte";
    import Card from "./Card.svelte";

    let { onchange }: { onchange: () => void } = $props();

    const load = new Load(fetch_stickers);
    load.run();

    let creating = $state(false);
    let no_time = $state(false);
    let cap = $state<number | null>(null);

    const cap_valid = $derived(
        cap == null || (Number.isInteger(cap) && cap >= 1),
    );
    const selected = $derived.by(() => {
        if (!load.data) return 0;
        const available = no_time
            ? load.data.pending_no_time
            : load.data.pending;
        return cap_valid && cap != null ? Math.min(cap, available) : available;
    });
    let fulfilling = $state<Wave | null>(null);
    let checking = $state(false);

    async function start() {
        if (checking) return;
        checking = true;
        await load.run();
        checking = false;
        if (!load.error && selected && cap_valid) creating = true;
    }

    const stickers = (count: number) =>
        `${count} sticker${count === 1 ? "" : "s"}`;
</script>

<div role="status">
    {#if !load.error && !load.data}<p class="note">Loading…</p>{/if}
</div>
{#if load.error}
    <LoadError error={load.error} onretry={load.run} />
{:else if load.data}
    {@const { pending, pending_no_time, waves } = load.data}
    <div class="pending">
        <p>
            {pending
                ? `${stickers(pending)} waiting for the next wave, ${pending_no_time} from users with no time logged`
                : "No stickers waiting for a wave"}
        </p>
        <div class="options">
            <label class="check">
                <input type="checkbox" bind:checked={no_time} />
                Only users with no time logged
            </label>
            <label class="cap">
                Cap
                <input
                    type="number"
                    min="1"
                    step="1"
                    placeholder="None"
                    bind:value={cap}
                />
            </label>
            <button
                type="button"
                aria-disabled={!selected || !cap_valid || checking}
                onclick={() => {
                    if (selected && cap_valid) start();
                }}>{checking ? "Checking…" : "Create wave"}</button
            >
        </div>
    </div>
    <div class="list">
        {#each waves as wave (wave.id)}
            <Card dim={!wave.unfulfilled}>
                {#snippet title()}
                    Wave #{wave.id} · {short_time(wave.created_at)}{wave.unfulfilled
                        ? ""
                        : " · Fulfilled"}
                {/snippet}
                {#snippet meta()}
                    {stickers(wave.stickers)} · by {wave.admin_name || "Unknown admin"}
                {/snippet}
                {#snippet actions()}
                    <a class="download" href={wave_csv(wave.id)} download
                        >Download CSV</a
                    >
                    {#if wave.unfulfilled}
                        <button
                            type="button"
                            onclick={() => (fulfilling = wave)}
                            >Mark wave fulfilled</button
                        >
                    {/if}
                {/snippet}
            </Card>
        {:else}
            <p class="note">No waves yet</p>
        {/each}
    </div>
{/if}

{#if creating && load.data}
    <ActionDialog
        title="Create a sticker wave?"
        submit="Create wave"
        warning="The {stickers(selected)} waiting longest{no_time
            ? ' from users with no time logged'
            : ''} will go into this wave and won't be in later ones"
        onsubmit={async () => load.set(await create_wave({ no_time, cap }))}
        onclose={() => (creating = false)}
    />
{:else if fulfilling}
    {@const wave = fulfilling}
    <ActionDialog
        title="Mark wave #{wave.id} fulfilled?"
        submit="Mark wave fulfilled"
        warning="{stickers(
            wave.unfulfilled,
        )} still unfulfilled in this wave will be marked fulfilled"
        onsubmit={async () => {
            load.set(await fulfill_wave(wave.id));
            onchange();
        }}
        onclose={() => (fulfilling = null)}
    />
{/if}

<style>
    .pending {
        display: flex;
        flex-wrap: wrap;
        justify-content: space-between;
        align-items: center;
        gap: 0.5rem 1rem;
        margin-bottom: 1rem;
    }

    .options {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 0.5rem 1rem;
        font-size: 0.85rem;
    }

    .check,
    .cap {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        cursor: pointer;
    }

    .check input {
        accent-color: #d6ab70;
    }

    .cap input {
        box-sizing: border-box;
        width: 5rem;
        padding: 0.3rem 0.5rem;
        background: #3d3029;
        border: 1px solid #d6ab70;
        border-radius: 4px;
        color: #ead7c2;
        font: inherit;
    }

    .cap input::placeholder {
        color: #ead7c2;
        opacity: 0.75;
    }

    .cap input:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: 1px;
    }

    .pending p,
    .note {
        margin: 0;
    }

    .note {
        color: #ead7c2;
    }

    .list {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .pending button,
    .download {
        padding: 0.4rem 0.8rem;
        background-color: #4f2f1f;
        border: none;
        color: #ead7c2;
        font: inherit;
        font-size: 0.85rem;
        font-weight: 700;
        text-decoration: none;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .pending button[aria-disabled="true"] {
        opacity: 0.5;
        cursor: default;
    }

    .pending button:focus-visible,
    .download:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .pending button:not([aria-disabled="true"]):hover,
        .download:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }
</style>
