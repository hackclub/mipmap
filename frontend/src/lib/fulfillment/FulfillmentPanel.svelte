<script lang="ts">
    import Section from "$lib/dashboard/Section.svelte";
    import type { Bounds } from "$lib/dashboard/PanZoom.svelte";
    import { long_date } from "$lib/format";
    import { rounds } from "$lib/fulfillment/rounds";

    let { bounds }: { bounds: Bounds } = $props();

    const columns: [keyof (typeof rounds)[number], string][] = [
        ["ship_by", "You ship a project by..."],
        ["review_by", "We'll get it reviewed by..."],
        ["fulfill_by", "After you redeem prizes, they'll be sent out by..."],
    ];

    const now = Date.now();
    const closed = (round: (typeof rounds)[number]) =>
        now >= Date.parse(round.ship_by) + 86_400_000;
</script>

<Section title="Fulfillment Rounds" {bounds} zoom={3} padded>
    <p>
        If you ship a project by each of these dates (end of day, UTC), we'll
        make sure we review your project and fulfill redeemed prizes by the
        specified dates.
    </p>
    <table>
        <thead>
            <tr>
                {#each columns as [key, label] (key)}
                    <th scope="col">{label}</th>
                {/each}
            </tr>
        </thead>
        <tbody>
            {#each rounds as round}
                {@const past = closed(round)}
                <tr class:past>
                    {#each columns as [key] (key)}
                        <td>
                            {#if past && key === "ship_by"}
                                <s>{long_date(round[key])}</s>
                                <span class="closed">(closed)</span>
                            {:else}
                                {long_date(round[key])}
                            {/if}
                        </td>
                    {/each}
                </tr>
            {/each}
        </tbody>
    </table>
    <p>
        Maybe more to come...     ...gotta see how well Mipmap's doing so get your
        friends to join!
    </p>
</Section>

<style>
    table {
        width: 100%;
        table-layout: fixed;
        border-collapse: separate;
        border-spacing: 0;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        overflow: hidden;
        font-size: 0.85em;
    }

    th,
    td {
        padding: 0.5rem 0.75rem;
        text-align: center;
    }

    th {
        font-weight: 700;
        vertical-align: bottom;
        border-bottom: 2px solid #4f2f1f;
    }

    :is(th, td) + :is(th, td) {
        border-left: 2px solid #4f2f1f;
    }

    tbody tr:nth-child(odd) {
        background: rgb(79 47 31 / 0.4);
    }

    .past {
        opacity: 0.85;
    }

    .closed {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }
</style>
