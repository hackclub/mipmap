<script lang="ts">
    import { get_json } from "$lib/api";
    import { Load } from "$lib/load.svelte";
    import { duration } from "$lib/format";
    import LoadError from "$lib/ui/LoadError.svelte";
    import type { Stats } from "$lib/stats/types";
    import { fetch_overview } from "./api";

    const load = new Load(() =>
        Promise.all([get_json<Stats>("/api/stats"), fetch_overview()]),
    );

    export const refresh = () => load.run();

    load.run();

    const groups = $derived.by(() => {
        if (!load.data) return;
        const [stats, overview] = load.data;
        const totals = stats.totals;
        const count = (value: number) => value.toLocaleString();
        return [
            [
                "Users",
                [
                    ["Signed up", count(stats.hours.at(-1)?.signed_up ?? 0)],
                    ["Logged in (last 24 hours)", count(totals.logged_in_last_day)],
                    [
                        "Active today so far (UTC)",
                        count(stats.days.at(-1)?.active ?? 0),
                    ],
                ],
            ],
            [
                "Review queue",
                [
                    ["Awaiting review", count(totals.awaiting_review)],
                    ["Average wait in queue", duration(totals.average_wait_hours)],
                    ["Average time to approve", duration(totals.average_approval_hours)],
                ],
            ],
            [
                "Totals",
                [
                    ["Projects shipped", count(totals.projects_shipped)],
                    ["Hours approved", count(Math.round(totals.hours_approved))],
                    ["Mips awarded", count(totals.mips_awarded)],
                    ["Orders placed", count(totals.orders_placed)],
                    [
                        "Unfulfilled orders, stickers included",
                        count(overview.unfulfilled_orders),
                    ],
                ],
            ],
        ] as const;
    });
</script>

<aside>
    <div class="head">
        <h2>Overview</h2>
        <a href="/stats">Full stats</a>
    </div>
    {#if groups}
        {#each groups as [label, values] (label)}
            <section>
                <h3>{label}</h3>
                {#each values as [name, value] (name)}
                    <div class="stat">
                        <span>{name}</span>
                        <strong>{value}</strong>
                    </div>
                {/each}
            </section>
        {/each}
    {/if}
    {#if load.error}
        <LoadError error={load.error} onretry={load.run} />
    {/if}
</aside>

<style>
    aside {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        padding: 1.25rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    .head {
        display: flex;
        justify-content: space-between;
        align-items: baseline;
        gap: 1rem;
    }

    h2 {
        margin: 0;
        font-size: 1.25rem;
    }

    h3 {
        margin: 0 0 0.5rem;
        color: #ead7c2;
        font-size: 0.9rem;
        font-weight: 400;
    }

    .stat {
        display: flex;
        justify-content: space-between;
        gap: 1rem;
        padding: 0.25rem 0;
        font-size: 0.9rem;
    }

    strong {
        font-variant-numeric: tabular-nums;
    }
</style>
