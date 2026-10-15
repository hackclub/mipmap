<script lang="ts">
    import type { Snippet } from "svelte";

    let {
        title,
        meta,
        dim = false,
        children,
        actions,
    }: {
        title: Snippet;
        meta?: Snippet;
        dim?: boolean;
        children?: Snippet;
        actions?: Snippet;
    } = $props();
</script>

<article class:dim>
    <header>
        <div class="title">{@render title()}</div>
        {#if meta}<div class="meta">{@render meta()}</div>{/if}
    </header>
    {@render children?.()}
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
</article>

<style>
    article {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        min-width: 0;
        padding: 1rem;
        background: #5f4838;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        overflow-wrap: anywhere;
    }

    article.dim {
        background: #3d3029;
    }

    header {
        display: flex;
        flex-wrap: wrap;
        justify-content: space-between;
        align-items: baseline;
        gap: 0.25rem 1rem;
    }

    .title {
        font-weight: 700;
    }

    .meta {
        color: #d6ab70;
        font-size: 0.85rem;
    }

    .actions {
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem;
    }

    .actions :global(button) {
        padding: 0.4rem 0.8rem;
        background-color: #4f2f1f;
        border: none;
        color: #ead7c2;
        font: inherit;
        font-size: 0.85rem;
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .actions :global(button:focus-visible) {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .actions :global(button:hover) {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    article :global(dl) {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));
        gap: 0.5rem 1rem;
        margin: 0;
    }

    article :global(dt) {
        color: #d6ab70;
        font-size: 0.75rem;
    }

    article :global(dd) {
        margin: 0;
        font-size: 0.9rem;
    }

    article :global(.links) {
        display: flex;
        flex-wrap: wrap;
        gap: 0.25rem 1rem;
        font-size: 0.9rem;
    }
</style>
