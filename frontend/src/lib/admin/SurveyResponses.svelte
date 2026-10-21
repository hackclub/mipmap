<script lang="ts">
    import { short_date } from "$lib/format";
    import { delete_survey_response, fetch_survey_responses } from "./api";
    import { Pages } from "./pages.svelte";
    import type { SurveyResponse, SurveyResults } from "./types";
    import ActionDialog from "./ActionDialog.svelte";
    import Card from "./Card.svelte";
    import PagedList from "./PagedList.svelte";

    let {
        survey,
        ondelete,
    }: { survey: SurveyResults; ondelete: () => void } = $props();

    const pages = new Pages<SurveyResponse>((before) =>
        fetch_survey_responses(survey.id, { before }),
    );
    pages.reset();

    let deleting = $state<SurveyResponse | null>(null);
</script>

<h3>Responses to #{survey.id}</h3>

<PagedList {pages} empty="No responses yet">
    {#snippet item(response)}
        <Card level={4}>
            {#snippet title()}
                Response #{response.id}
            {/snippet}
            {#snippet meta()}
                {short_date(response.created_on)}
            {/snippet}
            {#each survey.questions as question (question.key)}
                {@const answer = response.answers[question.key]}
                {#if question.type === "scale" && typeof answer === "object"}
                    <dl>
                        {#each question.items as entry (entry.key)}
                            <div>
                                <dt>{entry.label}</dt>
                                <dd>
                                    {answer[entry.key] === "unsure"
                                        ? "Unsure"
                                        : (answer[entry.key] ?? "–")}
                                </dd>
                            </div>
                        {/each}
                    </dl>
                {:else if question.type === "text" && typeof answer === "string"}
                    <div>
                        <div class="label">{question.label}</div>
                        <p>{answer}</p>
                    </div>
                {/if}
            {/each}
            {#snippet actions()}
                <button type="button" onclick={() => (deleting = response)}
                    >Delete</button
                >
            {/snippet}
        </Card>
    {/snippet}
</PagedList>

{#if deleting}
    {@const response = deleting}
    <ActionDialog
        title="Delete response #{response.id}?"
        submit="Delete"
        warning="It's removed from these results. The deletion is recorded in the audit log."
        onsubmit={async () => {
            await delete_survey_response(response.id);
            pages.remove(response.id);
            ondelete();
        }}
        onclose={() => (deleting = null)}
    />
{/if}

<style>
    h3 {
        margin: 0.5rem 0 0;
        font-size: 1rem;
    }

    .label {
        color: #ead7c2;
        font-size: 0.75rem;
    }

    p {
        margin: 0;
        white-space: pre-wrap;
    }
</style>
