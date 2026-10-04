<script lang="ts">
    import Dialog from "$lib/ui/Dialog.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";

    const uid = $props.id();
</script>

<Dialog labelledby="{uid}-title" width="32rem">
    <h2 id="{uid}-title">Linking Hackatime</h2>
    <p>Linking your Hackatime account will let you track your time</p>
    <div class="actions">
        <MipmapButton
            type="button"
            data-autofocus
            onclick={() => (location.href = "/hackatime/login")}
        >
            Link
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
