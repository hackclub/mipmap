<script lang="ts">
    import { untrack } from "svelte";
    import Field from "$lib/ui/Field.svelte";
    import { short_time } from "$lib/format";
    import {
        cancel_order,
        display_name,
        fetch_orders,
        set_fulfilled,
    } from "./api";
    import { Pages } from "./pages.svelte";
    import type { Order } from "./types";
    import ActionDialog from "./ActionDialog.svelte";
    import Card from "./Card.svelte";
    import Filters from "./Filters.svelte";
    import PagedList from "./PagedList.svelte";

    let { onchange }: { onchange: () => void } = $props();

    let status = $state<"unfulfilled" | "all">("unfulfilled");
    const pages = new Pages<Order>((before) => fetch_orders({ before, status }));

    $effect(() => {
        void status;
        untrack(pages.reset);
    });

    let editing = $state<{
        order: Order;
        kind: "fulfill" | "unfulfill" | "cancel";
    } | null>(null);
    let tracking = $state("");
    let reason = $state("");

    function open(order: Order, kind: "fulfill" | "unfulfill" | "cancel") {
        editing = { order, kind };
        tracking = order.tracking_number ?? "";
        reason = "";
    }

    function update(order: Order) {
        if (status === "unfulfilled" && order.fulfilled) pages.remove(order.id);
        else pages.replace(order);
        onchange();
    }

    const address = (order: Order) =>
        [
            [order.address_first_name, order.address_last_name]
                .filter(Boolean)
                .join(" "),
            order.address_line_1,
            order.address_line_2,
            [order.address_city, order.address_state, order.address_postal_code]
                .filter(Boolean)
                .join(", "),
            order.address_country,
            order.address_phone_number,
        ].filter(Boolean);
</script>

<Filters
    options={[
        ["unfulfilled", "Unfulfilled"],
        ["all", "All"],
    ]}
    bind:value={status}
/>

<PagedList {pages} empty="No orders here">
    {#snippet item(order)}
        <Card dim={order.fulfilled}>
            {#snippet title()}
                #{order.id} · {order.item_name}{order.fulfilled
                    ? " · Fulfilled"
                    : ""}
            {/snippet}
            {#snippet meta()}
                {display_name(order.user_name, order.user_id)}{order.email
                    ? ` · ${order.email}`
                    : ""} · {short_time(order.ordered_at)}
            {/snippet}
            <dl>
                <div>
                    <dt>Address</dt>
                    <dd>
                        {#each address(order) as line, i (i)}{line}<br />{/each}
                    </dd>
                </div>
                <div>
                    <dt>Fulfilled</dt>
                    <dd>
                        {order.fulfilled_at
                            ? short_time(order.fulfilled_at)
                            : "–"}
                    </dd>
                </div>
                <div>
                    <dt>Tracking</dt><dd>{order.tracking_number ?? "–"}</dd>
                </div>
            </dl>
            {#snippet actions()}
                <button type="button" onclick={() => open(order, "fulfill")}
                    >{order.fulfilled ? "Edit tracking" : "Mark fulfilled"}</button
                >
                {#if order.fulfilled}
                    <button
                        type="button"
                        onclick={() => open(order, "unfulfill")}
                        >Mark unfulfilled</button
                    >
                {/if}
                {#if !order.fulfilled}
                    <button type="button" onclick={() => open(order, "cancel")}
                        >Cancel & refund</button
                    >
                {/if}
            {/snippet}
        </Card>
    {/snippet}
</PagedList>

{#if editing?.kind === "fulfill"}
    {@const order = editing.order}
    <ActionDialog
        title="{order.fulfilled ? 'Edit tracking for' : 'Fulfill'} order #{order.id}"
        submit={order.fulfilled ? "Save" : "Mark fulfilled"}
        onsubmit={async () =>
            update(
                await set_fulfilled(order.id, {
                    fulfilled: true,
                    tracking_number: tracking,
                }),
            )}
        onclose={() => (editing = null)}
    >
        <Field
            label="Tracking number"
            hint="Optional, shown on their dashboard"
            maxlength={200}
            focus
            bind:value={tracking}
        />
    </ActionDialog>
{:else if editing?.kind === "unfulfill"}
    {@const order = editing.order}
    <ActionDialog
        title="Mark order #{order.id} unfulfilled?"
        submit="Mark unfulfilled"
        warning="This also clears its tracking number"
        onsubmit={async () =>
            update(
                await set_fulfilled(order.id, {
                    fulfilled: false,
                    tracking_number: "",
                }),
            )}
        onclose={() => (editing = null)}
    />
{:else if editing?.kind === "cancel"}
    {@const order = editing.order}
    <ActionDialog
        title="Cancel order #{order.id}?"
        submit="Cancel & refund"
        warning="The order is removed and its Token goes back to {display_name(
            order.user_name,
            order.user_id,
        )}, who can redeem it again"
        bind:reason
        onsubmit={async () => {
            await cancel_order(order.id, reason);
            pages.remove(order.id);
            onchange();
        }}
        onclose={() => (editing = null)}
    />
{/if}
