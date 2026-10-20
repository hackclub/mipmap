<script lang="ts">
    import { Load } from "$lib/load.svelte";
    import { short_date } from "$lib/format";
    import LoadError from "$lib/ui/LoadError.svelte";
    import { fetch_located_people, fetch_locations } from "../api";
    import Awake from "./Awake.svelte";
    import { country_name, dim, load_world, ramp, scale, small, us_zone } from "./geo";
    import Globe from "./Globe.svelte";
    import Growth from "./Growth.svelte";
    import Pie from "./Pie.svelte";

    let { onopenuser }: { onopenuser: (hca_id: string) => void } = $props();

    const SLICES = 8;
    const BARS = 15;
    const LINES = 5;

    const load = new Load(fetch_locations);
    load.run();

    let picked = $state<string | null>(null);
    let offsets = $state.raw(new Map<string, number>());

    const countries = $derived(load.data?.countries ?? []);
    const counts = $derived(new Map(countries.map((c) => [c.code, c.users])));
    const stats = $derived(new Map(countries.map((c) => [c.code, c])));
    const total = $derived(countries.reduce((sum, c) => sum + c.users, 0));
    const max = $derived(countries[0]?.users ?? 0);
    const selected = $derived(picked ?? countries[0]?.code ?? null);

    const share = (users: number) =>
        total ? `${((users / total) * 100).toFixed(users / total < 0.1 ? 1 : 0)}%` : "0%";
    const rate = (part: number, whole: number) =>
        whole ? `${Math.round((part / whole) * 100)}%` : "–";

    const slices = $derived.by(() => {
        const top = countries.slice(0, SLICES).map((c) => ({
            label: country_name(c.code),
            value: c.users,
        }));
        const rest = countries.slice(SLICES).reduce((sum, c) => sum + c.users, 0);
        return rest ? [...top, { label: "Other", value: rest }] : top;
    });

    const growth = $derived.by(() => {
        const weeks = [...new Set((load.data?.weeks ?? []).map((w) => w.week))].sort();
        const index = new Map(weeks.map((week, i) => [week, i]));
        const series = countries.slice(0, LINES).map((c) => {
            const values = weeks.map(() => 0);
            for (const w of load.data?.weeks ?? [])
                if (w.country === c.code) values[index.get(w.week)!] += w.users;
            for (let i = 1; i < values.length; i++) values[i] += values[i - 1];
            return { label: country_name(c.code), values };
        });
        return { weeks, series };
    });

    const groups = $derived.by(() => {
        const out: { zone?: string; offset?: number; users: number }[] = [];
        for (const c of countries) {
            if (c.code === "US") {
                let placed = 0;
                for (const region of load.data?.regions ?? [])
                    if (region.country === "US") {
                        out.push({ zone: us_zone(region.name), users: region.users });
                        placed += region.users;
                    }
                if (c.users > placed)
                    out.push({ zone: "America/Chicago", users: c.users - placed });
            } else if (offsets.has(c.code)) {
                out.push({ offset: offsets.get(c.code), users: c.users });
            }
        }
        return out;
    });

    const regions = $derived(
        (load.data?.regions ?? []).filter((r) => r.country === selected),
    );
    const regions_total = $derived(regions.reduce((sum, r) => sum + r.users, 0));

    const people = new Load(() =>
        selected ? fetch_located_people(selected) : Promise.resolve([]),
    );
    $effect(() => {
        void selected;
        people.run();
    });

    $effect(() => {
        if (!countries.length) return;
        let stale = false;
        Promise.all([import("d3-geo"), load_world()]).then(([d3, world]) => {
            if (stale) return;
            const next = new Map<string, number>();
            for (const country of world)
                if (country.code)
                    next.set(country.code, Math.round(d3.geoCentroid(country)[0] / 15));
            for (const [code, [lon]] of Object.entries(small))
                next.set(code, Math.round(lon / 15));
            offsets = next;
        });
        return () => (stale = true);
    });

    function pick(code: string) {
        picked = code;
    }
</script>

