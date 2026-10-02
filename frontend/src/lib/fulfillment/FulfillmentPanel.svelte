<script lang="ts">
    import { onMount } from "svelte";
    import Section from "$lib/Section.svelte";
    import LoadError from "$lib/LoadError.svelte";
    import type { Bounds } from "$lib/PanZoom.svelte";
    import { json, request } from "$lib/api";
    import { Load } from "$lib/load.svelte";

    interface Round {
        ship_by: string;
        review_by: string;
        fulfill_by: string;
    }

    let { bounds }: { bounds: Bounds } = $props();

    const rows: [keyof Round, string][] = [
        ["ship_by", "You ship a project by..."],
        ["review_by", "We'll get it reviewed by..."],
        ["fulfill_by", "After you claim prizes, they'll be shipped out by..."],
    ];

    const format = (date: string) =>
        new Date(date).toLocaleDateString(undefined, {
            month: "short",
            day: "numeric",
            year: "numeric",
            timeZone: "UTC",
        });

    const rounds = new Load(() =>
        request("/dashboard/fulfillment-rounds.json").then((r) =>
            json<Round[]>(r),
        ),
    );

    onMount(rounds.run);
</script>

<Section title="Fulfillment Rounds" {bounds} zoom={3} padded>
    <p>
        You ship a project each of these dates, we'll make sure we review your
        project and fulfill claimed prizes by the specified dates.
    </p>
    {#if rounds.error}
        <LoadError error={rounds.error} onretry={rounds.run} />
    {:else if rounds.data?.length}
        <table style:--count={rounds.data.length}>
            <tbody>
                {#each rows as [key, label] (key)}
                    <tr>
                        <th scope="row">{label}</th>
                        {#each rounds.data as round}
                            <td>{format(round[key])}</td>
                        {/each}
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</Section>

<style>
    table {
        width: 100%;
        table-layout: fixed;
        border-collapse: separate;
        border-spacing: 0;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        font-size: 0.85em;
    }

    th,
    td {
        padding: 0.5rem 0.75rem;
    }

    th {
        width: calc(200% / (var(--count) + 2));
        text-align: left;
        font-weight: 700;
    }

    td {
        border-left: 2px solid #4f2f1f;
        text-align: center;
    }

    tr + tr > :is(th, td) {
        border-top: 2px solid #4f2f1f;
    }
</style>
