<script lang="ts">
    import { Load } from "$lib/load.svelte";
    import { short_date } from "$lib/format";
    import LoadError from "$lib/ui/LoadError.svelte";
    import { fetch_surveys } from "./api";
    import Card from "./Card.svelte";

    const load = new Load(fetch_surveys);
    load.run();

    const rated = (counts: number[]) =>
        counts.reduce((total, count) => total + count, 0);

    const average = (counts: number[]) => {
        const total = rated(counts);
        return total
            ? (
                  counts.reduce((sum, count, i) => sum + count * (i + 1), 0) /
                  total
              ).toFixed(2)
            : "–";
    };
</script>

{#if load.error}
    <LoadError error={load.error} onretry={load.run} />
{:else if !load.data}
    <p class="note">Loading…</p>
{:else}
    {#each load.data as survey (survey.id)}
        <Card>
            {#snippet title()}
                <a href="/surveys/{survey.id}" target="_blank"
                    >#{survey.id} {survey.title}</a
                >
            {/snippet}
            {#snippet meta()}
                {survey.responses}
                {survey.responses === 1 ? "response" : "responses"}
            {/snippet}
            {#each survey.questions as question (question.key)}
                {#if question.type === "scale"}
                    {#each question.items as item (item.key)}
                        {@const total = rated(item.counts)}
                        <div class="item">
                            <div class="row">
                                <span>{item.label}</span>
                                <span class="note"
                                    >avg {average(item.counts)} · {item.unsure} unsure</span
                                >
                            </div>
                            <div class="bars">
                                {#each item.counts as count, i (i)}
                                    <div class="bar">
                                        <div class="track">
                                            <div
                                                class="fill"
                                                style:height="{total
                                                    ? (count / total) * 100
                                                    : 0}%"
                                            ></div>
                                        </div>
                                        <span class="note">{i + 1}: {count}</span>
                                    </div>
                                {/each}
                            </div>
                        </div>
                    {/each}
                {:else}
                    <div class="item">
                        <div class="label">{question.label}</div>
                        {#each question.answers as answer, i (i)}
                            <div class="answer">
                                <span class="note"
                                    >{short_date(answer.created_on)}</span
                                >
                                <p>{answer.text}</p>
                            </div>
                        {:else}
                            <p class="note">No answers yet</p>
                        {/each}
                    </div>
                {/if}
            {/each}
        </Card>
    {/each}
{/if}

<style>
    p {
        margin: 0;
    }

    .note {
        color: #d6ab70;
        font-size: 0.8rem;
    }

    .label {
        color: #d6ab70;
        font-size: 0.75rem;
    }

    .item {
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
    }

    .row {
        display: flex;
        flex-wrap: wrap;
        justify-content: space-between;
        gap: 0.25rem 1rem;
        font-size: 0.9rem;
    }

    .bars {
        display: grid;
        grid-template-columns: repeat(5, minmax(0, 1fr));
        gap: 0.4rem;
    }

    .bar {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.2rem;
    }

    .track {
        display: flex;
        align-items: flex-end;
        width: 100%;
        height: 3rem;
        background: #3d3029;
        border-radius: 3px;
    }

    .fill {
        width: 100%;
        min-height: 2px;
        background: #d6ab70;
        border-radius: 3px 3px 0 0;
    }

    .answer {
        padding: 0.5rem 0.75rem;
        background: #3d3029;
        border: 1px solid #4f2f1f;
        border-radius: 5px;
    }

    .answer p {
        white-space: pre-wrap;
    }
</style>
