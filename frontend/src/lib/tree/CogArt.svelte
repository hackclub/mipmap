<script lang="ts">
    import type { NodeKind } from "$lib/tree/types";
    import { cog_path, cog_inner_radius } from "$lib/tree/style";

    let {
        kind,
        color,
        size,
        solid = false,
        outline,
    }: {
        kind: NodeKind;
        color: string;
        size: number;
        solid?: boolean;
        outline?: { color: string; width: number };
    } = $props();

    const uid = $props.id();
    const units = $derived(800 / size);
    const darker = $derived(`color-mix(in srgb, ${color} 72%, black)`);
    const body_radius = 340;
    const line = 24;
    const coin_inner = cog_inner_radius * 0.55;
    const checker = cog_inner_radius / 3;
</script>

<svg
    width={size}
    height={size}
    viewBox="0 0 800 800"
    overflow="visible"
    aria-hidden="true"
>
    {#if outline}
        <path
            d={cog_path}
            fill={outline.color}
            stroke={outline.color}
            stroke-width={outline.width * 2 * units}
            stroke-linejoin="round"
        />
    {:else}
        <path d={cog_path} fill={color} />
        {#if kind !== "origin" && !solid}
            <circle
                cx="400"
                cy="400"
                r={body_radius * 0.7}
                style:fill={darker}
            />
        {/if}
        {#if kind === "token"}
            <g
                fill="none"
                stroke={color}
                stroke-width={line}
                stroke-linecap="round"
            >
                <circle cx="400" cy="400" r={cog_inner_radius * 0.8} />
                <circle cx="400" cy="400" r={coin_inner} />
                <line
                    x1="400"
                    y1={400 - coin_inner * 0.8}
                    x2="400"
                    y2={400 + coin_inner * 0.8}
                />
            </g>
        {:else if kind === "origin"}
            <defs>
                <pattern
                    id="{uid}-checker"
                    width={checker * 2}
                    height={checker * 2}
                    patternUnits="userSpaceOnUse"
                    x="400"
                    y="400"
                >
                    <rect
                        width={checker * 2}
                        height={checker * 2}
                        fill="#4f2f1f"
                    />
                    <rect width={checker} height={checker} fill="#5f4838" />
                    <rect
                        x={checker}
                        y={checker}
                        width={checker}
                        height={checker}
                        fill="#5f4838"
                    />
                </pattern>
            </defs>
            <circle
                cx="400"
                cy="400"
                r={cog_inner_radius}
                fill="url(#{uid}-checker)"
            />
        {/if}
    {/if}
</svg>

<style>
    svg {
        display: block;
    }
</style>
