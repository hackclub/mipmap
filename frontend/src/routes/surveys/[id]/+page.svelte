<script lang="ts">
    import { onMount } from "svelte";
    import { Task } from "$lib/load.svelte";
    import SiteHead from "$lib/SiteHead.svelte";
    import Field from "$lib/ui/Field.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import Scale from "$lib/surveys/Scale.svelte";
    import { completed, mark_completed, submit_survey } from "$lib/surveys/api";
    import type { Rating } from "$lib/surveys/types";

    let { data } = $props();
    const survey = $derived(data.survey);

    let status = $state<"loading" | "open" | "done" | "sent">("loading");
    let ratings = $state<Record<string, Record<string, Rating>>>({});
    let texts = $state<Record<string, string>>({});
    const send = new Task();

    const missing = $derived(
        survey.questions.reduce(
            (total, question) =>
                question.type === "scale"
                    ? total +
                      question.items.filter(
                          (item) => !ratings[question.key]?.[item.key],
                      ).length
                    : !question.optional && !texts[question.key]?.trim()
                      ? total + 1
                      : total,
            0,
        ),
    );

    onMount(() => {
        for (const question of survey.questions) {
            if (question.type === "scale") ratings[question.key] = {};
            else texts[question.key] = "";
        }
        status = completed(survey.id) ? "done" : "open";
    });

    function submit(e: SubmitEvent) {
        e.preventDefault();
        if (missing || send.pending) return;
        send.run(
            () => submit_survey(survey.id, { ...ratings, ...texts }),
            () => {
                mark_completed(survey.id);
                status = "sent";
            },
        );
    }
</script>

<SiteHead
    title="{survey.title} - Hack Club Mipmap"
    description="An anonymous Hack Club Mipmap survey."
    path="/surveys/{survey.id}"
/>

<main>
    <header>
        <h1>
            {#each survey.heading as segment, i (i)}{#if segment.colour}<span
                        style:color={segment.colour}>{segment.text}</span
                    >{:else}{segment.text}{/if}{/each}
        </h1>
        <p class="note">Responses are anonymous</p>
    </header>

    {#if status === "done"}
        <p class="status">
            It looks like you've already completed the form based on a little
            thing tucked away in your browser's private local storage
        </p>
    {:else if status === "sent"}
        <p class="status">Thanks! Your response has been recorded.</p>
    {:else if status === "open"}
        <form onsubmit={submit}>
            {#each survey.questions as question (question.key)}
                {#if question.type === "scale"}
                    <Scale {question} bind:ratings={ratings[question.key]} />
                {:else}
                    <div class="text">
                        <Field
                            label={question.label}
                            multiline
                            rows={3}
                            maxlength={2000}
                            bind:value={texts[question.key]}
                        />
                    </div>
                {/if}
            {/each}
            <div class="actions">
                <MipmapButton type="submit" disabled={!!missing || send.pending}
                    >Submit</MipmapButton
                >
                <p class="note" aria-live="polite">
                    {missing
                        ? `Rate ${missing} more ${missing === 1 ? "topic" : "topics"} to submit`
                        : send.error}
                </p>
            </div>
        </form>
    {/if}
</main>

<style>
    main {
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: 2rem;
        max-width: 44rem;
        margin: 0 auto;
        padding: 3rem 1rem;
        color: #ead7c2;
    }

    header {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    h1 {
        margin: 0;
        font-size: clamp(1.75rem, 5vw, 2.75rem);
        font-weight: 700;
        line-height: 1.2;
    }

    p {
        margin: 0;
    }

    .note {
        color: #d6ab70;
    }

    .status {
        font-size: 1.25rem;
    }

    form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .text {
        padding: 1rem 1.25rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    .text :global(label) {
        margin-bottom: 0.5rem;
        font-size: 1rem;
        font-weight: 700;
    }

    .actions {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 1rem;
    }

    @media (max-width: 30rem) {
        main {
            gap: 1.5rem;
            padding: 2rem 1rem;
        }

        h1 {
            font-size: clamp(1.5rem, 6.5vw, 1.75rem);
        }

        .text {
            padding: 0.85rem 0.9rem;
        }

        .text :global(label) {
            margin-bottom: 0.4rem;
            line-height: 1.3;
        }

        .text :global(textarea) {
            font-size: 1rem;
        }

        .actions {
            flex-direction: column;
            align-items: stretch;
            gap: 0.5rem;
            text-align: center;
        }
    }
</style>
