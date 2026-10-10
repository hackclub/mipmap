<script lang="ts">
    let { steps }: { steps: { label: string; count: number }[] } = $props();

    const max = $derived(Math.max(1, ...steps.map((step) => step.count)));
    const inset = (count: number) => ((1 - count / max) / 2) * 100;
</script>

<div class="funnel">
    {#each steps as step, i (step.label)}
        {@const top = inset(step.count)}
        {@const bottom = inset((steps[i + 1] ?? step).count)}
        <span>{step.label}</span>
        <div
            class="segment"
            style:clip-path="polygon({top}% 0, {100 - top}% 0, {100 - bottom}% 100%, {bottom}% 100%)"
        ></div>
        <span class="count">
            {step.count.toLocaleString()}
            {#if i > 0 && steps[i - 1].count > 0}
                <span class="percent"
                    >{Math.round((step.count / steps[i - 1].count) * 100)}%</span
                >
            {/if}
        </span>
    {/each}
</div>

<style>
    .funnel {
        display: grid;
        grid-template-columns: max-content 1fr max-content;
        align-items: center;
        gap: 3px 0.75rem;
    }

    .segment {
        height: 2.5rem;
        background: #d6ab70;
    }

    .count {
        text-align: right;
        font-variant-numeric: tabular-nums;
    }

    .percent {
        display: inline-block;
        min-width: 3.5ch;
        margin-left: 0.5rem;
        color: #d6ab70;
    }
</style>
