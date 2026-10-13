<script lang="ts">
    import type { Snippet } from "svelte";
    import Dialog from "$lib/ui/Dialog.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { Task } from "$lib/load.svelte";

    let {
        title,
        submit,
        width,
        valid = true,
        cancel = false,
        onsubmit,
        onclose,
        children,
    }: {
        title: string;
        submit: string;
        width?: string;
        valid?: boolean;
        cancel?: boolean;
        onsubmit: () => Promise<unknown>;
        onclose: () => void;
        children: Snippet;
    } = $props();

    const uid = $props.id();
    const task = new Task();
    let closed = false;

    function close() {
        closed = true;
        onclose();
    }

    async function save(e: SubmitEvent) {
        e.preventDefault();
        if (!valid || task.pending) return;
        await task.run(onsubmit, () => {
            if (!closed) close();
        });
    }
</script>

<Dialog
    labelledby="{uid}-title"
    corner="bottom-right"
    {width}
    onclose={close}
>
    <form onsubmit={save}>
        <h2 id="{uid}-title">{title}</h2>
        {@render children()}
        {#if task.error}
            <div class="error">{task.error}</div>
        {/if}
        <div class="actions" data-tour="actions">
            {#if cancel}
                <MipmapButton type="button" onclick={close}>Cancel</MipmapButton>
            {/if}
            <MipmapButton type="submit" disabled={task.pending}>
                {submit}
            </MipmapButton>
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
        gap: 1rem;
    }

    .error {
        font-size: 0.85em;
    }
</style>
