<script lang="ts">
    import Dialog from "$lib/ui/Dialog.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { Task } from "$lib/load.svelte";
    import { log_out } from "$lib/account/logout";

    let { error }: { error?: string } = $props();

    const uid = $props.id();
    const logout = new Task("Couldn't log out");
</script>

<Dialog labelledby="{uid}-title" width="32rem">
    <h2 id="{uid}-title">Link your Hackatime account</h2>
    <p>This lets you track your hours for prizes!</p>
    <p>You'll need to link it before you can keep going</p>
    {#if error}
        <p class="error" role="alert">{error}</p>
    {/if}
    {#if logout.error}
        <p class="error" role="alert">{logout.error}</p>
    {/if}
    <div class="actions">
        <MipmapButton
            type="button"
            disabled={logout.pending}
            onclick={() => logout.run(() => log_out())}
        >
            Log out
        </MipmapButton>
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

    .error {
        padding: 0.75rem 1rem;
        background: #4f2f1f;
        border-radius: 7px;
    }

    .actions {
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        gap: 0.75rem;
        margin-top: 1.5rem;
    }
</style>
