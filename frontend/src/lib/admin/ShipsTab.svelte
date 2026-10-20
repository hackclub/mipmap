<script lang="ts">
    import { untrack } from "svelte";
    import Field from "$lib/ui/Field.svelte";
    import { short_time } from "$lib/format";
    import { display_name, fetch_ships, safe_url, set_bonus } from "./api";
    import { Pages } from "./pages.svelte";
    import type { Ship } from "./types";
    import ActionDialog from "./ActionDialog.svelte";
    import Card from "./Card.svelte";
    import Filters from "./Filters.svelte";
    import PagedList from "./PagedList.svelte";

    type Status = "pending" | "approved" | "rejected" | "all";

    let { onchange }: { onchange: () => void } = $props();

    let status = $state<Status>("pending");
    const pages = new Pages<Ship>((before) => fetch_ships({ before, status }));

    $effect(() => {
        void status;
        untrack(pages.reset);
    });

    let editing = $state<Ship | null>(null);
    let percent = $state("");
    let raw = $state("");
    let reason = $state("");

    const parsed_percent = $derived(Number(percent || 0));
    const parsed_raw = $derived(Number(raw || 0));
    const valid = $derived(
        Number.isFinite(parsed_percent) &&
            parsed_percent >= 0 &&
            parsed_percent <= 1000 &&
            Number.isInteger(parsed_raw) &&
            parsed_raw >= 0 &&
            parsed_raw <= 100000,
    );
    const lowering = $derived(
        !!editing &&
            (parsed_percent < editing.bonus_percent ||
                parsed_raw < editing.bonus_mips),
    );

    function edit(ship: Ship) {
        editing = ship;
        percent = String(ship.bonus_percent);
        raw = String(ship.bonus_mips);
        reason = "";
    }

    const state_of = (ship: Ship) =>
        ship.rejected
            ? "Rejected"
            : ship.awaiting_id
              ? "Waiting for ID verification"
              : ship.review_status === "2–Submitted"
                ? "Approved"
                : "Pending";

    const hours = (value: number | null) =>
        value === null ? "–" : `${value.toFixed(1)}h`;
</script>

<Filters
    options={[
        ["pending", "Pending"],
        ["approved", "Approved"],
        ["rejected", "Rejected"],
        ["all", "All"],
    ]}
    bind:value={status}
/>

<PagedList {pages} empty="No ships here">
    {#snippet item(ship)}
        <Card dim={ship.rejected}>
            {#snippet title()}
                #{ship.id} · {ship.project}
            {/snippet}
            {#snippet meta()}
                {display_name(ship.user_name, ship.user_id)}{ship.email
                    ? ` · ${ship.email}`
                    : ""} · {short_time(ship.created_at)}
            {/snippet}
            <dl>
                <div><dt>Status</dt><dd>{state_of(ship)}</dd></div>
                <div>
                    <dt>Review status</dt><dd>{ship.review_status ?? "–"}</dd>
                </div>
                <div><dt>Tier</dt><dd>{ship.tier ?? "–"}</dd></div>
                <div><dt>Hours</dt><dd>{hours(ship.hours)}</dd></div>
                <div>
                    <dt>Tracked hours</dt><dd>{hours(ship.tracked_hours)}</dd>
                </div>
                <div><dt>Mips/hour</dt><dd>{ship.mips_per_hour}</dd></div>
                <div>
                    <dt>Multiplier</dt><dd>{ship.multiplier ?? "–"}</dd>
                </div>
                <div><dt>Mips</dt><dd>{ship.mips}</dd></div>
                <div>
                    <dt>Bonus</dt>
                    <dd>+{ship.bonus_percent}% +{ship.bonus_mips}</dd>
                </div>
            </dl>
            <div class="links">
                {#if safe_url(ship.code_url)}
                    <a href={safe_url(ship.code_url)} target="_blank" rel="noreferrer"
                        >Code</a
                    >
                {/if}
                {#if safe_url(ship.repo_url)}
                    <a href={safe_url(ship.repo_url)} target="_blank" rel="noreferrer"
                        >Repo</a
                    >
                {/if}
                {#if safe_url(ship.demo_url)}
                    <a href={safe_url(ship.demo_url)} target="_blank" rel="noreferrer"
                        >Demo</a
                    >
                {/if}
                {#if safe_url(ship.screenshot_url)}
                    <a
                        href={safe_url(ship.screenshot_url)}
                        target="_blank"
                        rel="noreferrer">Screenshot</a
                    >
                {/if}
                {#if ship.airtable_url}
                    <a href={ship.airtable_url} target="_blank" rel="noreferrer"
                        >Airtable</a
                    >
                {/if}
            </div>
            {#snippet actions()}
                <button type="button" onclick={() => edit(ship)}>Bonus</button>
            {/snippet}
        </Card>
    {/snippet}
</PagedList>

{#if editing}
    {@const ship = editing}
    <ActionDialog
        title="Bonus for {ship.project}"
        submit="Save bonus"
        warning={lowering
            ? "This lowers the bonus, so their Mips will go down"
            : "Bonuses only pay out while the ship is approved"}
        {valid}
        bind:reason
        onsubmit={async () => {
            pages.replace(
                await set_bonus(ship.id, {
                    percent: parsed_percent,
                    mips: parsed_raw,
                    reason,
                }),
            );
            onchange();
        }}
        onclose={() => (editing = null)}
    >
        <div class="row">
            <Field
                label="Percentage"
                type="number"
                min="0"
                max="1000"
                step="any"
                focus
                bind:value={percent}
            />
            <Field
                label="Raw Mips"
                type="number"
                min="0"
                max="100000"
                step="1"
                bind:value={raw}
            />
        </div>
    </ActionDialog>
{/if}

<style>
    .row {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1rem;
    }
</style>
