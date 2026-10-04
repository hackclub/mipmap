<script lang="ts">
    import Section from "$lib/dashboard/Section.svelte";
    import type { Order } from "$lib/dashboard/state.svelte";
    import type { Bounds } from "$lib/dashboard/PanZoom.svelte";

    let { bounds, orders }: { bounds: Bounds; orders: Order[] } = $props();
</script>

<Section title="Orders" {bounds} zoom={3} padded>
    {#if orders.length}
        <ul>
            {#each orders as order (order.id)}
                <li>
                    <strong>{order.item_name}</strong>
                    <div>
                        Ordered {new Date(order.ordered_at).toLocaleDateString()}
                    </div>
                    <div>
                        {#if order.fulfilled && order.tracking_number}
                            Shipped, tracking number {order.tracking_number}
                        {:else if order.fulfilled}
                            Shipped
                        {:else}
                            Waiting for the next fulfillment round
                        {/if}
                    </div>
                </li>
            {/each}
        </ul>
    {:else}
        <p class="empty">No orders yet</p>
    {/if}
</Section>

<style>
    ul {
        margin: 0;
        padding: 0;
        list-style: none;
    }

    li + li {
        border-top: 2px solid #4f2f1f;
    }

    li {
        padding: 0.5rem 0;
        font-size: 0.85em;
    }
</style>
