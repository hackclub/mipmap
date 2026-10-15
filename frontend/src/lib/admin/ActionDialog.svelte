<script lang="ts">
    import type { Snippet } from "svelte";
    import FormDialog from "$lib/ui/FormDialog.svelte";
    import Field from "$lib/ui/Field.svelte";

    let {
        title,
        submit,
        warning,
        reason = $bindable(),
        valid = true,
        onsubmit,
        onclose,
        children,
    }: {
        title: string;
        submit: string;
        warning?: string;
        reason?: string;
        valid?: boolean;
        onsubmit: () => Promise<unknown>;
        onclose: () => void;
        children?: Snippet;
    } = $props();
</script>

<FormDialog
    {title}
    {submit}
    width="32rem"
    cancel
    valid={valid && (reason === undefined || !!reason.trim())}
    {onsubmit}
    {onclose}
>
    {#if warning}
        <p class="warning">{warning}</p>
    {/if}
    {@render children?.()}
    {#if reason !== undefined}
        <Field
            label="Reason"
            multiline
            required
            maxlength={500}
            focus={!children}
            bind:value={reason}
        />
    {/if}
</FormDialog>

<style>
    .warning {
        margin: 0;
        font-size: 0.85em;
    }
</style>
