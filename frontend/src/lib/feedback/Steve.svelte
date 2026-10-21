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
    const roam_ms = 20000;
    const edge = 16;
    const gap = 12;
    const controls_height = 3 * 48 + 4 * 2;

    let show = $state(false);
    let stage = $state<"floating" | "asking" | "survey" | "thanks" | "gone">(
        "floating",
    );
    let x = $state(0);
    let y = $state(0);
    let parked = $state(false);

    function controls_top() {
        const controls = document.querySelector('[aria-label="Map controls"]');
        const top = controls?.getBoundingClientRect().top;
        return top && top > 0 ? top : innerHeight - edge - controls_height;
    }

    function wander() {
        const blocked = controls_top() - gap - size;
        const right = innerWidth - edge - 52 - size;
        for (let tries = 0; tries < 10; tries++) {
            x = Math.random() * Math.max(0, innerWidth - size);
            y = Math.random() * Math.max(0, innerHeight - size);
            if (x < right || y < blocked) return;
        }
    }

    function park() {
        parked = true;
        x = Math.max(0, innerWidth - edge - size);
        y = Math.max(0, controls_top() - gap - size);
    }

    function clamp() {
        if (parked) return park();
        x = Math.min(x, Math.max(0, innerWidth - size));
        y = Math.min(y, Math.max(0, innerHeight - size));
    }

    onMount(() => {
        const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
        let start = 0;
        let timer = 0;
        let stop = 0;
        let alive = true;
        if (still) park();
        else wander();
        visit_survey().then((due) => {
            if (!alive) return;
            show = due;
            if (!due || still) return;
            start = window.setTimeout(wander, 100);
            timer = window.setInterval(wander, wander_ms);
            stop = window.setTimeout(() => {
                clearInterval(timer);
                park();
            }, roam_ms);
        });
        return () => {
            alive = false;
            clearTimeout(start);
            clearInterval(timer);
            clearTimeout(stop);
        };
    });
</script>

<svelte:window onresize={clamp} />

{#if show && active}
    {#if stage === "floating"}
        <button
            type="button"
            class="steve"
            class:parked
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

    .steve.parked {
        animation: none;
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
