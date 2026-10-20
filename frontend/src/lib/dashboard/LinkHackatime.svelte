<script lang="ts">
    import Dialog from "$lib/ui/Dialog.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";

    const uid = $props.id();
</script>

<Dialog labelledby="{uid}-title" width="32rem">
    <h2 id="{uid}-title">Link your Hackatime account</h2>
    <p>This lets you track your hours for prizes!</p>
    <p>Features like Snapshots won't work until you do this</p>
    <div class="actions">
        <MipmapButton
            type="button"
            data-autofocus
            onclick={() => (location.href = "/hackatime/login")}
        >
            Click to Link!
        </MipmapButton>
    </div>
</Dialog>

<style>
    p {
        margin: 0.75rem 0 0;
        font-size: 0.85em;
    }

    .actions {
        display: flex;
        justify-content: flex-end;
        margin-top: 1.5rem;
    }
</style>
