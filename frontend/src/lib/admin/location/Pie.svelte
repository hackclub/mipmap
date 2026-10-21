<script lang="ts">
    import { ramp } from "./geo";

    let { slices }: { slices: { label: string; value: number }[] } = $props();

    let hovered = $state<number | null>(null);

    const total = $derived(slices.reduce((sum, slice) => sum + slice.value, 0));

    const arcs = $derived.by(() => {
        let angle = -Math.PI / 2;
        return slices.map((slice, i) => {
            const sweep = total ? (slice.value / total) * Math.PI * 2 : 0;
            const start = angle;
            angle += sweep;
            const point = (a: number) => `${Math.cos(a) * 50} ${Math.sin(a) * 50}`;
            const path =
                sweep >= Math.PI * 2 - 1e-6
                    ? "M -50 0 A 50 50 0 1 1 50 0 A 50 50 0 1 1 -50 0 Z"
                    : `M 0 0 L ${point(start)} A 50 50 0 ${sweep > Math.PI ? 1 : 0} 1 ${point(angle)} Z`;
            const other = i === slices.length - 1 && slice.label === "Other";
            return {
                ...slice,
                path,
                color: other
                    ? "#4f2f1f"
                    : ramp(0.4 + 0.6 * (1 - i / Math.max(1, slices.length - 1))),
                share: total ? slice.value / total : 0,
            };
        });
    });

    const percent = (share: number) =>
        `${(share * 100).toFixed(share < 0.1 ? 1 : 0)}%`;
</script>

<div class="pie">
    <svg viewBox="-52 -52 104 104" role="img" aria-label="Share of people by country">
        {#each arcs as arc, i (i)}
            <path
                d={arc.path}
                fill={arc.color}
                class:faded={hovered !== null && hovered !== i}
                role="presentation"
                onpointerenter={() => (hovered = i)}
                onpointerleave={() => (hovered = null)}
            />
        {/each}
    </svg>
    <ul>
        {#each arcs as arc, i (i)}
            <li
                class:active={hovered === i}
                onpointerenter={() => (hovered = i)}
                onpointerleave={() => (hovered = null)}
            >
                <span class="swatch" style:background={arc.color}></span>
                <span class="label">{arc.label}</span>
                <span class="value">{arc.value.toLocaleString()} · {percent(arc.share)}</span>
            </li>
        {/each}
    </ul>
</div>

<style>
    .pie {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 1rem;
    }

    svg {
        flex: 0 0 auto;
        width: 11rem;
        height: 11rem;
    }

    path {
        stroke: #5f4838;
        stroke-width: 0.75;
        transition: opacity 0.2s;
    }

    path.faded {
        opacity: 0.35;
    }

    ul {
        display: flex;
        flex: 1 1 10rem;
        flex-direction: column;
        gap: 0.2rem;
        min-width: 0;
        margin: 0;
        padding: 0;
        list-style: none;
        font-size: 0.85rem;
    }

    li {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.1rem 0.25rem;
        border-radius: 4px;
    }

    li.active {
        background: #4f2f1f;
    }

    .swatch {
        flex: 0 0 auto;
        width: 0.75rem;
        height: 0.75rem;
        border: 1px solid #3d3029;
        border-radius: 2px;
    }

    .label {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .value {
        color: #ead7c2;
        white-space: nowrap;
    }
</style>
