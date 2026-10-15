<script lang="ts">
    import { onMount } from "svelte";
    import Avatar from "$lib/onboarding/Avatar.svelte";
    import Dialog from "$lib/ui/Dialog.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import FeedbackForm from "$lib/feedback/FeedbackForm.svelte";
    import { dismiss_survey, visit_survey } from "$lib/feedback/api";

    let { active }: { active: boolean } = $props();

    const size = 96;
    const wander_ms = 7000;

    let show = $state(false);
    let stage = $state<"floating" | "asking" | "survey" | "thanks" | "gone">(
        "floating",
    );
    let x = $state(0);
    let y = $state(0);

    function wander() {
        x = Math.random() * Math.max(0, innerWidth - size);
        y = Math.random() * Math.max(0, innerHeight - size);
    }

    onMount(() => {
        const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
        let timer = 0;
        let alive = true;
        if (still) {
            x = innerWidth - size - 24;
            y = innerHeight - size - 24;
        } else wander();
        visit_survey().then((due) => {
            if (!alive) return;
            show = due;
            if (!due || still) return;
            setTimeout(wander, 100);
            timer = window.setInterval(wander, wander_ms);
        });
        return () => {
            alive = false;
            clearInterval(timer);
        };
    });
</script>

{#if show && active}
    {#if stage === "floating"}
        <button
            type="button"
            class="steve"
            aria-label="Steve the Sprite has a question"
            style:transform="translate({x}px, {y}px)"
            style:--size="{size}px"
            style:--wander="{wander_ms}ms"
            onclick={() => (stage = "asking")}
        >
            <Avatar size={size * 0.8} />
        </button>
    {:else if stage === "asking"}
        <Dialog
            labelledby="steve-ask"
            width="32rem"
            onclose={() => (stage = "floating")}
        >
            <div class="ask">
                <div class="disc"><Avatar size={size * 0.8} state="idle" /></div>
                <p id="steve-ask">
                    Hey there! Would you mind filling out a quick survey for 15
                    Mips?
                </p>
            </div>
            <div class="actions">
                <MipmapButton
                    type="button"
                    onclick={() => {
                        stage = "gone";
                        dismiss_survey();
                    }}>Nay</MipmapButton
                >
                <MipmapButton
                    type="button"
                    data-autofocus
                    onclick={() => (stage = "survey")}>Yay</MipmapButton
                >
            </div>
        </Dialog>
    {:else if stage === "survey"}
        <Dialog labelledby="steve-survey" onclose={() => (stage = "gone")}>
            <div class="survey">
                <h2 id="steve-survey">Quick survey</h2>
                <FeedbackForm onsent={() => (stage = "thanks")} />
            </div>
        </Dialog>
    {:else if stage === "thanks"}
        <Dialog
            labelledby="steve-thanks"
            width="32rem"
            onclose={() => (stage = "gone")}
        >
            <div class="ask">
                <div class="disc"><Avatar size={size * 0.8} state="idle" /></div>
                <p id="steve-thanks">Thanks! Here's your 15 Mips</p>
            </div>
            <div class="actions">
                <MipmapButton
                    type="button"
                    data-autofocus
                    onclick={() => (stage = "gone")}>OK</MipmapButton
                >
            </div>
        </Dialog>
    {/if}
{/if}

<style>
    .steve {
        position: fixed;
        top: 0;
        left: 0;
        z-index: 900;
        width: var(--size);
        height: var(--size);
        padding: 0;
        cursor: pointer;
        transition: transform var(--wander) ease-in-out;
        animation: bob 2.4s ease-in-out infinite alternate;
    }

    .steve,
    .disc {
        display: grid;
        place-items: center;
        box-sizing: border-box;
        background: #3d3029;
        border: 2px solid #4f2f1f;
        border-radius: 50%;
    }

    .disc {
        flex-shrink: 0;
        width: 96px;
        height: 96px;
    }

    .steve:focus-visible {
        outline: 3px solid #e8c58c;
        outline-offset: 4px;
    }

    .ask {
        display: flex;
        align-items: center;
        gap: 1.25rem;
    }

    .ask p {
        margin: 0;
    }

    .actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.75rem;
        margin-top: 1.5rem;
    }

    .survey {
        display: grid;
        gap: 1.25rem;
    }

    @keyframes bob {
        to {
            translate: 0 -12px;
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .steve {
            transition: none;
            animation: none;
        }
    }
</style>
