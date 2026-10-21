<script lang="ts">
    let { steps }: { steps: { label: string; count: number | null }[] } = $props();

    const max = $derived(Math.max(1, ...steps.map((step) => step.count ?? 0)));
    const inset = (count: number | null) => ((1 - (count ?? 0) / max) / 2) * 100;
    const percent = (count: number | null, previous: number | null) =>
        count === null || !previous
            ? null
            : count > previous
              ? ">100%"
              : `${Math.round((count / previous) * 100)}%`;
</script>

<ol class="funnel">
    {#each steps as step, i (step.label)}
        {@const top = inset(step.count)}
        {@const bottom = inset((steps[i + 1] ?? step).count)}
        {@const share = i > 0 ? percent(step.count, steps[i - 1].count) : null}
        <li>
            <span class="name">{step.label}</span>
            <div
                class="segment"
                style:clip-path="polygon({top}% 0, {100 - top}% 0, {100 - bottom}% 100%, {bottom}% 100%)"
            ></div>
            <span class="count">
                {step.count === null ? "<5" : step.count.toLocaleString()}
                {#if share}
                    <span class="percent"
                        >{share}<span class="sr-only"> of the previous step</span></span
                    >
                {/if}
            </span>
        </li>
    {/each}
</ol>

<style>
    .funnel {
        display: grid;
        grid-template-columns: max-content minmax(2rem, 1fr) max-content;
        align-items: center;
        gap: 3px 0.75rem;
        margin: 0;
        padding: 0;
        list-style: none;
    }

    li {
        display: grid;
        grid-template-columns: subgrid;
        grid-column: 1 / -1;
        align-items: center;
    }

    .segment {
        height: 2.5rem;
        background: #d6ab70;
    }

    .count {
        text-align: right;
        font-variant-numeric: tabular-nums;
        white-space: nowrap;
    }

    .percent {
        display: inline-block;
        min-width: 3.5ch;
        margin-left: 0.5rem;
        color: #ead7c2;
    }

    .sr-only {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }

    @media (max-width: 26.25rem) {
        .funnel {
            grid-template-columns: minmax(0, 1fr) max-content;
            row-gap: 0.5rem;
        }

        li {
            row-gap: 0.25rem;
        }

        .name {
            grid-column: 1 / -1;
        }
    }
</style>
