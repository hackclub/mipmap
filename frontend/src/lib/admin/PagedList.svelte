<script lang="ts" generics="T extends { id: number }">
    import { tick, type Snippet } from "svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import type { Pages } from "./pages.svelte";

    let {
        pages,
        empty,
        item,
    }: { pages: Pages<T>; empty: string; item: Snippet<[T]> } = $props();

    let root = $state<HTMLElement>();
    let list = $state<HTMLElement>();

    const entries = () => [...(list?.children ?? [])] as HTMLElement[];

    export function focus_item(id: number) {
        list?.querySelector<HTMLElement>(`[data-id="${id}"]`)?.focus();
    }

    async function more() {
        if (pages.loading) return;
        const count = pages.items.length;
        await pages.more();
        await tick();
        if (pages.items.length > count) entries()[count]?.focus();
    }

    $effect(() => {
        const removed = pages.removed;
        if (!removed) return;
        let frame = 0;
        const started = performance.now();
        const settle = () => {
            const active = document.activeElement;
            if (
                active?.closest('[role="dialog"]') &&
                performance.now() - started < 2000
            ) {
                frame = requestAnimationFrame(settle);
                return;
            }
            if (
                !active ||
                active === document.body ||
                !active.isConnected ||
                active === root
            ) {
                const all = entries();
                (all[removed.index] ?? all[removed.index - 1] ?? root)?.focus();
            }
        };
        frame = requestAnimationFrame(settle);
        return () => cancelAnimationFrame(frame);
    });

    const status = $derived(
        pages.loading && !pages.items.length
            ? "Loading…"
            : !pages.loading && !pages.error && !pages.items.length
              ? empty
              : "",
    );
</script>

<div class="paged" tabindex="-1" bind:this={root}>
    <div class="list" bind:this={list}>
        {#each pages.items as entry (entry.id)}
            <div class="entry" data-id={entry.id} tabindex="-1">
                {@render item(entry)}
            </div>
        {/each}
    </div>
    {#if pages.error}
        <LoadError error={pages.error} onretry={pages.more} />
    {/if}
    <div role="status">
        {#if status}<p class="note">{status}</p>{/if}
        {#if pages.items.length && !pages.loading}<span class="sr"
                >{pages.items.length} shown{pages.done ? "" : ", more available"}</span
            >{/if}
    </div>
    {#if pages.items.length && !pages.done && !pages.error}
        <button
            type="button"
            class="more"
            aria-disabled={pages.loading}
            onclick={more}>{pages.loading ? "Loading…" : "Load more"}</button
        >
    {/if}
</div>

<style>
    .paged {
        display: flex;
        flex-direction: column;
    }

    .paged:focus {
        outline: none;
    }

    .list {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .entry {
        border-radius: 7px;
    }

    .entry:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: 2px;
    }

    .note {
        margin: 1rem 0 0;
        color: #ead7c2;
    }

    .sr {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
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

    .more[aria-disabled="true"] {
        opacity: 0.6;
        cursor: default;
    }

    .more:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .more:not([aria-disabled="true"]):hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }
</style>
