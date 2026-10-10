<script lang="ts">
    let {
        values,
        start,
        end,
    }: { values: number[]; start: string; end: string } = $props();

    const width = 100;
    const height = 100;

    const ticks = $derived.by(() => {
        const max = Math.max(1, ...values);
        const raw = max / 4;
        const magnitude = 10 ** Math.floor(Math.log10(raw));
        const step = Math.max(
            1,
            ([1, 2, 5, 10].find((m) => m * magnitude >= raw) ?? 10) *
                magnitude,
        );
        const top = Math.ceil(max / step) * step;
        return Array.from({ length: top / step + 1 }, (_, i) => i * step);
    });
    const top = $derived(ticks[ticks.length - 1]);

    const points = $derived.by(() => {
        const last = values.length - 1;
        return values.map((v, i) => ({
            x: last === 0 ? 0 : (i / last) * width,
            y: height - (v / top) * height,
        }));
    });
    const line = $derived(
        (points.length === 1
            ? [points[0], { x: width, y: points[0].y }]
            : points
        )
            .map((p) => `${p.x},${p.y}`)
            .join(" "),
    );
</script>

<div class="chart">
    <div class="axis" aria-hidden="true">
        {#each ticks as tick (tick)}
            <span style:bottom="{(tick / top) * 100}%"
                >{tick.toLocaleString()}</span
            >
        {/each}
        <span class="sizer">{top.toLocaleString()}</span>
    </div>
    <div class="plot">
    <svg
        viewBox="0 0 {width} {height}"
        preserveAspectRatio="none"
        aria-hidden="true"
    >
        <polygon class="area" points="0,{height} {line} {width},{height}" />
        {#each ticks as tick (tick)}
            {@const y = height - (tick / top) * height}
            <line class="grid" x1="0" x2={width} y1={y} y2={y} />
        {/each}
        <polyline class="line" points={line} />
    </svg>
    </div>
    <div class="dates">
        <span>{start}</span>
        <span>{end}</span>
    </div>
</div>

<style>
    .chart {
        flex: 1;
        display: grid;
        grid-template-columns: max-content 1fr;
        grid-template-rows: minmax(7rem, 1fr) auto;
        column-gap: 0.5rem;
    }

    .axis {
        position: relative;
        color: #d6ab70;
        font-size: 0.75rem;
        font-variant-numeric: tabular-nums;
        text-align: right;
    }

    .axis span {
        position: absolute;
        right: 0;
        transform: translateY(50%);
        line-height: 1;
    }

    .axis .sizer {
        position: static;
        visibility: hidden;
    }

    .plot {
        position: relative;
    }

    svg {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        overflow: visible;
    }

    .grid {
        stroke: #4f2f1f;
        stroke-width: 1;
        vector-effect: non-scaling-stroke;
    }

    .area {
        fill: #5f4838;
    }

    .line {
        fill: none;
        stroke: #d6ab70;
        stroke-width: 2;
        stroke-linejoin: round;
        stroke-linecap: round;
        vector-effect: non-scaling-stroke;
    }

    .dates {
        grid-column: 2;
        display: flex;
        justify-content: space-between;
        margin-top: 0.25rem;
        color: #d6ab70;
        font-size: 0.8rem;
    }
</style>
