<script lang="ts">
    import { onMount } from "svelte";
    import type { LocationCountry } from "../types";
    import { country_name, dim, load_world, scale, small } from "./geo";

    let {
        countries,
        selected,
        onselect,
    }: {
        countries: Map<string, LocationCountry>;
        selected: string | null;
        onselect: (code: string) => void;
    } = $props();

    let canvas = $state<HTMLCanvasElement>();
    let hovered = $state<{ code: string; x: number; y: number } | null>(null);
    let failed = $state(false);

    const max = $derived(Math.max(0, ...[...countries.values()].map((c) => c.users)));
    const total = $derived([...countries.values()].reduce((sum, c) => sum + c.users, 0));
    const stats = $derived(hovered ? countries.get(hovered.code) : undefined);
    const rate = (part: number, whole: number) => `${Math.round((part / whole) * 100)}%`;

    onMount(() => {
        let frame = 0;
        let stopped = false;
        let cleanup = () => {};
        Promise.all([import("d3-geo"), load_world()]).then(([d3, world]) => {
            if (stopped || !canvas) return;
            const surface = canvas;
            const context = surface.getContext("2d")!;
            const projection = d3.geoOrthographic().clipAngle(90).precision(0.5);
            const path = d3.geoPath(projection, context);
            const sphere = { type: "Sphere" } as const;
            const graticule = d3.geoGraticule10();
            const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
            let rotation: [number, number] = [-20, -20];
            let target: [number, number] | null = null;
            let dragging: { x: number; y: number; from: [number, number] } | null = null;
            let moved = false;
            let resume = 0;
            let pointer: [number, number] | null = null;
            let size = 0;
            let last = performance.now();
            let focused: string | null = null;

            const resize = new ResizeObserver(() => {
                const ratio = devicePixelRatio || 1;
                size = surface.clientWidth;
                surface.width = size * ratio;
                surface.height = size * ratio;
                context.setTransform(ratio, 0, 0, ratio, 0, 0);
                projection.scale(size / 2 - 4).translate([size / 2, size / 2]);
            });
            resize.observe(surface);

            const at = (event: PointerEvent) => {
                const box = surface.getBoundingClientRect();
                return [event.clientX - box.left, event.clientY - box.top] as const;
            };

            const dots = () =>
                Object.entries(small).filter(([code]) => countries.has(code));
            const facing = (at: [number, number]) =>
                d3.geoDistance(at, [-rotation[0], -rotation[1]]) < Math.PI / 2;

            function find(x: number, y: number) {
                const point = projection.invert?.([x, y]);
                if (!point || Number.isNaN(point[0])) return null;
                const [cx, cy] = [size / 2, size / 2];
                if ((x - cx) ** 2 + (y - cy) ** 2 > (size / 2 - 4) ** 2) return null;
                for (const [code, at] of dots()) {
                    const [px, py] = projection(at)!;
                    if (facing(at) && (x - px) ** 2 + (y - py) ** 2 <= 8 ** 2) return code;
                }
                return world.find((country) => d3.geoContains(country, point))?.code ?? null;
            }

            function draw(now: number) {
                const dt = Math.min(100, now - last);
                last = now;
                if (selected !== focused) {
                    focused = selected;
                    const country = world.find((c) => c.code === selected);
                    const at = country ? d3.geoCentroid(country) : selected && small[selected];
                    if (at) target = [-at[0], Math.max(-60, Math.min(60, -at[1]))];
                }
                if (target && !dragging) {
                    const step = still ? 1 : Math.min(1, dt / 250);
                    const delta = ((target[0] - rotation[0] + 540) % 360) - 180;
                    rotation = [rotation[0] + delta * step, rotation[1] + (target[1] - rotation[1]) * step];
                    if (Math.abs(delta) < 0.1 && Math.abs(target[1] - rotation[1]) < 0.1) target = null;
                } else if (!dragging && now > resume) {
                    const ease = Math.min(1, (now - resume) / 1000);
                    rotation = [rotation[0] + dt * 0.006 * ease, rotation[1]];
                }
                projection.rotate(rotation);
                if (pointer && !dragging) {
                    const code = find(...pointer);
                    if (code !== hovered?.code)
                        hovered = code ? { code, x: pointer[0], y: pointer[1] } : null;
                }
                context.clearRect(0, 0, size, size);
                context.beginPath();
                path(sphere);
                context.fillStyle = "#3d3029";
                context.fill();
                context.beginPath();
                path(graticule);
                context.strokeStyle = "rgb(95 72 56 / 0.6)";
                context.lineWidth = 0.5;
                context.stroke();
                for (const country of world) {
                    const users = (country.code && countries.get(country.code)?.users) || 0;
                    context.beginPath();
                    path(country);
                    context.fillStyle = users ? dim(scale(users, max)) : "#4f2f1f";
                    context.fill();
                    context.strokeStyle = "#3d3029";
                    context.lineWidth = 0.5;
                    context.stroke();
                }
                for (const country of world.filter(
                    (c) => c.code && (c.code === hovered?.code || c.code === selected),
                )) {
                    context.beginPath();
                    path(country);
                    context.strokeStyle = "#ead7c2";
                    context.lineWidth = country.code === selected ? 2 : 1.25;
                    context.stroke();
                }
                for (const [code, at] of dots()) {
                    if (!facing(at)) continue;
                    const [px, py] = projection(at)!;
                    const lit = code === hovered?.code || code === selected;
                    context.beginPath();
                    context.arc(px, py, 4, 0, Math.PI * 2);
                    context.fillStyle = dim(scale(countries.get(code)!.users, max));
                    context.fill();
                    context.strokeStyle = lit ? "#ead7c2" : "#3d3029";
                    context.lineWidth = code === selected ? 2 : lit ? 1.25 : 1;
                    context.stroke();
                }
                context.beginPath();
                path(sphere);
                context.strokeStyle = "#4f2f1f";
                context.lineWidth = 2;
                context.stroke();
                frame = requestAnimationFrame(draw);
            }
            frame = requestAnimationFrame(draw);

            const down = (event: PointerEvent) => {
                const [x, y] = at(event);
                dragging = { x, y, from: rotation };
                moved = false;
                target = null;
                surface.setPointerCapture(event.pointerId);
            };
            const move = (event: PointerEvent) => {
                const [x, y] = at(event);
                pointer = [x, y];
                if (dragging) {
                    const [dx, dy] = [x - dragging.x, y - dragging.y];
                    if (Math.abs(dx) + Math.abs(dy) > 3) moved = true;
                    const speed = 180 / size;
                    rotation = [
                        dragging.from[0] + dx * speed,
                        Math.max(-70, Math.min(70, dragging.from[1] - dy * speed)),
                    ];
                }
                const code = dragging ? null : find(x, y);
                hovered = code ? { code, x, y } : null;
            };
            const up = (event: PointerEvent) => {
                const [x, y] = at(event);
                if (dragging && !moved) {
                    const code = find(x, y);
                    if (code && countries.get(code)) onselect(code);
                }
                if (dragging) resume = performance.now() + 3000;
                dragging = null;
            };
            const leave = () => {
                pointer = null;
                hovered = null;
            };
            surface.addEventListener("pointerdown", down);
            surface.addEventListener("pointermove", move);
            surface.addEventListener("pointerup", up);
            surface.addEventListener("pointercancel", up);
            surface.addEventListener("pointerleave", leave);
            cleanup = () => {
                resize.disconnect();
                surface.removeEventListener("pointerdown", down);
                surface.removeEventListener("pointermove", move);
                surface.removeEventListener("pointerup", up);
                surface.removeEventListener("pointercancel", up);
                surface.removeEventListener("pointerleave", leave);
            };
        }, () => (failed = true));
        return () => {
            stopped = true;
            cancelAnimationFrame(frame);
            cleanup();
        };
    });
