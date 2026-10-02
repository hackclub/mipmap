<script lang="ts">
    import type { Snapshot } from "$lib/snapshots/types";

    let { snapshot }: { snapshot: Snapshot } = $props();
</script>

<figure>
    <img src={snapshot.image} alt="" loading="lazy" />
    <figcaption>{snapshot.caption} - {snapshot.author}</figcaption>
</figure>

<style>
    figure {
        display: flex;
        flex-direction: column;
        min-width: 0;
        margin: 0;
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
        padding: 0.5rem 0.75rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-top: none;
        border-radius: 0 0 7px 7px;
        font-size: 0.85em;
    }
</style>
