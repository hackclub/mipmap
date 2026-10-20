<script lang="ts">
    import { short_date } from "$lib/format";

    let {
        weeks,
        series,
    }: {
        weeks: string[];
        series: { label: string; values: number[] }[];
    } = $props();

    const styles = [
        { color: "#ead7c2", dash: "" },
        { color: "#d6ab70", dash: "" },
        { color: "#ead7c2", dash: "4 3" },
        { color: "#d6ab70", dash: "4 3" },
        { color: "#e8c58c", dash: "1 3" },
    ];

    const max = $derived(Math.max(1, ...series.flatMap((s) => s.values)));
    const x = (i: number) => (weeks.length < 2 ? 50 : (i / (weeks.length - 1)) * 100);
    const y = (value: number) => 100 - (value / max) * 100;

    let hovered = $state<number | null>(null);

    function hover(event: PointerEvent) {
        const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
        const ratio = Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
        hovered = Math.round(ratio * (weeks.length - 1));
    }
</script>

<div class="growth">
    <div class="axis" aria-hidden="true">
        <span>{max.toLocaleString()}</span>
        <span>0</span>
    </div>
    <div
        class="plot"
        role="presentation"
        onpointermove={hover}
        onpointerdown={hover}
        onpointerleave={() => (hovered = null)}
    >
        <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
            {#each [0, 25, 50, 75, 100] as line (line)}
                <line class="grid" x1="0" x2="100" y1={line} y2={line} />
            {/each}
            {#each series as s, i (s.label)}
                <polyline
                    points={s.values.map((v, w) => `${x(w)},${y(v)}`).join(" ")}
                    stroke={styles[i % styles.length].color}
                    stroke-dasharray={styles[i % styles.length].dash}
                />
            {/each}
            {#if hovered !== null}
                <line class="cursor" x1={x(hovered)} x2={x(hovered)} y1="0" y2="100" />
            {/if}
        </svg>
        {#if hovered !== null}
            <div class="tip" class:left={x(hovered) > 60} style:left="{x(hovered)}%">
                <strong>Week of {short_date(weeks[hovered])}</strong>
                {#each series as s (s.label)}
                    <span>{s.label}: {s.values[hovered].toLocaleString()}</span>
                {/each}
            </div>
        {/if}
    </div>
    <div class="dates" aria-hidden="true">
        <span>{weeks.length ? short_date(weeks[0]) : ""}</span>
        <span>{weeks.length ? short_date(weeks[weeks.length - 1]) : ""}</span>
    </div>
    <ul class="legend">
        {#each series as s, i (s.label)}
            <li>
                <svg viewBox="0 0 20 4" aria-hidden="true">
                    <line
                        x1="0"
                        x2="20"
                        y1="2"
                        y2="2"
                        stroke={styles[i % styles.length].color}
                        stroke-dasharray={styles[i % styles.length].dash}
                    />
                </svg>
                {s.label} · {(s.values.at(-1) ?? 0).toLocaleString()}
            </li>
        {/each}
    </ul>
</div>

<style>
    .growth {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        gap: 0.25rem 0.5rem;
    }

    .axis {
        display: flex;
        flex-direction: column;
        justify-content: space-between;
        color: #d6ab70;
        font-size: 0.75rem;
        text-align: right;
    }

    .plot {
        position: relative;
        height: 12rem;
        touch-action: pan-y;
    }

    .plot > svg {
        width: 100%;
        height: 100%;
        overflow: visible;
    }

    polyline {
        fill: none;
        stroke-width: 2;
        vector-effect: non-scaling-stroke;
        stroke-linejoin: round;
    }

    .grid {
        stroke: #4f2f1f;
        stroke-width: 1;
        vector-effect: non-scaling-stroke;
    }

    .cursor {
        stroke: #ead7c2;
        stroke-width: 1;
        opacity: 0.5;
        vector-effect: non-scaling-stroke;
    }

    .tip {
        position: absolute;
        top: 0;
        z-index: 1;
        display: flex;
        flex-direction: column;
        padding: 0.35rem 0.6rem;
        background: #3d3029;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        font-size: 0.8rem;
        white-space: nowrap;
        pointer-events: none;
        transform: translateX(0.75rem);
    }

    .tip.left {
        transform: translateX(calc(-100% - 0.75rem));
    }

    .dates {
        grid-column: 2;
        display: flex;
        justify-content: space-between;
        color: #d6ab70;
        font-size: 0.75rem;
    }

    .legend {
        grid-column: 1 / -1;
        display: flex;
        flex-wrap: wrap;
        gap: 0.25rem 1rem;
        margin: 0.25rem 0 0;
        padding: 0;
        list-style: none;
        font-size: 0.85rem;
    }

    .legend li {
        display: flex;
        align-items: center;
        gap: 0.4rem;
    }

    .legend svg {
        width: 1.5rem;
        height: 0.4rem;
    }

    .legend line {
        stroke-width: 2.5;
    }
</style>
