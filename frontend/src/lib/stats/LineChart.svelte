<script lang="ts">
    import { short_date } from "$lib/format";
    import type { Day } from "./types";

    let { days, value }: { days: Day[]; value: (day: Day) => number } =
        $props();

    const width = 100;
    const height = 100;

    const points = $derived.by(() => {
        const values = days.map(value);
        const top = Math.max(1, ...values) * 1.1;
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
    <svg
        viewBox="0 0 {width} {height}"
        preserveAspectRatio="none"
        aria-hidden="true"
    >
        <polygon class="area" points="0,{height} {line} {width},{height}" />
        <polyline class="line" points={line} />
    </svg>
    <div class="dates">
        <span>{short_date(days[0].date)}</span>
        <span>{short_date(days[days.length - 1].date)}</span>
    </div>
</div>

<style>
    svg {
        display: block;
        width: 100%;
        height: 8rem;
        overflow: visible;
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
        display: flex;
        justify-content: space-between;
        margin-top: 0.25rem;
        color: #d6ab70;
        font-size: 0.8rem;
    }
</style>
