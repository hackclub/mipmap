<script lang="ts">
    import { onMount } from "svelte";
    import { get_json } from "$lib/api";
    import { Load } from "$lib/load.svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import SiteHead from "$lib/SiteHead.svelte";
    import Funnel from "$lib/stats/Funnel.svelte";
    import LineChart from "$lib/stats/LineChart.svelte";
    import type { Day, Hour, Stats } from "$lib/stats/types";
    import { age, clock_time, duration, short_date, short_time } from "$lib/format";

    const stats = new Load(() => get_json<Stats>("/api/stats"));

    const data = $derived(stats.data);

    type Period = "day" | "week" | "all";
    const ranges: [string, Period, string, string][] = [
        ["24h", "day", "last 24 hours", "signed up in the last 24 hours"],
        ["7d", "week", "last 7 days", "signed up in the last 7 days"],
        ["All", "all", "all time", "all users"],
    ];
    let period = $state<Period>("week");
    const range = $derived(ranges.find(([, value]) => value === period) ?? ranges[1]);

    const memo = (format: (key: string) => string) => {
        const cache = new Map<string, string>();
        return (key: string) => {
            let label = cache.get(key);
            if (label === undefined) {
                label = format(key);
                cache.set(key, label);
            }
            return label;
        };
    };
    const hour_label = memo(short_time);
    const day_label = memo(short_date);

    const hourly = (hours: Hour[], pick: (hour: Hour) => number | null) => {
        const values = hours.map(pick);
        if (values.some((value) => value === null)) return null;
        return {
            values: values as number[],
            labels: hours.map((hour) => hour_label(hour.hour)),
        };
    };
    const daily = (days: Day[], pick: (day: Day) => number) => ({
        values: days.map(pick),
        labels: days.map((day) => day_label(day.date)),
    });
    const recent = (days: Day[]) => (period === "week" ? days.slice(-7) : days);

    const complete_days = $derived(data?.days.slice(0, -1) ?? []);
    const signed_up_chart = $derived(
        data &&
            (period === "day"
                ? hourly(data.hours, (hour) => hour.signed_up)
                : daily(recent(data.days), (day) => day.signed_up)),
    );
    const logged_in_chart = $derived(
        data &&
            (period === "day"
                ? hourly(data.hours, (hour) => hour.logged_in)
                : daily(recent(complete_days), (day) => day.logged_in)),
    );
    const active_chart = $derived(daily(complete_days, (day) => day.active));
    const active_day = $derived(complete_days.at(-1));
    const logged_in = $derived(
        data &&
            {
                day: data.totals.logged_in_last_day,
                week: data.totals.logged_in_last_week,
                all: data.totals.logged_in_ever,
            }[period],
    );

    const count = (value: number | null) =>
        value === null ? "<5" : value.toLocaleString();

    const funnel = $derived(data?.funnel[period]);
    const onboarding = $derived(data?.onboarding[period]);
    const masked = $derived(
        [funnel, onboarding].some(
            (counts) =>
                counts &&
                Object.entries(counts).some(
                    ([key, value]) => key !== "median_minutes" && value === null,
                ),
        ),
    );
    const onboarding_steps = $derived(
        onboarding && [
            { label: "Started", count: onboarding.started },
            { label: "Made a project", count: onboarding.made_project },
            { label: "Reached the Tech Tree", count: onboarding.reached_tech_tree },
            { label: "Unlocked the sticker", count: onboarding.unlocked_sticker },
            { label: "Redeemed the sticker", count: onboarding.stickers },
            { label: "Finished", count: onboarding.finished },
        ],
    );
    const onboarding_quick = $derived(
        onboarding && [
            [
                "Completion rate",
                onboarding.started && onboarding.finished !== null
                    ? `${Math.min(100, Math.round((onboarding.finished / onboarding.started) * 100))}%`
                    : "–",
            ],
            [
                "Median time to finish",
                duration(
                    onboarding.median_minutes === null
                        ? null
                        : onboarding.median_minutes / 60,
                ),
            ],
            ["Stickers redeemed", count(onboarding.stickers)],
            ["Still onboarding (last 7 days)", count(onboarding.in_progress)],
        ],
    );
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
            { label: "Visited the home page", count: funnel.visited },
            { label: "Signed in", count: funnel.signed_in },
            { label: "Created a project", count: funnel.created_project },
            { label: "Linked Hackatime", count: funnel.linked_hackatime },
            { label: "Logged time", count: funnel.logged_time },
            { label: "Shipped a project", count: funnel.shipped },
            { label: "Redeemed a prize", count: funnel.redeemed },
        ],
    );

    const refresh_ms = 30_000;
    const wake_gap_ms = 10_000;

    let refreshing = $state(false);
    let paused = $state(false);
    let refreshed = $state(0);
    let clock = $state(Date.now());
    let attempted = 0;
    let timer = 0;
    let destroyed = false;

    function schedule() {
        clearTimeout(timer);
        if (destroyed || refreshing || paused || document.hidden) return;
        timer = window.setTimeout(refresh, Math.max(0, attempted + refresh_ms - Date.now()));
    }

    async function refresh() {
        if (refreshing) return;
        clearTimeout(timer);
        attempted = Date.now();
        refreshing = true;
        await stats.run();
        refreshing = false;
        if (!stats.error) refreshed = clock = Date.now();
        schedule();
    }

    function wake() {
        if (document.hidden) return clearTimeout(timer);
        if (paused) return;
        clock = Date.now();
        if (Date.now() - attempted >= wake_gap_ms) refresh();
        else schedule();
    }

    function toggle_pause() {
        paused = !paused;
        if (paused) clearTimeout(timer);
        else wake();
    }

    onMount(() => {
        refresh();
        const tick = setInterval(() => {
            if (!paused) clock = Date.now();
        }, 10_000);
        return () => {
            destroyed = true;
            clearTimeout(timer);
            clearInterval(tick);
        };
    });
