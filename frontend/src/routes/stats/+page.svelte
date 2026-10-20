<script lang="ts">
    import { onMount } from "svelte";
    import { get_json } from "$lib/api";
    import { Load } from "$lib/load.svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import SiteHead from "$lib/SiteHead.svelte";
    import Funnel from "$lib/stats/Funnel.svelte";
    import LineChart from "$lib/stats/LineChart.svelte";
    import type { Stats } from "$lib/stats/types";
    import { index } from "$lib/onboarding/script.svelte";
    import { duration, short_date, short_time } from "$lib/format";

    const stats = new Load(() => get_json<Stats>("/api/stats"));

    const data = $derived(stats.data);
    const today = $derived(data?.days[data.days.length - 1]);
    const now = $derived(data?.hours[data.hours.length - 1]);

    const ranges: [string, number, "day" | "week" | "all"][] = [
        ["24h", 24, "day"],
        ["7d", 24 * 7, "week"],
        ["All", Infinity, "all"],
    ];
    let range = $state(24 * 7);
    const hours = $derived(data?.hours.slice(-range) ?? []);
    const period = $derived(
        ranges.find(([, value]) => value === range)?.[2] ?? "all",
    );
    const funnel = $derived(data?.funnel[period]);
    const onboarding = $derived(data?.onboarding[period]);
    const past = (key: string) =>
        onboarding?.steps
            .filter((entry) => index(entry.step) >= index(key))
            .reduce((total, entry) => total + entry.users, 0) ?? 0;
    const onboarding_steps = $derived(
        onboarding && [
            { label: "Started", count: onboarding.started },
            { label: "Made a project", count: past("Edit") },
            { label: "Reached the Tech Tree", count: past("Tech Tree") },
            { label: "Unlocked the sticker", count: past("Unlock token") },
            { label: "Redeemed the sticker", count: past("Sticker sent") },
            { label: "Finished", count: onboarding.finished },
        ],
    );
    const onboarding_quick = $derived(
        onboarding && [
            [
                "Completion rate",
                onboarding.started
                    ? `${Math.round((onboarding.finished / onboarding.started) * 100)}%`
                    : "–",
            ],
            [
                "Median time to finish",
                onboarding.median_minutes === null
                    ? "–"
                    : `${Math.round(onboarding.median_minutes)}m`,
            ],
            ["Stickers redeemed", onboarding.stickers.toLocaleString()],
            [
                "Still onboarding",
                (onboarding.started - onboarding.finished).toLocaleString(),
            ],
        ],
    );
    const hour_labels = $derived(hours.map((hour) => short_time(hour.hour)));
    const quick = $derived(
        data && [
            [
                ["Projects created", data.totals.projects_created.toLocaleString()],
                ["Hours spent building", Math.round(data.totals.hours_building).toLocaleString()],
                ["Projects shipped", data.totals.projects_shipped.toLocaleString()],
                ["Hours approved", Math.round(data.totals.hours_approved).toLocaleString()],
            ],
            [
                ["Mips awarded", data.totals.mips_awarded.toLocaleString()],
                ["Snapshots shared", data.totals.snapshots_shared.toLocaleString()],
                ["Successful referrals", data.totals.successful_referrals.toLocaleString()],
                ["Orders placed", data.totals.orders_placed.toLocaleString()],
            ],
            [
                ["Awaiting review", data.totals.awaiting_review.toLocaleString()],
                ["Average wait in queue", duration(data.totals.average_wait_hours)],
                ["Average time to approve", duration(data.totals.average_approval_hours)],
            ],
        ],
    );
    const steps = $derived(
        funnel && [
            { label: "Visited", count: funnel.visited },
            { label: "Signed in", count: funnel.signed_in },
            { label: "Created a project", count: funnel.created_project },
            { label: "Linked Hackatime", count: funnel.linked_hackatime },
            { label: "Logged time", count: funnel.logged_time },
            { label: "Shipped a project", count: funnel.shipped },
            { label: "Redeemed a prize", count: funnel.redeemed },
        ],
    );

    const refresh_ms = 30_000;

    let refreshing = $state(false);
    let refreshed = $state(0);
    let clock = $state(Date.now());
    let attempted = 0;
    let timer = 0;

    const ago = $derived.by(() => {
        const seconds = Math.max(0, Math.floor((clock - refreshed) / 1000));
        return seconds < 60
            ? `${seconds}s ago`
            : `${Math.floor(seconds / 60)}m ${seconds % 60}s ago`;
    });

    function schedule() {
        clearTimeout(timer);
        if (document.hidden || refreshing) return;
        timer = window.setTimeout(refresh, Math.max(0, attempted + refresh_ms - Date.now()));
    }

    async function refresh() {
        clearTimeout(timer);
        attempted = Date.now();
        refreshing = true;
        await stats.run();
        refreshing = false;
        if (!stats.error) refreshed = clock = Date.now();
        schedule();
    }

    onMount(() => {
        refresh();
        const tick = setInterval(() => (clock = Date.now()), 1000);
        return () => {
            clearTimeout(timer);
            clearInterval(tick);
        };
    });
