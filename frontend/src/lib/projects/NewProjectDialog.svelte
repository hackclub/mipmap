<script lang="ts">
    import Dialog from "$lib/Dialog.svelte";
    import Field from "$lib/Field.svelte";
    import MipmapButton from "$lib/MipmapButton.svelte";
    import { create_project } from "$lib/projects/api";
    import type { Project } from "$lib/projects/types";

    let {
        onclose,
        oncreate,
    }: { onclose: () => void; oncreate: (project: Project) => void } = $props();

    const uid = $props.id();
    let name = $state("");
    let pending = $state(false);
    let error = $state("");
    let closed = false;

    function close() {
        closed = true;
        onclose();
    }

    async function submit(e: SubmitEvent) {
        e.preventDefault();
        if (!name.trim() || pending) return;
        pending = true;
        error = "";
        try {
            oncreate(await create_project(name.trim()));
            if (!closed) close();
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            pending = false;
        }
    }
</script>

<Dialog
    labelledby="{uid}-title"
    corner="bottom-right"
    width="32rem"
    onclose={close}
>
    <form onsubmit={submit}>
        <h2 id="{uid}-title">New project</h2>
        <Field label="Name" bind:value={name} focus />
        {#if error}
            <div class="error">{error}</div>
        {/if}
        <div class="actions">
            <MipmapButton type="submit" disabled={pending}>Create</MipmapButton>
        </div>
    </form>
</Dialog>

<style>
    form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .actions {
        display: flex;
        justify-content: flex-end;
    }

    .error {
        font-size: 0.85em;
    }
</style>
