<script lang="ts">
    import Section from "$lib/Section.svelte";
    import type { Bounds } from "$lib/PanZoom.svelte";

    let { bounds }: { bounds: Bounds } = $props();

    let orders = $state.raw<{ id: number }[]>([]);
</script>

<Section title="Orders" {bounds} zoom={3} padded>
    {#each orders as order (order.id)}
        <div>{order.id}</div>
    {:else}
        <p>No orders yet.</p>
    {/each}
</Section>