</script>

<div class="globe">
    {#if failed}
        <p class="note">The globe couldn't load. Reload to try again.</p>
    {:else}
        <canvas bind:this={canvas}>
            Spinning globe shading each country by how many people are from there. The country list has the same numbers.
        </canvas>
        {#if hovered}
            <div
                class="tip"
                class:start={hovered.x < 110}
                class:end={canvas && hovered.x > canvas.clientWidth - 110}
                class:below={hovered.y < 150}
                style:left="{hovered.x}px"
                style:top="{hovered.y}px"
            >
                <strong>{country_name(hovered.code)}</strong>
                {#if stats}
                    <dl>
                        <dt>People</dt>
                        <dd>{stats.users.toLocaleString()} · {rate(stats.users, total)}</dd>
                        <dt>Onboarded</dt>
                        <dd>{rate(stats.onboarded, stats.users)}</dd>
                        <dt>Shipped</dt>
                        <dd>{rate(stats.shipped, stats.users)}</dd>
                        <dt>Hours per person</dt>
                        <dd>{(stats.hours / stats.users).toFixed(1)}</dd>
                        <dt>Mips earned</dt>
                        <dd>{stats.mips.toLocaleString()}</dd>
                    </dl>
                {:else}
                    <span class="none">Nobody yet</span>
                {/if}
            </div>
        {/if}
    {/if}
</div>

<style>
    .globe {
        position: relative;
        width: 100%;
        max-width: 26rem;
        margin: 0 auto;
    }

    canvas {
        display: block;
        width: 100%;
        aspect-ratio: 1;
        cursor: grab;
        touch-action: none;
    }

    canvas:active {
        cursor: grabbing;
    }

    .tip {
        position: absolute;
        z-index: 1;
        display: flex;
        flex-direction: column;
        gap: 0.35rem;
        padding: 0.6rem 0.75rem;
        background: #3d3029;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        font-size: 0.8rem;
        white-space: nowrap;
        pointer-events: none;
        --x: -50%;
        --y: calc(-100% - 0.75rem);
        transform: translate(var(--x), var(--y));
    }

    .tip.start {
        --x: -0.5rem;
    }

    .tip.end {
        --x: calc(-100% + 0.5rem);
    }

    .tip.below {
        --y: 1.25rem;
    }

    dl {
        display: grid;
        grid-template-columns: auto auto;
        gap: 0.15rem 1rem;
        margin: 0;
    }

    dt {
        color: #d6ab70;
    }

    dd {
        margin: 0;
        text-align: right;
    }

    .none {
        color: #d6ab70;
    }

    .note {
        margin: 0;
        color: #d6ab70;
        font-size: 0.85rem;
    }
</style>
