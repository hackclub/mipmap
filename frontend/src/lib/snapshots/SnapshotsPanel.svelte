<script lang="ts">
    import { onMount } from "svelte";
    import LoadError from "$lib/LoadError.svelte";
    import Section from "$lib/Section.svelte";
    import type { Bounds } from "$lib/PanZoom.svelte";
    import { Load } from "$lib/load.svelte";
    import SendSnapshot from "$lib/snapshots/SendSnapshot.svelte";
    import SnapshotCard from "$lib/snapshots/SnapshotCard.svelte";
    import {
        fetch_my_snapshots,
        fetch_snapshots,
        fetch_status,
    } from "$lib/snapshots/api";

    let { bounds }: { bounds: Bounds } = $props();

    const columns = 6;
    const rows = 4;

    const everyone = new Load(fetch_snapshots);
    const mine = new Load(fetch_my_snapshots);
    const status = new Load(fetch_status);

    onMount(() => {
        everyone.run();
        mine.run();
        status.run();
    });
</script>

<Section title="Snapshots" {bounds} zoom={2.5}>
    <div class="areas">
        <div class="area send">
            <h3>Send a Snapshot!</h3>
            {#if status.error}
                <LoadError error={status.error} onretry={status.run} />
            {:else}
                <SendSnapshot
                    status={status.data}
                    onsend={async (snapshot) => {
                        mine.data = [snapshot, ...(mine.data ?? [])];
                        await status.run();
                    }}
                />
            {/if}
        </div>
        <div class="area mine" data-scroll>
            {#if mine.error}
                <LoadError error={mine.error} onretry={mine.run} />
            {:else if mine.data}
                {#each mine.data as snapshot (snapshot.id)}
                    <SnapshotCard {snapshot} />
                {:else}
                    <p class="empty">No snapshots yet.</p>
                {/each}
            {/if}
        </div>
        <div class="area everyone" style:--columns={columns}>
            {#if everyone.error}
                <div class="error">
                    <LoadError error={everyone.error} onretry={everyone.run} />
                </div>
            {:else if everyone.data}
                {#each everyone.data.slice(0, columns * rows) as snapshot (snapshot.id)}
                    <SnapshotCard {snapshot} />
                {/each}
            {/if}
        </div>
    </div>
</Section>

<style>
    .areas {
        display: grid;
        grid-template-columns: minmax(0, 1fr) minmax(0, 2fr);
        grid-template-rows: minmax(0, 1fr) minmax(0, 3fr);
        grid-template-areas:
            "send everyone"
            "mine everyone";
        height: 100%;
    }

    .area {
        box-sizing: border-box;
        min-height: 0;
        padding: 1.5rem;
    }

    .send {
        grid-area: send;
        display: flex;
        flex-direction: column;
        gap: 1rem;
        border-right: 2px solid #4f2f1f;
        border-bottom: 2px solid #4f2f1f;
    }

    .send > :global(form) {
        flex: 1;
        min-height: 0;
    }

    h3 {
        margin: 0;
        font: inherit;
        font-weight: 700;
    }

    .mine {
        grid-area: mine;
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
        overflow-y: auto;
        touch-action: pan-y;
        border-right: 2px solid #4f2f1f;
    }

    .everyone {
        grid-area: everyone;
        display: grid;
        grid-template-columns: repeat(var(--columns), minmax(0, 1fr));
        align-content: start;
        gap: 1.5rem;
    }

    .error {
        grid-column: 1 / -1;
    }

    .empty {
        margin: 0;
        font-size: 0.85em;
    }
</style>
