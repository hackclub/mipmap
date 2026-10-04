<script lang="ts">
    import type { Snippet } from "svelte";
    import type { Bounds } from "$lib/dashboard/PanZoom.svelte";

    let {
        title,
        bounds,
        zoom = 1,
        padded = false,
        actions,
        children,
    }: {
        title: string;
        bounds: Bounds;
        zoom?: number;
        padded?: boolean;
        actions?: Snippet;
        children: Snippet;
    } = $props();

    const uid = $props.id();
</script>

<section
    class="section"
    aria-labelledby={uid}
    style:left="{bounds.left}px"
    style:top="{bounds.top}px"
    style:width="{bounds.width}px"
    style:height="{bounds.height}px"
>
    <header>
        <h2 id={uid}>{title}</h2>
        {@render actions?.()}
    </header>
    <div class="body" class:padded data-scroll style:zoom>
        {@render children()}
    </div>
</section>

<style>
    .section {
        position: absolute;
        display: flex;
        flex-direction: column;
        box-sizing: border-box;
        overflow: hidden;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
        line-height: 1.35;
    }

    header {
        display: flex;
        align-items: stretch;
        justify-content: space-between;
        border-bottom: 2px solid #4f2f1f;
    }

    h2 {
        margin: 0;
        padding: 1.5rem 2rem;
        font: inherit;
        font-size: 8rem;
        font-weight: 700;
        line-height: 1.15;
    }

    header > :global(button) {
        width: 12rem;
        padding: 0;
        background: none;
        border: none;
        border-left: 2px solid #4f2f1f;
        color: #ead7c2;
        font: inherit;
        font-size: 8rem;
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    header > :global(button:hover),
    header > :global(button:focus-visible) {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    .body {
        flex: 1;
        min-height: 0;
        overflow: auto;
    }

    .padded {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        padding: 1.5rem;
    }

    .padded :global(p) {
        margin: 0;
        font-size: 0.85em;
    }
</style>
