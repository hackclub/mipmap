<script lang="ts" generics="T extends string">
    let {
        options,
        value = $bindable(),
    }: { options: [T, string][]; value: T } = $props();
</script>

<div class="filters">
    {#each options as [key, label] (key)}
        <button
            type="button"
            class:active={value === key}
            aria-pressed={value === key}
            onclick={() => (value = key)}>{label}</button
        >
    {/each}
</div>

<style>
    .filters {
        display: flex;
        flex-wrap: wrap;
    }

    button {
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

    button:focus-visible,
    button.active {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        button:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }
</style>
