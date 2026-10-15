<script lang="ts">
    import Field from "$lib/ui/Field.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { send_feedback } from "$lib/feedback/api";
    import { Task } from "$lib/load.svelte";

    let { onsent }: { onsent: () => void } = $props();

    const scores = Array.from({ length: 10 }, (_, i) => i + 1);

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
    <fieldset>
        <legend>How likely are you to recommend Mipmap to a friend?</legend>
        <div class="scores">
            {#each scores as value}
                <label class:chosen={score === value}>
                    <input
                        type="radio"
                        name="score"
                        {value}
                        checked={score === value}
                        onchange={() => (score = value)}
                    />
                    {value}
                </label>
            {/each}
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
            {send.error}
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

    .scores label:has(:focus-visible),
    .scores label.chosen {
        background: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .scores label:hover {
            background: #ead7c2;
            color: #4f2f1f;
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