</script>

<svelte:document onvisibilitychange={schedule} />

<SiteHead
    title="Stats - Hack Club Mipmap"
    description="Live sign-ups, daily activity and shipping numbers for Hack Club Mipmap."
    path="/stats"
/>

{#if data && today && now && quick && steps && onboarding_steps && onboarding_quick}
    <main>
        <header>
            <h1>Mipmap Stats</h1>
            <div class="refresh">
                <span>Refreshed {ago}</span>
                <button type="button" disabled={refreshing} onclick={refresh}
                    >Refresh</button
                >
            </div>
        </header>

        <div class="pair">
            <section class="card split">
                <div class="ranges">
                    {#each ranges as [label, value] (label)}
                        <button
                            type="button"
                            class:active={range === value}
                            aria-pressed={range === value}
                            onclick={() => (range = value)}>{label}</button
                        >
                    {/each}
                </div>
                <div class="half">
                    <div class="label">Signed Up Users</div>
                    <div class="big">{now.signed_up.toLocaleString()}</div>
                    <LineChart
                        values={hours.map((hour) => hour.signed_up)}
                        labels={hour_labels}
                    />
                </div>
                <div class="half">
                    <div class="label">Logged In (last 24 hours)</div>
                    <div class="big">
                        {data.totals.logged_in_last_day.toLocaleString()}
                    </div>
                    <LineChart
                        values={hours.map((hour) => hour.logged_in)}
                        labels={hour_labels}
                    />
                </div>
            </section>
            <section class="card column">
                <div class="label">Daily Active Users</div>
                <div class="big">{today.active.toLocaleString()}</div>
                <LineChart
                    values={data.days.map((day) => day.active)}
                    labels={data.days.map((day) => short_date(day.date))}
                />
            </section>
        </div>

        {#each quick as row, i (i)}
            <section class="card quick">
                {#each row as [label, value] (label)}
                    <div>
                        <div class="value">{value}</div>
                        <div class="label small">{label}</div>
                    </div>
                {/each}
            </section>
        {/each}

        <section class="card">
            <div class="label">Funnel</div>
            <Funnel {steps} />
        </section>

        <section class="card onboarding">
            <div class="label">Onboarding</div>
            <div class="quick">
                {#each onboarding_quick as [label, value] (label)}
                    <div>
                        <div class="value">{value}</div>
                        <div class="label small">{label}</div>
                    </div>
                {/each}
            </div>
            <Funnel steps={onboarding_steps} />
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

    header {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        justify-content: space-between;
        gap: 0.5rem 1rem;
    }

    .refresh {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        color: #d6ab70;
        font-size: 0.9rem;
        font-variant-numeric: tabular-nums;
    }

    .refresh button,
    .ranges button {
        padding: 0.5rem 1rem;
        background-color: #4f2f1f;
        border: none;
        color: #ead7c2;
        font: inherit;
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .refresh button:focus-visible,
    .ranges button:focus-visible,
    .ranges button.active {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .refresh button:hover:not(:disabled),
        .ranges button:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    .refresh button:disabled {
        opacity: 0.5;
        cursor: default;
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

    .column,
    .half {
        display: flex;
        flex-direction: column;
    }

    .split {
        position: relative;
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
    }

    .ranges {
        position: absolute;
        top: 1.25rem;
        right: 1.25rem;
        display: flex;
    }

    .ranges button {
        padding: 0.25rem 0.6rem;
        font-size: 0.85rem;
    }

    .half {
        flex: 1;
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

    .onboarding {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .onboarding > .label {
        margin: 0;
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
