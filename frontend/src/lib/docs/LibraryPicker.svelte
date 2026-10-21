<script lang="ts" module>
    export type Library = {
        name: string;
        description: string;
        note?: string;
        tier: "window" | "romeo" | "sierra";
        links: { text: string; href: string }[];
    };

    export const tier_labels = {
        window: "Window library",
        romeo: "Tier Romeo",
        sierra: "Tier Sierra",
    };
</script>

<script lang="ts">
    import Dialog, { refocus } from "$lib/ui/Dialog.svelte";

    let {
        libraries,
        onclose,
    }: { libraries: Library[]; onclose: () => void } = $props();

    const uid = $props.id();

    type Kind = "window" | "gpu";
    type Levels = [string[], string[], string[]];

    const picks: Record<string, { window: string[]; gpu: Levels; tip?: string }> = {
        "C / C++": {
            window: ["SDL", "GLFW", "sokol"],
            gpu: [["OpenGL"], ["bgfx", "sokol"], ["Vulkan", "Software renderer", "Bare metal"]],
        },
        Rust: {
            window: ["SDL", "miniquad", "winit"],
            gpu: [["miniquad"], ["glow", "wgpu"], ["Vulkan", "Software renderer"]],
        },
        "C#": {
            window: ["OpenTK", "Silk.NET"],
            gpu: [["OpenTK"], ["Silk.NET", "OpenGL"], ["Silk.NET", "Vulkan", "Software renderer"]],
        },
        Java: {
            window: ["LWJGL", "GLFW"],
            gpu: [["LWJGL", "OpenGL"], ["LWJGL", "OpenGL"], ["LWJGL", "Vulkan", "Software renderer"]],
        },
        Python: {
            window: ["SDL", "GLFW"],
            gpu: [["moderngl"], ["moderngl", "OpenGL"], ["OpenGL", "Software renderer"]],
        },
        Go: {
            window: ["SDL", "go-gl", "GLFW"],
            gpu: [["go-gl", "OpenGL"], ["go-gl", "OpenGL"], ["OpenGL", "Software renderer"]],
        },
        JavaScript: {
            tip: "The browser already gives you a window, so grab a canvas and go straight to the GPU.",
            window: ["WebGL"],
            gpu: [["WebGL"], ["WebGL"], ["Software renderer"]],
        },
    };

    const kinds: [Kind, string][] = [
        ["window", "A library for managing windows"],
        ["gpu", "A library for talking to the GPU"],
    ];

    const levels = ["Keep it easy", "Some challenge", "Make it hard"];

    let language = $state<string>();
    let kind = $state<Kind>();
    let level = $state<number>();
    let box = $state<HTMLElement>();
    let question = $state<HTMLElement>();
    let moved = false;

    const step = $derived(
        language === undefined
            ? 0
            : kind === undefined
              ? 1
              : kind === "gpu" && level === undefined
                ? 2
                : 3,
    );

    const results = $derived(
        language && step === 3
            ? (kind === "window" ? picks[language].window : picks[language].gpu[level!])
                  .map((name) => libraries.find((library) => library.name === name))
                  .filter((library) => library !== undefined)
            : [],
    );

    const questions = [
        "Which language do you want to use?",
        "What are you looking for?",
        "How much of a challenge do you want?",
        "Try these:",
    ];

    $effect(() => {
        void step;
        if (moved) question?.focus();
        else refocus(box, true);
        moved = true;
    });

    function back() {
        if (step === 3 && kind === "gpu") level = undefined;
        else if (step >= 2) kind = undefined;
        else language = undefined;
    }

    function restart() {
        language = kind = level = undefined;
    }
</script>

<Dialog labelledby="{uid}-title" width="40rem" {onclose}>
    <h2 id="{uid}-title">Which library to choose?</h2>
    <div class="body" bind:this={box}>
        <h3 class="question" tabindex="-1" bind:this={question}>{questions[step]}</h3>
        {#if step === 0}
            <div class="options">
                {#each Object.keys(picks) as option}
                    <button type="button" onclick={() => (language = option)}>{option}</button>
                {/each}
            </div>
        {:else if step === 1}
            <div class="options">
                {#each kinds as [value, label]}
                    <button type="button" onclick={() => (kind = value)}>{label}</button>
                {/each}
            </div>
        {:else if step === 2}
            <div class="options">
                {#each levels as label, index}
                    <button type="button" onclick={() => (level = index)}>{label}</button>
                {/each}
            </div>
        {:else}
            {#if kind === "window" && language && picks[language].tip}
                <p class="description">{picks[language].tip}</p>
            {/if}
            <div class="results">
                {#each results as { name, description, tier, links }}
                    <section>
                        <h4>{name}</h4>
                        <p class="tier">{tier_labels[tier]}</p>
                        <p class="description">{description}</p>
                        <ul>
                            {#each links as { text, href }}
                                <li>
                                    <a {href} target="_blank" rel="noopener">{text}</a>
                                </li>
                            {/each}
                        </ul>
                    </section>
                {/each}
            </div>
        {/if}
        {#if step > 0}
            <div class="controls">
                <button type="button" onclick={back}>Back</button>
                {#if step === 3}
                    <button type="button" onclick={restart}>Start over</button>
                {/if}
            </div>
        {/if}
    </div>
</Dialog>

<style>
    .body {
        font-size: 1.125rem;
    }

    p {
        margin: 1rem 0 0;
    }

    .question {
        margin: 1rem 0 0;
        font-size: inherit;
        font-weight: 400;
    }

    .question:focus:not(:focus-visible) {
        outline: none;
    }

    .options {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
        margin-top: 1rem;
    }

    .controls {
        display: flex;
        gap: 0.75rem;
        margin-top: 1.5rem;
    }

    button {
        padding: 0.5rem 1rem;
        background-color: #4f2f1f;
        border: none;
        color: #ead7c2;
        font: inherit;
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    button:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        button:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    .results {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        margin-top: 1rem;
    }

    section {
        padding: 1rem;
        background: #5f4838;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    h4 {
        margin: 0;
        font-size: 1.25rem;
        font-weight: 700;
    }

    .tier {
        display: inline-block;
        margin: 0.35rem 0 0;
        padding: 0.1rem 0.6rem;
        background-color: #4f2f1f;
        border-radius: 999px;
        font-size: 0.85rem;
        font-weight: 700;
    }

    .description {
        margin: 0.25rem 0 0;
        opacity: 0.85;
    }

    ul {
        margin: 0.5rem 0 0;
        padding-left: 1.25rem;
    }

    a {
        color: #c8d5f3;
    }

    a:focus-visible {
        color: #ead7c2;
    }

    @media (hover: hover) {
        a:hover {
            color: #ead7c2;
        }
    }
</style>
