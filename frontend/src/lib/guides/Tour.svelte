<script lang="ts">
    import { untrack } from "svelte";
    import type PanZoom from "$lib/dashboard/PanZoom.svelte";
    import type { Bounds, Inset } from "$lib/dashboard/PanZoom.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { steps, type Part } from "$lib/guides/TourSteps.svelte";

    let {
        sections,
        panzoom,
        step = $bindable(0),
        onclose,
    }: {
        sections: Record<Part | "all", Bounds>;
        panzoom: PanZoom | undefined;
        step?: number;
        onclose: () => void;
    } = $props();

    const margin = 16;
    const ms = 700;
    const uid = $props.id();

    let card: HTMLDivElement;
    let view_width = $state(0);
    let view_height = $state(0);
    let card_width = $state(0);
    let card_height = $state(0);
    let ready = $state(false);
    let measured = false;

    const current = $derived(steps[step]);
    const last = $derived(step === steps.length - 1);

    const target = $derived.by(() => {
        const parts = current.focus?.map((part) => sections[part]);
        if (!parts) return sections.all;
        const left = Math.min(...parts.map((b) => b.left));
        const top = Math.min(...parts.map((b) => b.top));
        const right = Math.max(...parts.map((b) => b.left + b.width));
        const bottom = Math.max(...parts.map((b) => b.top + b.height));
        return { left, top, width: right - left, height: bottom - top };
    });

    const side = $derived.by(() => {
        if (view_width < 760 || view_width < view_height) return "bottom";
        const all = sections.all.left + sections.all.width / 2;
        return target.left + target.width / 2 < all ? "right" : "left";
    });

    const position = $derived(
        side === "bottom"
            ? {
                  x: (view_width - card_width) / 2,
                  y: view_height - card_height - margin,
              }
            : {
                  x:
                      side === "left"
                          ? margin
                          : view_width - card_width - margin,
                  y: (view_height - card_height) / 2,
              },
    );

    const inset = $derived<Inset>({
        left: side === "left" ? card_width + margin : 0,
        top: 0,
        right: side === "right" ? card_width + margin : 0,
        bottom: side === "bottom" ? card_height + margin : 0,
    });

    $effect(() => {
        if (!card_width || !card_height) return;
        const [view, bounds, area] = [panzoom, target, inset];
        untrack(() =>
            view?.show(bounds, {
                ms,
                padding: 24,
                cap: Infinity,
                floor: 0.01,
                free: true,
                inset: area,
            }),
        );
        if (!measured) {
            measured = true;
            requestAnimationFrame(() => (ready = true));
        }
    });

    $effect(() => {
        card.querySelector<HTMLElement>("[data-next]")?.focus();
    });

    function back() {
        step -= 1;
        if (step === 0) card.querySelector<HTMLElement>("[data-next]")?.focus();
    }

    function next() {
        if (last) onclose();
        else step += 1;
    }
</script>

<svelte:window
    bind:innerWidth={view_width}
    bind:innerHeight={view_height}
    onkeydown={(e) => e.key === "Escape" && onclose()}
/>

<div
    class="card"
    class:ready
    bind:this={card}
    bind:offsetWidth={card_width}
    bind:offsetHeight={card_height}
    style:transform="translate({position.x}px, {position.y}px)"
    style:--duration="{ms}ms"
    role="dialog"
    aria-labelledby="{uid}-title"
>
    <button type="button" class="close" aria-label="Close" onclick={onclose}>
        <svg viewBox="0 0 10 10" aria-hidden="true">
            <path
                d="M5 3.94 8.47.47A.75.75 0 0 1 9.53 1.53L6.06 5 9.53 8.47A.75.75 0 0 1 8.47 9.53L5 6.06 1.53 9.53A.75.75 0 0 1 .47 8.47L3.94 5 .47 1.53A.75.75 0 0 1 1.53.47Z"
            />
        </svg>
    </button>
    <h2 id="{uid}-title">{current.title}</h2>
    {@render current.body()}
    <div class="actions">
        <MipmapButton type="button" disabled={step === 0} onclick={back}>
            Back
        </MipmapButton>
        <MipmapButton type="button" data-next onclick={next}>
            {last ? "Finish" : "Next"}
        </MipmapButton>
    </div>
</div>

<style>
    .card {
        position: fixed;
        top: 0;
        left: 0;
        z-index: 900;
        box-sizing: border-box;
        width: min(28rem, calc(100vw - 2rem));
        max-height: calc(100vh - 2rem);
        overflow-y: auto;
        padding: 1.5rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
        line-height: 1.35;
        visibility: hidden;
        filter: drop-shadow(4px 6px 6px rgba(0, 0, 0, 0.45));
    }

    .card.ready {
        visibility: visible;
        transition: transform var(--duration) ease-in-out;
    }

    h2 {
        margin: 0;
        padding-right: 1.5rem;
        font: inherit;
        font-size: calc(clamp(2rem, 4vw, 3.5rem) * 0.75);
        font-weight: 700;
        line-height: 1.15;
    }

    .card :global(p) {
        margin: 0.75rem 0 0;
        font-size: 0.85em;
    }

    .close {
        position: absolute;
        top: 0.75rem;
        right: 0.75rem;
        display: grid;
        padding: 0.25rem;
        background: none;
        border: none;
        color: inherit;
        cursor: pointer;
        opacity: 0.6;
        transition: opacity 0.3s;
    }

    .close:hover,
    .close:focus-visible {
        opacity: 1;
    }

    .close svg {
        width: 0.75em;
        height: 0.75em;
        fill: currentColor;
    }

    .actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.75rem;
        margin-top: 1.5rem;
    }

    @media (prefers-reduced-motion: reduce) {
        .card.ready {
            transition: none;
        }
    }
</style>
