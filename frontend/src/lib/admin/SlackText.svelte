<script lang="ts">
    import { render } from "./slack.svelte";

    let { text, footer = false }: { text: string; footer?: boolean } =
        $props();

    const rendered = $derived(render(text));
</script>

<div class="slack" data-plain>
    {@html rendered}
    {#if footer}
        <p class="footer">
            Manage these DMs from your
            <a href="/dashboard" target="_blank">Mipmap dashboard</a>
        </p>
    {/if}
</div>

<style>
    .slack {
        min-width: 0;
        font-size: 0.9rem;
        line-height: 1.45;
        overflow-wrap: anywhere;
    }

    .slack :global(p) {
        margin: 0;
    }

    .slack :global(a) {
        color: #a2b8eb;
    }

    .slack :global(code) {
        padding: 0.1em 0.3em;
        background: #3d3029;
        border: 1px solid #4f2f1f;
        border-radius: 3px;
        color: #d6ab70;
        font-size: 0.85em;
    }

    .slack :global(pre) {
        margin: 0.25rem 0;
        padding: 0.5rem;
        background: #3d3029;
        border: 1px solid #4f2f1f;
        border-radius: 4px;
        font-size: 0.85em;
        white-space: pre-wrap;
    }

    .slack :global(blockquote) {
        margin: 0.25rem 0;
        padding-left: 0.75rem;
        border-left: 4px solid #d6ab70;
    }

    .slack :global(.mention) {
        padding: 0 0.15em;
        background: #4f2f1f;
        border-radius: 3px;
        color: #e8c58c;
    }

    .slack :global(.channel) {
        text-decoration: none;
        transition: background-color 0.2s;
    }

    .slack :global(.channel:hover) {
        background: #5f4838;
        text-decoration: underline;
    }

    .footer {
        margin-top: 0.5rem;
        color: #d6ab70;
        font-size: 0.75rem;
    }
</style>
