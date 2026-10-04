<script lang="ts">
    import { onMount } from "svelte";
    import { get_json } from "$lib/api";
    import { Load } from "$lib/load.svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import SiteHead from "$lib/SiteHead.svelte";
    import Funnel from "$lib/stats/Funnel.svelte";
    import LineChart from "$lib/stats/LineChart.svelte";
    import type { Stats } from "$lib/stats/types";

    const stats = new Load(() => get_json<Stats>("/api/stats"));

    const data = $derived(stats.data);
    const today = $derived(data?.days[data.days.length - 1]);
    const quick = $derived(
        data && [
            ["Hours spent building", Math.round(data.totals.hours_building)],
            ["Projects shipped", data.totals.projects_shipped],
            ["Hours approved", Math.round(data.totals.hours_approved)],
            ["Mips awarded", data.totals.mips_awarded],
            ["Snapshots shared", data.totals.snapshots_shared],
            ["Successful referrals", data.totals.successful_referrals],
            ["Orders placed", data.totals.orders_placed],
        ],
    );
    const steps = $derived(
        data && [
            { label: "Visited", count: data.funnel.visited },
            { label: "Signed in", count: data.funnel.signed_in },
            { label: "Linked Hackatime", count: data.funnel.linked_hackatime },
            { label: "Logged time", count: data.funnel.logged_time },
            { label: "Shipped a project", count: data.funnel.shipped },
            { label: "Redeemed a prize", count: data.funnel.redeemed },
        ],
    );

    onMount(stats.run);
</script>

<SiteHead
    title="Stats - Hack Club Mipmap"
    description="Live sign-ups, daily activity and shipping numbers for Hack Club Mipmap."
    path="/stats"
/>

{#if data && today && quick && steps}
    <main>
        <h1>Mipmap Stats</h1>

        <div class="pair">
            <section class="card">
                <div class="label">Signed Up Users</div>
                <div class="big">{today.signed_up.toLocaleString()}</div>
                <LineChart days={data.days} value={(day) => day.signed_up} />
            </section>
            <section class="card">
                <div class="label">Daily Active Users</div>
                <div class="big">{today.active.toLocaleString()}</div>
                <LineChart days={data.days} value={(day) => day.active} />
            </section>
        </div>

        <section class="card quick">
            {#each quick as [label, value] (label)}
                <div>
                    <div class="value">{value.toLocaleString()}</div>
                    <div class="label small">{label}</div>
                </div>
            {/each}
        </section>

        <section class="card">
            <div class="label">Funnel</div>
            <Funnel {steps} />
        </section>
    </main>
{:else if stats.error}
    <div class="status">
        <LoadError error={stats.error} onretry={stats.run} />
    </div>
{/if}

<style>
    main {
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: 1rem;
        max-width: 64rem;
        margin: 0 auto;
        padding: 2rem 1rem;
        color: #ead7c2;
    }

    h1 {
        margin: 0;
        font-size: 2rem;
        font-weight: 700;
    }

    .pair {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1rem;
    }

    .card {
        box-sizing: border-box;
        min-width: 0;
        padding: 1.25rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    .label {
        margin-bottom: 0.5rem;
        color: #d6ab70;
        font-size: 0.9rem;
    }

    .big {
        margin-bottom: 1rem;
        font-size: 3rem;
        font-weight: 700;
    }

    .quick {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(7rem, max-content));
        justify-content: space-between;
        gap: 1rem 0.75rem;
    }

    .value {
        font-size: 1.75rem;
        font-weight: 700;
    }

    .label.small {
        margin: 0;
        font-size: 0.85rem;
    }

    .status {
        position: fixed;
        inset: 0;
        display: grid;
        place-items: center;
        padding: 1.5rem;
        color: #ead7c2;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
    }

    @media (max-width: 40rem) {
        .pair {
            grid-template-columns: 1fr;
        }
    }
</style>
