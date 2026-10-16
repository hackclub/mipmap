<script lang="ts">
    import Section from "$lib/dashboard/Section.svelte";
    import type { Bounds } from "$lib/dashboard/PanZoom.svelte";
    import type { Dashboard } from "$lib/dashboard/state.svelte";
    import SendSnapshot from "$lib/snapshots/SendSnapshot.svelte";
    import SnapshotCard from "$lib/snapshots/SnapshotCard.svelte";

    let {
        bounds,
        snapshots,
    }: { bounds: Bounds; snapshots: Dashboard["snapshots"] } = $props();

    const columns = 6;
    const rows = 4;
</script>

<Section title="Snapshots" {bounds} zoom={2.5}>
    <div class="areas">
        <div class="area send">
            <h3>Send a Snapshot!</h3>
            <SendSnapshot status={snapshots.status} />
            <div class="multiplier">
                <strong>{snapshots.status.multiplier.toFixed(2)}× Mips</strong>
                <p>
                    Each shared Snapshot gets you another 1% more Mips, up
                    to a maximum 32% boost!
                </p>
            </div>
        </div>
        <div class="area mine" data-scroll>
            {#each snapshots.mine as snapshot (snapshot.id)}
                <SnapshotCard {snapshot} />
            {:else}
                <p class="empty">No Snapshots yet</p>
            {/each}
        </div>
        <div class="area everyone" style:--columns={columns}>
            {#each snapshots.everyone.slice(0, columns * rows) as snapshot (snapshot.id)}
                <SnapshotCard {snapshot} />
            {/each}
        </div>
    </div>
</Section>

<style>
    .areas {
        display: grid;
        grid-template-columns: minmax(0, 1fr) minmax(0, 2fr);
        grid-template-rows: minmax(0, 9fr) minmax(0, 11fr);
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

    .multiplier {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        padding-top: 1rem;
        border-top: 2px solid #4f2f1f;
    }

    .multiplier p {
        margin: 0;
        font-size: 0.85em;
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

    .empty {
        margin: 0;
        font-size: 0.85em;
    }
</style>
