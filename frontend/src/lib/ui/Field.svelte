<script lang="ts">
    import type {
        HTMLInputAttributes,
        HTMLTextareaAttributes,
    } from "svelte/elements";

    let {
        label,
        hint,
        value = $bindable(""),
        multiline = false,
        focus = false,
        rows = 3,
        ...rest
    }: {
        label?: string;
        hint?: string;
        value?: string;
        multiline?: boolean;
        focus?: boolean;
        rows?: number;
    } & Omit<HTMLInputAttributes, "value"> = $props();

    const uid = $props.id();
</script>

<div class="field">
    {#if label}
        <label for={uid}>{label}</label>
    {/if}
    {#if hint}
        <p class="hint" id="{uid}-hint">{hint}</p>
    {/if}
    {#if multiline}
        <textarea
            id={uid}
            {rows}
            aria-describedby={hint ? `${uid}-hint` : undefined}
            bind:value
            data-autofocus={focus || undefined}
            {...rest as HTMLTextareaAttributes}></textarea>
    {:else}
        <input
            id={uid}
            aria-describedby={hint ? `${uid}-hint` : undefined}
            bind:value
            data-autofocus={focus || undefined}
            {...rest}
        />
    {/if}
</div>

<style>
    .field {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        min-width: 0;
    }

    label {
        font-size: 0.85em;
        font-weight: 700;
    }

    .hint {
        margin: 0;
        font-size: 0.75em;
        opacity: 0.85;
    }

    input {
        margin-top: auto;
    }

    input,
    textarea {
        box-sizing: border-box;
        width: 100%;
        padding: 0.5rem 0.75rem;
        background: #4f2f1f;
        color: #ead7c2;
        border: 1px solid #d6ab70;
        border-radius: 0;
        font: inherit;
        font-size: max(1rem, 0.85em);
        resize: vertical;
    }

    input[aria-invalid="true"],
    textarea[aria-invalid="true"] {
        border-color: #ead7c2;
        box-shadow: inset 0 0 0 1px #ead7c2;
    }

    input::placeholder,
    textarea::placeholder {
        color: inherit;
        opacity: 0.75;
    }

    input:disabled,
    textarea:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    input:focus-visible,
    textarea:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: -2px;
    }
</style>