</script>

<svelte:document onvisibilitychange={wake} />

<SiteHead
    title="Stats - Hack Club Mipmap"
    description="Live sign-ups, daily activity and shipping numbers for Hack Club Mipmap."
    path="/stats"
/>

<nav class="site" aria-label="Site">
    <a class="home" href="/">Home</a>
</nav>

<main>
    <header>
        <h1>Mipmap Stats</h1>
        {#if data}
            <div class="refresh">
                <span
                    >{paused
                        ? `Auto-refresh paused, refreshed at ${clock_time(refreshed)}`
                        : `Refreshed ${age(clock - refreshed)}`}</span
                >
                <button
                    type="button"
                    aria-disabled={refreshing}
                    onclick={refresh}>Refresh</button
                >
                <button
                    type="button"
                    class:active={paused}
                    aria-pressed={paused}
                    onclick={toggle_pause}>Pause auto-refresh</button
                >
            </div>
        {/if}
        <p class="warning" role="status">
            {data && stats.error
                ? `Couldn't refresh (${stats.error}). Showing the last numbers we got.`
                : ""}
        </p>
    </header>

    {#if data && quick && steps && onboarding_steps && onboarding_quick}
        <div class="pair">
            <section class="card split">
                <div class="half">
                    <div class="row">
                        <h2 class="label">Signed Up Users</h2>
                        <div
                            class="ranges"
                            role="group"
                            aria-label="Time range for the sign-up and sign-in charts, the funnel and onboarding"
                        >
                            {#each ranges as [label, value] (label)}
                                <button
                                    type="button"
                                    class:active={period === value}
                                    aria-pressed={period === value}
                                    onclick={() => (period = value)}>{label}</button
                                >
                            {/each}
                        </div>
                    </div>
                    <div class="big">
                        {(data.days.at(-1)?.signed_up ?? 0).toLocaleString()}
                    </div>
                    {#if signed_up_chart}
                        <LineChart name="Signed up users" {...signed_up_chart} />
                    {:else}
                        <p class="note">Hidden while fewer than 5 people signed up.</p>
                    {/if}
                </div>
                <div class="half">
                    <h2 class="label">Users who signed in with Hack Club ({range[2]})</h2>
                    <div class="big">{count(logged_in ?? null)}</div>
                    {#if logged_in_chart}
                        <LineChart name="Users who signed in" {...logged_in_chart} />
                    {:else}
                        <p class="note">Hidden while an hour has fewer than 5 sign-ins.</p>
                    {/if}
                </div>
            </section>
            <section class="card column">
                <h2 class="label">
                    Daily Active Users ({active_day
                        ? `${day_label(active_day.date)}, UTC`
                        : "today so far"})
                </h2>
                <div class="big">
                    {(active_day ?? data.days.at(-1))?.active.toLocaleString()}
                </div>
                <LineChart name="Daily active users" {...active_chart} />
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
            <h2 class="label">Funnel: {range[3]}</h2>
            <p class="note">
                Percentages are of the previous step. Visits only count the home
                page, so sign-ins can be higher.{#if masked}
                    {" "}Counts under 5 are hidden.{/if}
            </p>
            <Funnel {steps} />
        </section>

        <section class="card onboarding">
            <h2 class="label">Onboarding: {range[3]}</h2>
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
    {:else if stats.error}
        <div class="status">
            <LoadError error={stats.error} onretry={refresh} />
        </div>
    {:else}
        <p class="status" role="status">Loading stats…</p>
    {/if}
</main>

<style>
    .site {
        box-sizing: border-box;
        max-width: 64rem;
        margin: 0 auto;
        padding: 1rem 1rem 0;
    }

    .home {
        display: inline-block;
        padding: 0.5rem 1rem;
        background-color: #4f2f1f;
        color: #ead7c2;
        font-weight: 700;
        text-decoration: none;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .home:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .home:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    main {
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: 1rem;
        max-width: 64rem;
        margin: 0 auto;
        padding: 1rem 1rem 2rem;
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
        flex-wrap: wrap;
        align-items: center;
        gap: 0.5rem 0.75rem;
        color: #d6ab70;
        font-size: 0.9rem;
        font-variant-numeric: tabular-nums;
    }

    .warning {
        flex-basis: 100%;
        margin: 0;
        font-size: 0.9rem;
    }

    .refresh button,
    .ranges button {
        min-height: 2rem;
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
    .refresh button.active,
    .ranges button:focus-visible,
    .ranges button.active {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .refresh button:hover:not([aria-disabled="true"]),
        .ranges button:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    .refresh button[aria-disabled="true"] {
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
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
    }

    .row {
        display: flex;
        flex-wrap: wrap;
        align-items: flex-start;
        justify-content: space-between;
        gap: 0.5rem;
        margin-bottom: 0.5rem;
    }

    .row .label {
        margin: 0;
    }

    .ranges {
        display: flex;
        flex-wrap: wrap;
    }

    .ranges button {
        padding: 0.35rem 0.75rem;
        font-size: 0.85rem;
    }

    .half {
        flex: 1;
    }

    .label {
        margin: 0 0 0.5rem;
        color: #ead7c2;
        font-size: 0.9rem;
        font-weight: 400;
    }

    .note {
        margin: 0 0 1rem;
        font-size: 0.85rem;
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
        margin: 0;
        padding: 3rem 0;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
    }

    @media (max-width: 40rem) {
        .pair {
            grid-template-columns: 1fr;
        }
    }
</style>
