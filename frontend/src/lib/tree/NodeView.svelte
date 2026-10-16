<script lang="ts">
    import CogArt from "$lib/tree/CogArt.svelte";
    import { node_color, price_label, unlocked_border } from "$lib/tree/style";
    import type { Shape, TreeNode } from "$lib/tree/types";

    let {
        node,
        title,
        shape,
        x,
        y,
        currency,
        unlocked,
        purchased,
        onselect,
    }: {
        node: TreeNode;
        title: string;
        shape: Shape;
        x: number;
        y: number;
        currency: string;
        unlocked: boolean;
        purchased: boolean;
        onselect: () => void;
    } = $props();

    const border = $derived(
        unlocked && !purchased && node.kind === "item" ? 6 : 3,
    );
    const color = $derived(node_color(node));
    const price = $derived(price_label(node, currency, unlocked, purchased));
    const unlocked_ring = $derived({ color: unlocked_border, width: border });
    const focus_ring = $derived({ color: "#ead7c2", width: border + 8 });
</script>

{#snippet cog(ring?: { color: string; width: number })}
    <div
        class="cog"
        class:gray={unlocked && !purchased && !ring}
        style:left="{-shape.cog / 2}px"
        style:top="{-shape.cog / 2}px"
    >
        <CogArt
            kind={node.kind}
            color={purchased ? unlocked_border : color}
            size={shape.cog}
            solid={purchased}
            outline={ring}
        />
    </div>
{/snippet}

{#snippet label(ring?: { color: string; width: number })}
    <div
        class="label"
        class:outline={ring}
        class:gray={unlocked && !purchased && !ring}
        class:purchased={purchased && !ring}
        class:token={node.kind === "token"}
        style:left="{-shape.label_width / 2}px"
        style:top="{shape.label_offset - shape.label_height / 2}px"
        style:width="{shape.label_width}px"
        style:height="{shape.label_height}px"
        style:--border="{ring?.width}px"
        style:--border-color={ring?.color}
    >
        {#if !ring}
            {#if node.kind === "item"}
                <span class="title">{title}</span>
            {/if}
            <span class="price">{price}</span>
        {/if}
    </div>
{/snippet}

{#if node.kind === "origin"}
    <div
        class="node"
        style:left="{x}px"
        style:top="{y}px"
        data-node={node.id}
    >
        {@render cog()}
    </div>
{:else}
    <button
        type="button"
        class="node interactive"
        data-node={node.id}
        style:left="{x}px"
        style:top="{y}px"
        aria-label="{title}, {price}"
        onclick={onselect}
    >
        <div class="focus">
            {@render cog(focus_ring)}
            {@render label(focus_ring)}
        </div>
        {#if unlocked && !purchased}
            {@render cog(unlocked_ring)}
            {@render label(unlocked_ring)}
        {/if}
        {@render cog()}
        {@render label()}
    </button>
{/if}

<style>
    .node {
        position: absolute;
        width: 0;
        height: 0;
        padding: 0;
        border: none;
        background: none;
        font: inherit;
        color: inherit;
    }

    .interactive {
        cursor: pointer;
    }

    .interactive:focus-visible {
        outline: none;
    }

    .focus {
        display: none;
    }

    .interactive:focus-visible .focus {
        display: contents;
    }

    .cog,
    .label {
        position: absolute;
        transition: transform 0.3s;
    }

    .interactive:focus-visible .cog {
        transform: rotate(20deg);
    }

    @media (hover: hover) {
        .interactive:hover .cog {
            transform: rotate(20deg);
        }
    }

    .interactive:focus-visible .label {
        transform: scale(1.06);
    }

    @media (hover: hover) {
        .interactive:hover .label {
            transform: scale(1.06);
        }
    }

    .label {
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 0.4rem 1.2rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font-size: 2rem;
        line-height: 1.15;
        text-align: center;
    }

    .label.outline {
        background: var(--border-color);
        border-color: var(--border-color);
        box-shadow: 0 0 0 var(--border) var(--border-color);
    }

    .label.purchased {
        background: #d6ab70;
        border-color: #d6ab70;
        color: #4f2f1f;
    }

    .label.token {
        padding: 0;
        font-size: 1.7rem;
        font-weight: 700;
    }

    .title {
        display: -webkit-box;
        -webkit-box-orient: vertical;
        -webkit-line-clamp: 2;
        line-clamp: 2;
        overflow: hidden;
        font-weight: 700;
        text-wrap: balance;
    }

    .label:not(.token) .price {
        font-size: 0.85em;
        line-height: 1.35;
    }

    .gray {
        filter: grayscale(1);
    }
</style>
