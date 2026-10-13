<script lang="ts">
    import { tick, untrack } from "svelte";
    import type PanZoom from "$lib/dashboard/PanZoom.svelte";
    import type { Bounds, Inset } from "$lib/dashboard/PanZoom.svelte";
    import type { ProjectDialog } from "$lib/projects/ProjectsPanel.svelte";
    import type { Project } from "$lib/projects/types";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import {
        interactive,
        steps,
        type Part,
        type TourNote,
    } from "$lib/guides/TourSteps.svelte";

    type Placed = {
        x: number;
        y: number;
        width: number;
        arrow: number;
        measured: boolean;
    };

    let {
        sections,
        panzoom,
        projects,
        token,
        replay = false,
        step = $bindable(0),
        dialog = $bindable(null),
        onclose,
    }: {
        sections: Record<Part | "all" | "token", Bounds>;
        panzoom: PanZoom | undefined;
        projects: Project[];
        token?: number;
        replay?: boolean;
        step?: number;
        dialog?: ProjectDialog;
        onclose: () => void;
    } = $props();

    const margin = 16;
    const gap = 12;
    const note_width = 320;
    const note_min = 192;
    const ms = 700;
    const uid = $props.id();
    const resume = steps.findLastIndex(interactive) + 1;

    let card = $state<HTMLDivElement>();
    let spot = $state<HTMLButtonElement>();
    let view_width = $state(0);
    let view_height = $state(0);
    let card_width = $state(0);
    let card_height = $state(0);
    let ready = $state(false);
    let measured = false;
    let made = $state<number>();
    let known = new Set<number>();
    let spot_rect = $state<Bounds>();
    let glow_rect = $state<Bounds>();
    let placed = $state<Placed[] | null>(null);
    let note_els = $state<HTMLElement[]>([]);

    const current = $derived(steps[step]);
    const last = $derived(step === steps.length - 1);
    const narrow = $derived(view_width < 760 || view_width < view_height);
    const split = $derived(!!current.notes && placed !== null);

    const target = $derived.by(() => {
        const parts = (current.frame ?? current.focus)?.map(
            (part) => sections[part],
        );
        if (!parts) return sections.all;
        const left = Math.min(...parts.map((b) => b.left));
        const top = Math.min(...parts.map((b) => b.top));
        const right = Math.max(...parts.map((b) => b.left + b.width));
        const bottom = Math.max(...parts.map((b) => b.top + b.height));
        return { left, top, width: right - left, height: bottom - top };
    });

    const side = $derived.by(() => {
        if (current.center) return "center";
        if (current.dialog || narrow) return "bottom";
        const all = sections.all.left + sections.all.width / 2;
        return target.left + target.width / 2 < all ? "right" : "left";
    });

    const position = $derived(
        side === "center"
            ? {
                  x: (view_width - card_width) / 2,
                  y: (view_height - card_height) / 2,
              }
            : side === "bottom"
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

    const spotted = () =>
        document.querySelector<HTMLElement>(
            current.spot === "new"
                ? '[data-tour="new"]'
                : `[data-tour-edit="${made}"]`,
        );

    const same = (a: unknown, b: unknown) =>
        JSON.stringify(a) === JSON.stringify(b);

    function arrange(notes: TourNote[]): Placed[] | null {
        if (narrow) return null;
        const box = document
            .querySelector('[role="dialog"][aria-modal="true"]')
            ?.getBoundingClientRect();
        if (!box) return null;
        const room = {
            left: box.left - 2 * margin,
            right: view_width - box.right - 2 * margin,
        };
        if (notes.some((note) => room[note.side] < note_min)) return null;
        const width = {
            left: Math.min(note_width, room.left),
            right: Math.min(note_width, room.right),
        };
        const items = notes.map((note, i) => {
            const rect = document
                .querySelector(`[data-tour="${note.anchor}"]`)
                ?.getBoundingClientRect();
            return {
                side: note.side,
                center: rect ? rect.top + rect.height / 2 : view_height / 2,
                height: note_els[i]?.offsetHeight ?? 0,
                y: 0,
            };
        });
        for (const column of ["left", "right"]) {
            const stack = items
                .filter((item) => item.side === column)
                .sort((a, b) => a.center - b.center);
            let floor = margin;
            for (const item of stack) {
                item.y = Math.max(item.center - item.height / 2, floor);
                floor = item.y + item.height + gap;
            }
            let ceiling = view_height - margin;
            for (const item of stack.reverse()) {
                item.y = Math.min(item.y, ceiling - item.height);
                ceiling = item.y - gap;
            }
        }
        return items.map((item) => ({
            x:
                item.side === "left"
                    ? box.left - margin - width.left
                    : box.right + margin,
            y: item.y,
            width: width[item.side],
            arrow: Math.min(
                Math.max(item.center - item.y, 16),
                item.height - 16,
            ),
            measured: item.height > 0,
        }));
    }

    $effect(() => {
        const now = current;
        untrack(() => {
            spot_rect = undefined;
            glow_rect = undefined;
            placed = null;
        });
        if (!now.spot && !now.notes && !now.glow) return;
        if (now.spot === "edit")
            untrack(spotted)?.scrollIntoView({ block: "nearest" });
        let frame = 0;
        const loop = () => {
            untrack(() => {
                if (now.spot) {
                    const rect = spotted()?.getBoundingClientRect();
                    const next = rect && {
                        left: rect.left,
                        top: rect.top,
                        width: rect.width,
                        height: rect.height,
                    };
                    if (!same(next, spot_rect)) spot_rect = next;
                }
                if (now.glow && now.spot) {
                    const pad = 10;
                    const next = spot_rect && {
                        left: spot_rect.left - pad,
                        top: spot_rect.top - pad,
                        width: spot_rect.width + pad * 2,
                        height: spot_rect.height + pad * 2,
                    };
                    if (!same(next, glow_rect)) glow_rect = next;
                } else if (now.glow) {
                    const node = document.querySelector(`[data-node="${token}"]`);
                    const center = node?.getBoundingClientRect();
                    const cog = node?.firstElementChild?.getBoundingClientRect();
                    const size = (cog?.width ?? 0) * 1.3;
                    const next = center && {
                        left: center.left - size / 2,
                        top: center.top - size / 2,
                        width: size,
                        height: size,
                    };
                    if (!same(next, glow_rect)) glow_rect = next;
                }
                if (now.notes) {
                    const next = arrange(now.notes);
                    if (!same(next, placed)) placed = next;
                }
            });
            frame = requestAnimationFrame(loop);
        };
        loop();
        return () => cancelAnimationFrame(frame);
    });

    $effect(() => {
        const open = dialog;
        untrack(() => {
            if (current.spot === "new" && open?.kind === "new") {
                known = new Set(projects.map((project) => project.id));
                step += 1;
            } else if (current.dialog === "new" && !open) {
                const created = projects.find(
                    (project) => !known.has(project.id),
                );
                if (created) made = created.id;
                step += created ? 1 : -1;
            } else if (current.spot === "edit" && open?.kind === "edit") {
                step += 1;
            } else if (current.dialog === "edit" && !open) {
                step += 1;
            }
        });
    });

    $effect(() => {
        if (current.dialog || !card_width || !card_height) return;
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
        if (!current.dialog || split || !card_height) return;
        const root = document.documentElement;
        root.style.setProperty("--tour-space", `${card_height + margin}px`);
        return () => root.style.removeProperty("--tour-space");
    });

    $effect(() => {
        step;
        tick().then(() =>
            (spot ?? card?.querySelector<HTMLElement>("[data-next]"))?.focus(),
        );
    });

    function next() {
        if (last) onclose();
        else step += 1;
    }

    function press() {
        dialog =
            current.spot === "new"
                ? { kind: "new" }
                : { kind: "edit", id: made ?? projects[projects.length - 1].id };
    }
</script>

<svelte:window
    bind:innerWidth={view_width}
    bind:innerHeight={view_height}
    onkeydown={(e) => e.key === "Escape" && !current.dialog && onclose()}
/>

{#if current.glow && glow_rect}
    <div
        class="glow"
        class:round={!current.spot}
        style:left="{glow_rect.left}px"
        style:top="{glow_rect.top}px"
        style:width="{glow_rect.width}px"
        style:height="{glow_rect.height}px"
        aria-hidden="true"
    ></div>
{/if}

{#if current.spot}
    <button
        bind:this={spot}
        type="button"
        class="spot"
        class:hidden={!spot_rect}
        aria-label={current.spot === "new" ? "New project" : "Edit project"}
        style:left="{(spot_rect?.left ?? 0) - 6}px"
        style:top="{(spot_rect?.top ?? 0) - 6}px"
        style:width="{(spot_rect?.width ?? 0) + 12}px"
        style:height="{(spot_rect?.height ?? 0) + 12}px"
        onclick={press}
    ></button>
{/if}

{#if current.notes && placed}
    {#each current.notes as note, i (note.anchor)}
        {@const at = placed[i]}
        <div
            class="note {note.side}"
            class:hidden={!at?.measured}
            bind:this={note_els[i]}
            style:transform="translate({at?.x ?? 0}px, {at?.y ?? 0}px)"
            style:width="{at?.width ?? note_width}px"
            style:--arrow="{at?.arrow ?? 0}px"
        >
            {#if note.title}
                <h3>{@render note.title()}</h3>
            {/if}
            {@render note.body()}
        </div>
    {/each}
{/if}

<div
    class="card"
    class:ready
    class:gone={split}
    bind:this={card}
    bind:offsetWidth={card_width}
    bind:offsetHeight={card_height}
    style:transform="translate({position.x}px, {position.y}px)"
    style:--duration="{ms}ms"
    role="dialog"
    aria-labelledby={current.title ? `${uid}-title` : undefined}
    aria-label={current.title ? undefined : "Tour"}
>
    {#if !current.dialog}
        <button
            type="button"
            class="close"
            aria-label="Close"
            onclick={onclose}
        >
            <svg viewBox="0 0 10 10" aria-hidden="true">
                <path
                    d="M5 3.94 8.47.47A.75.75 0 0 1 9.53 1.53L6.06 5 9.53 8.47A.75.75 0 0 1 8.47 9.53L5 6.06 1.53 9.53A.75.75 0 0 1 .47 8.47L3.94 5 .47 1.53A.75.75 0 0 1 1.53.47Z"
                />
            </svg>
        </button>
    {/if}
    {#if current.title}
        <h2 id="{uid}-title">{@render current.title()}</h2>
    {/if}
    {@render current.body?.()}
    {#if current.notes && !split}
        {#each current.notes as note (note.anchor)}
            {#if note.title}
                <h3>{@render note.title()}</h3>
            {/if}
            {@render note.body()}
        {/each}
    {/if}
    {#if !current.dialog}
        <div class="actions">
            <MipmapButton
                type="button"
                disabled={step === 0 || interactive(steps[step - 1])}
                onclick={() => (step -= 1)}
            >
                Back
            </MipmapButton>
            {#if !current.spot}
                <MipmapButton type="button" data-next onclick={next}>
                    {last ? "Finish" : "Next"}
                </MipmapButton>
            {:else if replay}
                <MipmapButton
                    type="button"
                    data-next
                    onclick={() => (step = resume)}
                >
                    Skip
                </MipmapButton>
            {/if}
        </div>
    {/if}
</div>

<style>
    .card,
    .note {
        position: fixed;
        top: 0;
        left: 0;
        z-index: 1100;
        box-sizing: border-box;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        line-height: 1.35;
        filter: drop-shadow(4px 6px 6px rgba(0, 0, 0, 0.45));
    }

    .card {
        width: min(28rem, calc(100vw - 2rem));
        max-height: calc(100vh - 2rem);
        overflow-y: auto;
        padding: 1.5rem;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
        visibility: hidden;
    }

    .card.ready {
        visibility: visible;
        transition: transform var(--duration) ease-in-out;
    }

    .card.gone {
        display: none;
    }

    h2,
    h3 {
        margin: 0;
        padding-right: 1.5rem;
        font: inherit;
        font-size: calc(clamp(2rem, 4vw, 3.5rem) * 0.75);
        font-weight: 700;
        line-height: 1.15;
    }

    .card h3 {
        padding-right: 0;
    }

    .card :global(p + h3) {
        margin-top: 1.5rem;
    }

    .card :global(p) {
        margin: 0.75rem 0 0;
        font-size: 0.85em;
    }

    .note {
        padding: 1rem;
        font-size: clamp(1rem, 1.4vw, 1.25rem);
        pointer-events: none;
    }

    .note h3 {
        padding-right: 0;
        font-size: 1.25em;
    }

    .note :global(p) {
        margin: 0.5rem 0 0;
    }

    .note > :global(:first-child) {
        margin-top: 0;
    }

    .note::after {
        content: "";
        position: absolute;
        top: var(--arrow);
        width: 0.75rem;
        height: 0.75rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        transform: translateY(-50%) rotate(45deg);
    }

    .note.left::after {
        right: calc(-0.375rem - 2px);
        border-bottom: none;
        border-left: none;
    }

    .note.right::after {
        left: calc(-0.375rem - 2px);
        border-top: none;
        border-right: none;
    }

    .hidden {
        visibility: hidden;
    }

    .glow {
        position: fixed;
        z-index: 950;
        border-radius: 12px;
    }

    .glow.round {
        border-radius: 50%;
    }

    .glow {
        box-shadow: 0 0 0 200vmax rgba(0, 0, 0, 0.6);
        pointer-events: none;
        animation: dim 0.4s ease-out;
    }

    .spot {
        position: fixed;
        z-index: 950;
        box-sizing: border-box;
        padding: 0;
        background: none;
        border: 3px solid #ead7c2;
        border-radius: 7px;
        cursor: pointer;
        animation: pulse 1.4s ease-out infinite;
    }

    .spot:focus-visible {
        outline: 2px solid #d6ab70;
        outline-offset: 3px;
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

    .card :global(.mip),
    .note :global(.mip) {
        color: #ff9a3d;
    }

    .card :global(.mipmap),
    .note :global(.mipmap) {
        color: #8fcf6b;
    }

    .card :global(.tier),
    .note :global(.tier) {
        color: #ffe066;
    }

    .card :global(.tech-tree),
    .note :global(.tech-tree) {
        color: #7cc4f0;
    }

    @keyframes dim {
        from {
            box-shadow: 0 0 0 200vmax rgba(0, 0, 0, 0);
        }
    }

    @keyframes pulse {
        from {
            box-shadow: 0 0 0 0 rgba(234, 215, 194, 0.7);
        }
        to {
            box-shadow: 0 0 0 14px rgba(234, 215, 194, 0);
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .card.ready {
            transition: none;
        }

        .spot,
        .glow {
            animation: none;
        }
    }
</style>
