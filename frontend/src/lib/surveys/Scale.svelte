<script lang="ts">
    import type { Rating, ScaleQuestion } from "./types";

    let {
        question,
        ratings = $bindable(),
    }: { question: ScaleQuestion; ratings: Record<string, Rating> } = $props();

    const points = [1, 2, 3, 4, 5];
    const uid = $props.id();

    const label = (point: number) =>
        point === 1
            ? `1, ${question.low}`
            : point === points.length
              ? `${point}, ${question.high}`
              : String(point);
</script>

{#each question.items as item (item.key)}
    {@const name = `${uid}-${item.key}`}
    <div class="item" role="radiogroup" aria-labelledby="{name}-label">
        <p class="label" id="{name}-label">{item.label}</p>
        <div class="points">
            {#each points as point (point)}
                <label class:chosen={ratings[item.key] === point}>
                    <input
                        type="radio"
                        {name}
                        value={point}
                        aria-label={label(point)}
                        checked={ratings[item.key] === point}
                        onchange={() => (ratings[item.key] = point)}
                    />
                    {point}
                </label>
            {/each}
        </div>
        <div class="ends" aria-hidden="true">
            <span>{question.low}</span>
            <span>{question.high}</span>
        </div>
        <label class="unsure" class:chosen={ratings[item.key] === "unsure"}>
            <input
                type="radio"
                {name}
                value="unsure"
                checked={ratings[item.key] === "unsure"}
                onchange={() => (ratings[item.key] = "unsure")}
            />
            Unsure
        </label>
    </div>
{/each}

<style>
    .item {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        min-width: 0;
        padding: 1rem 1.25rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    .label {
        margin: 0 0 0.25rem;
        font-weight: 700;
    }

    .points {
        display: grid;
        grid-template-columns: repeat(5, minmax(0, 1fr));
        gap: 0.5rem;
    }

    label {
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

    label:has(:focus-visible),
    label.chosen {
        background: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        label:hover {
            background: #ead7c2;
            color: #4f2f1f;
        }
    }

    input {
        position: absolute;
        opacity: 0;
        pointer-events: none;
    }

    .ends {
        display: flex;
        justify-content: space-between;
        gap: 1rem;
        color: #d6ab70;
        font-size: 0.8rem;
    }

    .unsure {
        align-self: flex-end;
        padding: 0.3rem 0.9rem;
        font-size: 0.85rem;
    }

    @media (max-width: 30rem) {
        .item {
            gap: 0.4rem;
            padding: 0.85rem 0.9rem;
        }

        .label {
            line-height: 1.3;
        }

        .points {
            gap: 0.35rem;
        }

        .points label {
            padding: 0.7rem 0;
        }

        .unsure {
            padding: 0.45rem 1rem;
        }
    }
</style>
