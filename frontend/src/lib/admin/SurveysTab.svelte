<script lang="ts">
    import { tick } from "svelte";
    import { message } from "$lib/api";
    import { Load } from "$lib/load.svelte";
    import { short_date } from "$lib/format";
    import LoadError from "$lib/ui/LoadError.svelte";
    import definitions from "$lib/assets/surveys.json";
    import { fetch_surveys } from "./api";
    import type { SurveyResults } from "./types";
    import Card from "./Card.svelte";
    import SurveyResponses from "./SurveyResponses.svelte";

    type Answer = { id: number; created_on: string; text: string };
    type Extra = {
        answers: Answer[];
        next: number | null;
        loading: boolean;
        error: string;
    };

    const load = new Load(() => fetch_surveys());
    let extra = $state<Record<string, Extra>>({});

    const reload = () => {
        extra = {};
        return load.run();
    };
    reload();

    const scale_ends = (survey: number, key: string) => {
        const question = definitions
            .find((d) => d.id === survey)
            ?.questions.find((q) => q.key === key);
        return question && "low" in question
            ? `1 = ${question.low}, 5 = ${question.high}`
            : "";
    };

    async function more(
        survey: SurveyResults,
        key: string,
        before: number,
        shown: number,
    ) {
        const id = `${survey.id}:${key}`;
        const current = extra[id] ?? {
            answers: [],
            next: before,
            loading: false,
            error: "",
        };
        if (current.loading) return;
        const scope = extra;
        extra[id] = { ...current, loading: true, error: "" };
        try {
            const page = (await fetch_surveys(before))
                .find((s) => s.id === survey.id)
                ?.questions.find((q) => q.key === key);
            if (extra !== scope) return;
            const answers = page?.type === "text" ? page.answers : [];
            const next = page?.type === "text" ? page.next : null;
            extra[id] = {
                answers: [...current.answers, ...answers],
                next,
                loading: false,
                error: "",
            };
            await tick();
            document.getElementById(`answer-${id}-${shown}`)?.focus();
        } catch (e) {
            if (extra === scope)
                extra[id] = { ...current, loading: false, error: message(e) };
        }
    }

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
    <LoadError error={load.error} onretry={reload} />
{:else if !load.data}
    <p class="note" role="status">Loading…</p>
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
                            <div class="bars" role="group" aria-label="{item.label}: ratings, {scale_ends(survey.id, question.key)}">
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
                            {#if scale_ends(survey.id, question.key)}
                                <span class="note"
                                    >{scale_ends(survey.id, question.key)}</span
                                >
                            {/if}
                        </div>
                    {/each}
                {:else}
                    {@const id = `${survey.id}:${question.key}`}
                    {@const added = extra[id]}
                    {@const next = added ? added.next : question.next}
                    <div class="item">
                        <div class="label">{question.label}</div>
                        {#each [...question.answers, ...(added?.answers ?? [])] as answer, i (answer.id)}
                            <div class="answer" id="answer-{id}-{i}" tabindex="-1">
                                <span class="note"
                                    >{short_date(answer.created_on)}</span
                                >
                                <p>{answer.text}</p>
                            </div>
                        {:else}
                            <p class="note">No answers yet</p>
                        {/each}
                        {#if added?.error}
                            <p class="error" role="alert">{added.error}</p>
                        {/if}
                        {#if next}
                            <button
                                type="button"
                                class="more"
                                aria-disabled={added?.loading}
                                onclick={() =>
                                    more(
                                        survey,
                                        question.key,
                                        next,
                                        question.answers.length +
                                            (added?.answers.length ?? 0),
                                    )}
                                >{added?.loading
                                    ? "Loading…"
                                    : "Load more answers"}</button
                            >
                        {/if}
                    </div>
                {/if}
            {/each}
        </Card>
        <SurveyResponses {survey} ondelete={reload} />
    {/each}
{/if}

<style>
    p {
        margin: 0;
    }

    .note {
        color: #ead7c2;
        font-size: 0.8rem;
    }

    .label {
        color: #ead7c2;
        font-size: 0.75rem;
    }

    .error {
        font-size: 0.85rem;
        font-weight: 700;
    }

    .answer:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: 2px;
    }

    .more {
        align-self: flex-start;
        padding: 0.4rem 0.8rem;
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

    .more[aria-disabled="true"] {
        opacity: 0.6;
        cursor: default;
    }

    .more:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .more:not([aria-disabled="true"]):hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
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