{#if load.error}
    <LoadError error={load.error} onretry={load.run} />
{:else if !load.data}
    <p class="note">Loading…</p>
{:else if !countries.length}
    <p class="note">
        Nobody has an address yet{load.data.unknown
            ? ` (${load.data.unknown.toLocaleString()} people without one)`
            : ""}.
    </p>
{:else}
    <div class="location">
        <section class="summary" aria-label="Summary">
            <p><strong>{total.toLocaleString()}</strong> people with an address</p>
            <p><strong>{countries.length.toLocaleString()}</strong> {countries.length === 1 ? "country" : "countries"}</p>
            <p><strong>{load.data.unknown.toLocaleString()}</strong> unknown, no address on their Hack Club account</p>
        </section>

        <div class="pair">
            <section class="box">
                <h3>Globe</h3>
                <Globe countries={stats} {selected} onselect={pick} />
                <div class="key" aria-hidden="true">
                    <span>1</span>
                    <span class="gradient" style:background="linear-gradient(to right, {dim(0)}, {dim(0.5)}, {dim(1)})"></span>
                    <span>{max.toLocaleString()}</span>
                </div>
                <p class="note">Drag to turn it, hover a country for its stats, click one to open it below. Lighter means more people.</p>
            </section>
            <section class="box">
                <h3>Share by country</h3>
                <Pie {slices} />
            </section>
        </div>

        <section class="box">
            <h3>Top {Math.min(BARS, countries.length)} countries</h3>
            <ol class="bars">
                {#each countries.slice(0, BARS) as c (c.code)}
                    <li>
                        <button type="button" class="row" class:active={c.code === selected} onclick={() => pick(c.code)}>
                            <span class="name">{country_name(c.code)}</span>
                            <span class="track">
                                <span class="fill" style:width="{(c.users / max) * 100}%" style:background={ramp(scale(c.users, max))}></span>
                            </span>
                            <span class="count">{c.users.toLocaleString()}</span>
                        </button>
                    </li>
                {/each}
            </ol>
        </section>

        <section class="box">
            <h3>When people are awake</h3>
            <p class="note">
                How many people it's 9am to 9pm for at each hour of your day. Estimated from their country, and their state in the US.
            </p>
            <Awake {groups} />
        </section>

        <section class="box">
            <h3>Signups over time</h3>
            <p class="note">Total people from each of the top {growth.series.length} countries, week by week.</p>
            <Growth weeks={growth.weeks} series={growth.series} />
        </section>

        <section class="box">
            <h3>Engagement by country</h3>
            <div class="scroll">
                <table>
                    <thead>
                        <tr>
                            <th scope="col">Country</th>
                            <th scope="col">People</th>
                            <th scope="col">Onboarded</th>
                            <th scope="col">Shipped</th>
                            <th scope="col">Hours per person</th>
                            <th scope="col">Mips earned</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each countries as c (c.code)}
                            <tr class:active={c.code === selected}>
                                <th scope="row">
                                    <button type="button" class="link" onclick={() => pick(c.code)}>{country_name(c.code)}</button>
                                </th>
                                <td>{c.users.toLocaleString()}</td>
                                <td>{rate(c.onboarded, c.users)}</td>
                                <td>{rate(c.shipped, c.users)}</td>
                                <td>{(c.hours / c.users).toFixed(1)}</td>
                                <td>{c.mips.toLocaleString()}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        </section>

        <div class="pair">
            <section class="box">
                <h3>All countries</h3>
                <ol class="list">
                    {#each countries as c, i (c.code)}
                        <li>
                            <button type="button" class="row" class:active={c.code === selected} onclick={() => pick(c.code)}>
                                <span class="rank">{i + 1}</span>
                                <span class="name">{country_name(c.code)}</span>
                                <span class="count">{c.users.toLocaleString()} · {share(c.users)}</span>
                            </button>
                        </li>
                    {/each}
                </ol>
            </section>
            <section class="box">
                <h3>Top cities</h3>
                <ol class="list">
                    {#each load.data.cities as city, i (city.country + city.name)}
                        <li>
                            <span class="rank">{i + 1}</span>
                            <span class="name">{city.name}, {country_name(city.country)}</span>
                            <span class="count">{city.users.toLocaleString()}</span>
                        </li>
                    {:else}
                        <li class="note">No cities yet</li>
                    {/each}
                </ol>
            </section>
        </div>

        {#if selected}
            <h3 class="selected">{country_name(selected)}</h3>
            <div class="pair">
                <section class="box">
                    <h3>States and regions</h3>
                    <ol class="list">
                        {#each regions as region, i (region.name)}
                            <li>
                                <span class="rank">{i + 1}</span>
                                <span class="name">{region.name}</span>
                                <span class="count">{region.users.toLocaleString()}</span>
                            </li>
                        {:else}
                            <li class="note">Nobody here has a state or region on their address</li>
                        {/each}
                        {#if (counts.get(selected) ?? 0) > regions_total && regions.length}
                            <li class="note">{((counts.get(selected) ?? 0) - regions_total).toLocaleString()} without a state or region</li>
                        {/if}
                    </ol>
                </section>
                <section class="box">
                    <h3>People</h3>
                    {#if people.error}
                        <LoadError error={people.error} onretry={people.run} />
                    {:else if !people.data}
                        <p class="note">Loading…</p>
                    {:else}
                        <ul class="list people">
                            {#each people.data as person (person.id)}
                                <li>
                                    <span class="name">
                                        {person.name || `User #${person.id}`}
                                        <small>{[person.city, person.region].filter(Boolean).join(", ") || "No city or region"} · joined {short_date(person.signup_at)}</small>
                                    </span>
                                    <button type="button" class="open" onclick={() => onopenuser(person.hca_id)}>Open in Users</button>
                                </li>
                            {/each}
                        </ul>
                        {#if people.data.length >= 1000}
                            <p class="note">Showing the newest 1,000.</p>
                        {/if}
                    {/if}
                </section>
            </div>
        {/if}
    </div>
{/if}

<style>
    .location {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    h3 {
        margin: 0;
        font-size: 1rem;
    }

    p {
        margin: 0;
    }

    .note {
        color: #d6ab70;
        font-size: 0.85rem;
    }

    .summary {
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem 1.5rem;
        font-size: 0.9rem;
    }

    .summary strong {
        font-size: 1.25rem;
    }

    .pair {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 1rem;
        align-items: start;
    }

    .box {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        min-width: 0;
        padding: 1rem;
        background: #5f4838;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    .key {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        color: #d6ab70;
        font-size: 0.75rem;
    }

    .gradient {
        flex: 1;
        height: 0.5rem;
        border-radius: 999px;
    }

    ol,
    ul {
        margin: 0;
        padding: 0;
        list-style: none;
    }

    .bars {
        display: flex;
        flex-direction: column;
        gap: 0.2rem;
    }

    .row,
    .list > li:not(.note) {
        box-sizing: border-box;
        display: flex;
        align-items: center;
        gap: 0.6rem;
        width: 100%;
        padding: 0.25rem 0.4rem;
        background: none;
        border: none;
        border-radius: 4px;
        color: inherit;
        font: inherit;
        font-size: 0.85rem;
        text-align: left;
    }

    .list > li:not(.note):has(> .row) {
        padding: 0;
    }

    .row {
        cursor: pointer;
    }

    .row.active,
    tr.active {
        background: #4f2f1f;
    }

    @media (hover: hover) {
        .row:hover {
            background: #74472a;
        }
    }

    .row:focus-visible,
    .link:focus-visible,
    .open:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: -2px;
    }

    .bars .name {
        flex: 0 0 9rem;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .track {
        flex: 1;
        height: 0.75rem;
        background: #4f2f1f;
        border-radius: 2px;
    }

    .fill {
        display: block;
        height: 100%;
        border-radius: 2px;
    }

    .count {
        color: #d6ab70;
        white-space: nowrap;
    }

    .list {
        display: flex;
        flex-direction: column;
        gap: 0.1rem;
        max-height: 22rem;
        overflow: auto;
    }

    .rank {
        flex: 0 0 1.75rem;
        color: #d6ab70;
        text-align: right;
    }

    .list .name {
        flex: 1;
        min-width: 0;
        overflow-wrap: anywhere;
    }

    .people .name {
        display: flex;
        flex-direction: column;
    }

    .people small {
        color: #d6ab70;
        font-size: 0.75rem;
    }

    .open {
        flex: 0 0 auto;
        padding: 0.3rem 0.6rem;
        background: #4f2f1f;
        border: none;
        color: #ead7c2;
        font: inherit;
        font-size: 0.75rem;
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    @media (hover: hover) {
        .open:hover {
            background: #ead7c2;
            color: #4f2f1f;
        }
    }

    .selected {
        margin-top: 0.5rem;
        font-size: 1.25rem;
    }

    .scroll {
        overflow-x: auto;
    }

    table {
        width: 100%;
        border-collapse: collapse;
        font-size: 0.85rem;
    }

    th,
    td {
        padding: 0.35rem 0.5rem;
        text-align: right;
        white-space: nowrap;
    }

    th:first-child {
        text-align: left;
    }

    thead th {
        color: #d6ab70;
        font-weight: 700;
        border-bottom: 2px solid #4f2f1f;
    }

    tbody tr + tr {
        border-top: 1px solid #4f2f1f;
    }

    .link {
        padding: 0;
        background: none;
        border: none;
        color: #e8c58c;
        font: inherit;
        font-weight: 700;
        text-align: left;
        text-decoration: underline;
        cursor: pointer;
    }

    @media (max-width: 50rem) {
        .pair {
            grid-template-columns: minmax(0, 1fr);
        }

        .bars .name {
            flex-basis: 6.5rem;
        }
    }
</style>
