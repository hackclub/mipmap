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
    let fulfilling = $state<Wave | null>(null);

    const stickers = (count: number) =>
        `${count} sticker${count === 1 ? "" : "s"}`;
</script>

{#if load.error}
    <LoadError error={load.error} onretry={load.run} />
{:else if !load.data}
    <p class="note">Loading…</p>
{:else}
    {@const { pending, waves } = load.data}
    <div class="pending">
        <p>
            {pending
                ? `${stickers(pending)} waiting for the next wave`
                : "No stickers waiting for a wave"}
        </p>
        <button
            type="button"
            disabled={!pending}
            onclick={() => (creating = true)}>Create wave</button
        >
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
        warning="{stickers(
            load.data.pending,
        )} waiting will go into this wave and won't be in later ones"
        onsubmit={async () => load.set(await create_wave())}
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

    .pending p,
    .note {
        margin: 0;
    }

    .note {
        color: #d6ab70;
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

    .pending button:disabled {
        opacity: 0.5;
        cursor: default;
    }

    .pending button:focus-visible,
    .download:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .pending button:not(:disabled):hover,
        .download:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }
</style>
