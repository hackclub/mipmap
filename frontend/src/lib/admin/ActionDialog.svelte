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

    const uid = $props.id();
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
    <p class="warning" id="{uid}-warning" aria-live="polite">{warning ?? ""}</p>
    {@render children?.()}
    {#if reason !== undefined}
        <Field
            label="Reason"
            multiline
            required
            maxlength={500}
            focus={!children}
            aria-describedby={warning ? `${uid}-warning` : undefined}
            bind:value={reason}
        />
    {/if}
</FormDialog>

<style>
    .warning:empty {
        display: none;
    }

    .warning {
        margin: 0;
        font-size: 0.85em;
    }
</style>
