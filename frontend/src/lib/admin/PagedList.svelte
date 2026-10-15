<script lang="ts" generics="T extends { id: number }">
    import type { Snippet } from "svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import type { Pages } from "./pages.svelte";

    let {
        pages,
        empty,
        item,
    }: { pages: Pages<T>; empty: string; item: Snippet<[T]> } = $props();
</script>

<div class="list">
    {#each pages.items as entry (entry.id)}
        {@render item(entry)}
    {/each}
</div>
{#if pages.error}
    <LoadError error={pages.error} onretry={pages.more} />
{:else if pages.loading}
    <p class="note">Loading…</p>
{:else if !pages.items.length}
    <p class="note">{empty}</p>
{:else if !pages.done}
    <button type="button" class="more" onclick={pages.more}>Load more</button>
{/if}

<style>
    .list {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .note {
        margin: 1rem 0 0;
        color: #d6ab70;
    }

    .more {
        align-self: center;
        margin-top: 1rem;
        padding: 0.5rem 1rem;
        background-color: #4f2f1f;
        border: none;
        color: #ead7c2;
        font: inherit;
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .more:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .more:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }
</style>
