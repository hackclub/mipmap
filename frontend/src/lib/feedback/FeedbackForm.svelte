<script lang="ts">
    import Field from "$lib/ui/Field.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { send_feedback } from "$lib/feedback/api";
    import { Task } from "$lib/load.svelte";

    let { onsent }: { onsent: () => void } = $props();

    const scores = Array.from({ length: 10 }, (_, i) => i + 1);
    const low = "Not likely at all";
    const high = "Extremely likely";
    const uid = $props.id();

    let score = $state(0);
    let source = $state("");
    let positives = $state("");
    let improvements = $state("");
    const send = new Task();

    async function submit(e: SubmitEvent) {
        e.preventDefault();
        if (!score || send.pending) return;
        await send.run(
            () => send_feedback({ score, source, positives, improvements }),
            onsent,
        );
    }
</script>

<form onsubmit={submit}>
    <fieldset aria-describedby="{uid}-ends">
        <legend>How likely are you to recommend Mipmap to a friend?</legend>
        <div class="scores">
            {#each scores as value}
                <label class:chosen={score === value}>
                    <input
                        type="radio"
                        name="score"
                        {value}
                        aria-label={value === 1
                            ? `1, ${low}`
                            : value === 10
                              ? `10, ${high}`
                              : String(value)}
                        checked={score === value}
                        onchange={() => (score = value)}
                    />
                    {value}
                </label>
            {/each}
        </div>
        <div class="ends" id="{uid}-ends">
            <span>1: {low}</span>
            <span>10: {high}</span>
        </div>
    </fieldset>
    <Field
        label="How did you find out about Mipmap?"
        bind:value={source}
        maxlength={2000}
    />
    <Field
        label="What are we doing well?"
        bind:value={positives}
        multiline
        rows={2}
        maxlength={2000}
    />
    <Field
        label="How can we improve?"
        bind:value={improvements}
        multiline
        rows={2}
        maxlength={2000}
    />
    <div class="actions">
        <MipmapButton type="submit" disabled={!score || send.pending}>
            Submit
        </MipmapButton>
        <div class="note" aria-live="polite">
            {send.error || (score ? "" : "Pick a score from 1 to 10 to submit")}
        </div>
    </div>
</form>

<style>
    form {
        display: grid;
        gap: 1.25rem;
    }

    fieldset {
        margin: 0;
        padding: 0;
        border: none;
        min-width: 0;
    }

    legend {
        padding: 0;
        margin-bottom: 0.5rem;
    }

    .scores {
        display: grid;
        grid-template-columns: repeat(10, minmax(0, 1fr));
        gap: 0.5rem;
    }

    .scores label {
        display: grid;
        place-items: center;
        padding: 0.5rem 0;
        background: #4f2f1f;
        border-radius: 7px;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .scores label.chosen {
        background: #ead7c2;
        color: #4f2f1f;
    }

    .scores label:has(:focus-visible) {
        outline: 2px solid #ead7c2;
        outline-offset: 2px;
    }

    @media (hover: hover) {
        .scores label:hover:not(.chosen) {
            background: #5f4838;
        }
    }

    .ends {
        display: flex;
        justify-content: space-between;
        gap: 1rem;
        margin-top: 0.5rem;
        font-size: 0.75em;
        opacity: 0.85;
    }

    @media (max-width: 40rem) {
        .scores {
            grid-template-columns: repeat(5, minmax(0, 1fr));
        }

        .scores label {
            padding: 0.75rem 0;
        }
    }

    @media (forced-colors: active) {
        .scores label {
            border: 1px solid ButtonText;
        }

        .scores label.chosen {
            forced-color-adjust: none;
            background: Highlight;
            color: HighlightText;
        }

        .scores label:has(:focus-visible) {
            outline-color: CanvasText;
        }
    }

    .scores input {
        position: absolute;
        opacity: 0;
        pointer-events: none;
    }

    .actions {
        display: flex;
        align-items: center;
        gap: 1rem;
    }
</style>
