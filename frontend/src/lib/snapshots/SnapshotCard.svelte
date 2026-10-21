<script lang="ts">
    import type { Snapshot } from "$lib/snapshots/types";
    import Dialog from "$lib/ui/Dialog.svelte";

    let { snapshot }: { snapshot: Snapshot } = $props();

    const uid = $props.id();
    let long = $state(false);
    let open = $state(false);

    function measure(text: HTMLElement) {
        const check = () => {
            const line = parseFloat(getComputedStyle(text).lineHeight);
            long = text.scrollHeight > line * 3 + 1;
        };
        const observer = new ResizeObserver(check);
        observer.observe(text);
        return () => observer.disconnect();
    }
</script>

<figure>
    <button
        type="button"
        class="open"
        aria-label="Open Snapshot by {snapshot.author}"
        onclick={() => (open = true)}
    >
        <img src={snapshot.image} alt="" loading="lazy" />
    </button>
    <figcaption>
        <p class:long {@attach measure}>
            {snapshot.caption}
        </p>
        {#if long}
            <button type="button" onclick={() => (open = true)}>Read more</button>
        {/if}
        <span class="author">- {snapshot.author}</span>
    </figcaption>
</figure>

{#if open}
    <Dialog labelledby="{uid}-title" onclose={() => (open = false)}>
        <h2 id="{uid}-title">Snapshot by {snapshot.author}</h2>
        <img class="full" src={snapshot.image} alt="" />
        <p class="full">{snapshot.caption}</p>
    </Dialog>
{/if}

<style>
    figure {
        display: flex;
        flex-direction: column;
        min-width: 0;
        margin: 0;
    }

    .open {
        display: block;
        padding: 0;
        background: none;
        border: none;
    }

    img {
        display: block;
        box-sizing: border-box;
        width: 100%;
        aspect-ratio: 4 / 3;
        object-fit: cover;
        background: #4f2f1f;
        border: 2px solid #4f2f1f;
        border-radius: 7px 7px 0 0;
    }

    figcaption {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        height: 5.6em;
        padding: 0.5rem 0.75rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-top: none;
        border-radius: 0 0 7px 7px;
        font-size: 0.85em;
        line-height: 1.4;
    }

    figcaption p {
        display: -webkit-box;
        -webkit-box-orient: vertical;
        -webkit-line-clamp: 3;
        line-clamp: 3;
        overflow: hidden;
        margin: 0;
        overflow-wrap: anywhere;
    }

    figcaption p.long {
        -webkit-line-clamp: 2;
        line-clamp: 2;
    }

    .author {
        margin-top: auto;
        max-width: 100%;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    figcaption button {
        padding: 0;
        background: none;
        border: none;
        color: #a2b8eb;
        font: inherit;
        text-decoration: underline;
        cursor: pointer;
    }

    h2 {
        margin: 0 0 1rem;
    }

    img.full {
        aspect-ratio: auto;
        max-height: 60vh;
        object-fit: contain;
        border-radius: 7px;
    }

    p.full {
        margin: 1rem 0 0;
        white-space: pre-wrap;
        overflow-wrap: anywhere;
    }
</style>
