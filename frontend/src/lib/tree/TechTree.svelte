<script lang="ts">
    import NodeView from "$lib/tree/NodeView.svelte";
    import type { ItemMetadata, Layout, TreeNode } from "$lib/tree/types";

    let {
        nodes,
        layout,
        currency,
        metadata,
        unlocked,
        purchased,
        left = 0,
        onselect,
    }: {
        nodes: Map<number, TreeNode>;
        layout: Layout;
        currency: string;
        metadata: ItemMetadata;
        unlocked: Set<number>;
        purchased: Set<number>;
        left?: number;
        onselect: (id: number) => void;
    } = $props();

    const uid = $props.id();
    const arrow = 20;
    const ordered = $derived(
        [...layout.nodes].sort((a, b) => b.y - a.y || a.x - b.x),
    );
</script>

<div
    class="tree"
    style:left="{left}px"
    style:width="{layout.width}px"
    style:height="{layout.height}px"
>
    <svg
        class="edges"
        width={layout.width}
        height={layout.height}
        aria-hidden="true"
    >
        <defs>
            <marker
                id="{uid}-arrow"
                markerUnits="userSpaceOnUse"
                markerWidth={arrow}
                markerHeight={arrow}
                refX={arrow - 2}
                refY={arrow / 2}
                orient="auto"
            >
                <path d="M0 0 L{arrow} {arrow / 2} L0 {arrow} Z" />
            </marker>
        </defs>
        {#each layout.edges as edge (`${edge.from}-${edge.to}`)}
            <path d={edge.path} marker-end="url(#{uid}-arrow)" />
        {/each}
    </svg>
    {#each ordered as placed (placed.id)}
        {@const node = nodes.get(placed.id)}
        {#if node}
            <NodeView
                {node}
                title={metadata[node.id]?.title ?? node.name}
                shape={layout.shapes[node.kind]}
                x={placed.x}
                y={placed.y}
                {currency}
                unlocked={unlocked.has(node.id)}
                purchased={purchased.has(node.id)}
                onselect={() => onselect(node.id)}
            />
        {/if}
    {/each}
</div>

<style>
    .tree {
        position: absolute;
        top: 0;
        z-index: -1;
    }

    .edges {
        position: absolute;
        inset: 0;
        overflow: visible;
        opacity: 0.6;
    }

    .edges path {
        fill: none;
        stroke: #ead7c2;
        stroke-width: 4;
    }

    .edges marker path {
        fill: #ead7c2;
        stroke: none;
    }
</style>
