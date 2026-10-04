<script lang="ts">
    import Section from "$lib/dashboard/Section.svelte";
    import type { Bounds } from "$lib/dashboard/PanZoom.svelte";
    import { long_date } from "$lib/format";
    import rounds from "$lib/assets/fulfillment_rounds.json";

    let { bounds }: { bounds: Bounds } = $props();

    const rows: [keyof (typeof rounds)[number], string][] = [
        ["ship_by", "You ship a project by..."],
        ["review_by", "We'll get it reviewed by..."],
        ["fulfill_by", "After you claim prizes, they'll be shipped out by..."],
    ];
</script>

<Section title="Fulfillment Rounds" {bounds} zoom={3} padded>
    <p>
        You ship a project each of these dates, we'll make sure we review your
        project and fulfill claimed prizes by the specified dates.
    </p>
    <table style:--count={rounds.length}>
        <tbody>
            {#each rows as [key, label] (key)}
                <tr>
                    <th scope="row">{label}</th>
                    {#each rounds as round}
                        <td>{long_date(round[key])}</td>
                    {/each}
                </tr>
            {/each}
        </tbody>
    </table>
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
