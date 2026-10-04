<script lang="ts">
    import MipmapButton from "$lib/ui/MipmapButton.svelte";

    let { error, onretry }: { error: string; onretry: () => void } = $props();
</script>

<div class="load-error" role="alert">
    <p>{error}</p>
    <MipmapButton type="button" onclick={onretry}>Retry</MipmapButton>
</div>

<style>
    .load-error {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 0.75rem;
    }

    p {
        margin: 0;
        font-size: 0.85em;
    }
</style>
