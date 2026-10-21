<script lang="ts" generics="T extends string">
    let {
        options,
        label,
        value = $bindable(),
    }: { options: [T, string][]; label: string; value: T } = $props();
</script>

<div class="filters" role="group" aria-label={label}>
    {#each options as [key, name], i (i)}
        <button
            type="button"
            class:active={value === key}
            aria-pressed={value === key}
            onclick={() => (value = key)}>{name}</button
        >
    {/each}
</div>

<style>
    .filters {
        display: flex;
        flex-wrap: wrap;
        gap: 0.25rem;
    }

    button {
        min-height: 2rem;
        padding: 0.25rem 0.6rem;
        background-color: #4f2f1f;
        border: none;
        color: #ead7c2;
        font: inherit;
        font-size: 0.85rem;
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    button.active {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    button:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: 2px;
    }

    @media (hover: hover) {
        button:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }
</style>
