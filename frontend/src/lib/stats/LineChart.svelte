<script lang="ts">
    let {
        values,
        labels,
        name,
    }: { values: number[]; labels: string[]; name: string } = $props();

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

    const last = $derived(values.length - 1);
    const summary = $derived.by(() => {
        if (!values.length) return "";
        const min = Math.min(...values);
        const max = Math.max(...values);
        return `${name}, ${labels[0]} to ${labels[last]}: lowest ${min.toLocaleString()}, highest ${max.toLocaleString()}, latest ${values[last].toLocaleString()}.`;
    });
    const samples = $derived.by(() => {
        const count = Math.min(values.length, 8);
        if (count < 2) return values.length ? [last] : [];
        return [
            ...new Set(
                Array.from({ length: count }, (_, i) =>
                    Math.round((i / (count - 1)) * last),
                ),
            ),
        ];
    });

    let plot = $state<HTMLElement>();
    let hovered = $state<number>();
    const point = $derived(hovered === undefined ? undefined : points[hovered]);
    const current = $derived(
        Math.min(Math.max(hovered ?? last, 0), Math.max(last, 0)),
    );

    function hover(event: PointerEvent) {
        if (!values.length) return;
        const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
        const ratio = Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
        hovered = Math.round(ratio * last);
    }

    function leave(event: PointerEvent) {
        if (event.pointerType !== "touch") hovered = undefined;
    }

    function key(event: KeyboardEvent) {
        if (!values.length) return;
        const page = Math.max(1, Math.round(values.length / 10));
        const moves: Record<string, number> = {
            ArrowLeft: current - 1,
            ArrowDown: current - 1,
            ArrowRight: current + 1,
            ArrowUp: current + 1,
            PageDown: current - page,
            PageUp: current + page,
            Home: 0,
            End: last,
        };
        if (event.key === "Escape") {
            if (hovered === undefined) return;
            hovered = undefined;
        } else if (event.key in moves) {
            hovered = Math.min(last, Math.max(0, moves[event.key]));
        } else return;
        event.preventDefault();
        event.stopPropagation();
    }

    function dismiss(event: KeyboardEvent) {
        if (event.key === "Escape" && hovered !== undefined) hovered = undefined;
    }

    function outside(event: PointerEvent) {
        if (hovered !== undefined && !plot?.contains(event.target as Node))
            hovered = undefined;
    }
</script>

<svelte:window onkeydown={dismiss} onpointerdown={outside} />

<div class="chart">
    {#if values.length}
        <div class="axis" aria-hidden="true">
            {#each ticks as tick (tick)}
                <span style:bottom="{(tick / top) * 100}%"
                    >{tick.toLocaleString()}</span
                >
            {/each}
            <span class="sizer">{top.toLocaleString()}</span>
        </div>
        <div
            class="plot"
            bind:this={plot}
            role="slider"
            tabindex="0"
            aria-label="{name} chart"
            aria-valuemin={0}
            aria-valuemax={last}
            aria-valuenow={current}
            aria-valuetext="{labels[current]}: {values[current].toLocaleString()}"
            onpointermove={hover}
            onpointerdown={hover}
            onpointerleave={leave}
            onkeydown={key}
            onblur={() => (hovered = undefined)}
        >
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
            {#if hovered !== undefined && point}
                <div class="cursor" style:left="{point.x}%" aria-hidden="true"></div>
                <div
                    class="dot"
                    style:left="{point.x}%"
                    style:top="{point.y}%"
                    aria-hidden="true"
                ></div>
                <div
                    class="tip"
                    class:low={point.y < 50}
                    style:left="{point.x}%"
                    style:transform="translateX(-{point.x}%)"
                    aria-hidden="true"
                >
                    <strong>{values[hovered].toLocaleString()}</strong>
                    <span>{labels[hovered]}</span>
                </div>
            {/if}
        </div>
        <div class="dates" aria-hidden="true">
            <span>{labels[0]}</span>
            <span>{labels[last]}</span>
        </div>
        <div class="sr-only">
            <p>{summary}</p>
            <table>
                <caption>{name}, sample points</caption>
                <thead>
                    <tr><th scope="col">Time</th><th scope="col">Value</th></tr>
                </thead>
                <tbody>
                    {#each samples as i (i)}
                        <tr><td>{labels[i]}</td><td>{values[i].toLocaleString()}</td></tr>
                    {/each}
                </tbody>
            </table>
        </div>
    {:else}
        <p class="empty">No data yet</p>
    {/if}
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
        color: #ead7c2;
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
        touch-action: pan-y;
        border-radius: 2px;
    }

    .plot:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: 4px;
    }

    .cursor {
        position: absolute;
        top: 0;
        bottom: 0;
        width: 1px;
        background: #ead7c2;
        opacity: 0.6;
        pointer-events: none;
    }

    .dot {
        position: absolute;
        box-sizing: border-box;
        width: 0.85rem;
        height: 0.85rem;
        background: #ead7c2;
        border: 3px solid #4f2f1f;
        border-radius: 50%;
        transform: translate(-50%, -50%);
        pointer-events: none;
    }

    .tip {
        position: absolute;
        top: 0;
        display: flex;
        flex-direction: column;
        padding: 0.3rem 0.5rem;
        background: #3d3029;
        border: 2px solid #4f2f1f;
        border-radius: 5px;
        white-space: nowrap;
        font-size: 0.8rem;
        line-height: 1.3;
        pointer-events: none;
    }

    .tip.low {
        top: auto;
        bottom: 0;
    }

    .tip strong {
        color: #ead7c2;
        font-size: 1rem;
    }

    .tip span {
        color: #d6ab70;
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
        color: #ead7c2;
        font-size: 0.8rem;
    }

    .empty {
        grid-column: 1 / -1;
        align-self: center;
        margin: 0;
        text-align: center;
    }

    .sr-only {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }
</style>
