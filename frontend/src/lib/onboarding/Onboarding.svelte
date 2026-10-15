<script lang="ts">
    import { onMount, tick, untrack } from "svelte";
    import type PanZoom from "$lib/dashboard/PanZoom.svelte";
    import type { Bounds, Inset } from "$lib/dashboard/PanZoom.svelte";
    import type { ProjectDialog } from "$lib/projects/ProjectsPanel.svelte";
    import type { Project } from "$lib/projects/types";
    import type { TreeNode } from "$lib/tree/types";
    import { message } from "$lib/api";
    import { STICKER, STICKER_TOKEN } from "$lib/tree/tutorial";
    import Avatar from "$lib/onboarding/Avatar.svelte";
    import { finish, grant_mips, save_step } from "$lib/onboarding/api";
    import {
        index,
        steps,
        type Frame,
        type Hole,
        type Step,
    } from "$lib/onboarding/script.svelte";

    type Part =
        | "all"
        | "projects"
        | "tree"
        | "snapshots"
        | "fulfillment"
        | "orders"
        | "referral"
        | "profile"
        | "help";

    type Shape = Bounds & { round: boolean; quiet?: boolean };

    type Place = {
        layout: "column" | "row";
        side: "left" | "right" | "bottom" | "center";
        x: number;
        y: number;
        width: number;
    };

    let {
        sections,
        spots,
        nodes,
        panzoom,
        projects,
        unlocked,
        purchased,
        granted,
        mips,
        sticker_skipped = false,
        returning = false,
        stored = null,
        replay = false,
        dialog = $bindable(null),
        selected = $bindable(null),
        demo = $bindable(new Set()),
        onfinish,
    }: {
        sections: Record<Part, Bounds>;
        spots: { sticker: Bounds; token: Bounds };
        nodes: Map<number, TreeNode>;
        panzoom: PanZoom | undefined;
        projects: Project[];
        unlocked: Set<number>;
        purchased: Set<number>;
        granted: boolean;
        mips: number;
        sticker_skipped?: boolean;
        returning?: boolean;
        stored?: string | null;
        replay?: boolean;
        dialog?: ProjectDialog;
        selected?: number | null;
        demo?: Set<number>;
        onfinish: () => void;
    } = $props();

    const margin = 16;
    const fly_ms = 800;
    const side_parts = [
        "fulfillment",
        "orders",
        "referral",
        "profile",
        "help",
    ] as const;

    let at = $state(-1);
    let phase = $state<"in" | "idle" | "out">("in");
    let talking = $state(false);
    let busy = $state(false);
    let failure = $state<{ text: string; retry: () => unknown } | null>(null);
    let touch = $state(false);
    let ready = $state(false);
    let view_width = $state(0);
    let view_height = $state(0);
    let guide_height = $state(0);
    let controls_height = $state(0);
    let controls: HTMLDivElement;
    let text = $state<HTMLDivElement>();
    let picking = false;
    let shapes = $state<Shape[]>([]);
    let place = $state<Place>({
        layout: "column",
        side: "center",
        x: 0,
        y: 0,
        width: 0,
    });
    let tilt = $state(0);
    let talk_timer = 0;
    let switch_timer = 0;
    let switching = false;
    let opened: HTMLElement | null = null;
    let settle_until = 0;
    let tilt_timer = 0;
    let reframe_timer = 0;
    let furthest = -1;

    const current = $derived<Step | undefined>(steps[at]);
    const target = $derived(
        projects
            .filter(
                (project) =>
                    project.status === "draft" || project.status === "rejected",
            )
            .reduce<Project | undefined>(
                (best, project) =>
                    !best || project.id > best.id ? project : best,
                undefined,
            ),
    );
    const blocking = $derived(!!current?.wait);
    const first = $derived(steps.findIndex((step) => !skipped(step)));
    const reserve = $derived(controls_height + margin);
    const narrow = $derived(view_width < 760 || view_width < view_height);
    const column_width = $derived(
        Math.min(Math.max(352, view_width * 0.24), 520, view_width - 2 * margin),
    );
    const avatar_size = $derived.by(() => {
        const short = Math.min(view_width, view_height);
        return place.layout === "row"
            ? Math.max(72, short * 0.12)
            : Math.min(column_width, Math.max(120, short * 0.26));
    });

    const bubble_max = $derived(
        Math.max(
            120,
            view_height -
                2 * margin -
                reserve -
                (place.layout === "column" ? avatar_size + 14 : 0),
        ),
    );

    const outline = (s: Shape) => {
        if (s.round) {
            const r = s.width / 2;
            return `M${s.left} ${s.top + r}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0Z`;
        }
        const r = Math.min(10, s.width / 2, s.height / 2);
        const [w, h] = [s.width - 2 * r, s.height - 2 * r];
        return `M${s.left + r} ${s.top}h${w}a${r} ${r} 0 0 1 ${r} ${r}v${h}a${r} ${r} 0 0 1 ${-r} ${r}h${-w}a${r} ${r} 0 0 1 ${-r} ${-r}v${-h}a${r} ${r} 0 0 1 ${r} ${-r}Z`;
    };
    const overlaps = (a: Shape, b: Shape) =>
        a.left < b.left + b.width &&
        b.left < a.left + a.width &&
        a.top < b.top + b.height &&
        b.top < a.top + a.height;
    const cuts = $derived(
        shapes.filter(
            (s) => s.quiet || !shapes.some((q) => q.quiet && overlaps(s, q)),
        ),
    );
    const rings = $derived(shapes.filter((s) => !s.quiet));
    const dim = $derived(
        `M0 0H${view_width}V${view_height}H0Z` + cuts.map(outline).join(""),
    );

    const union = (list: Bounds[]): Bounds => {
        const left = Math.min(...list.map((b) => b.left));
        const top = Math.min(...list.map((b) => b.top));
        const right = Math.max(...list.map((b) => b.left + b.width));
        const bottom = Math.max(...list.map((b) => b.top + b.height));
        return { left, top, width: right - left, height: bottom - top };
    };

    const grow = (b: Bounds, pad: number, round = false): Shape => ({
        left: b.left - pad,
        top: b.top - pad,
        width: b.width + 2 * pad,
        height: b.height + 2 * pad,
        round,
    });

    const modal = () =>
        document.querySelector<HTMLElement>(
            '[role="dialog"][aria-modal="true"]',
        );

    function element(hole: Hole) {
        const inside = (selector: string) =>
            document.querySelector<HTMLElement>(
                `[role="dialog"][aria-modal="true"] ${selector}`,
            );
        switch (hole) {
            case "new":
                return document.querySelector<HTMLElement>('[data-tour="new"]');
            case "edit":
                return document.querySelector<HTMLElement>(
                    `[data-tour-edit="${target?.id}"]`,
                );
            case "ship":
                return document.querySelector<HTMLElement>(
                    `[data-tour-ship="${target?.id}"] button`,
                );
            case "name":
                return inside('[data-tour="name"]')?.closest<HTMLElement>(
                    ".field",
                );
            case "create":
            case "save":
                return inside('[data-tour="submit"]');
            case "tier":
                return inside('[data-tour="tier"]');
            case "hackatime":
                return inside('[data-tour="hackatime"]');
        }
    }

    function resolve(hole: Hole): Shape[] {
        if (!panzoom) return [];
        const view = panzoom;
        switch (hole) {
            case "projects":
            case "tree":
            case "snapshots":
                return [grow(view.rect(sections[hole]), 4)];
            case "side":
                return side_parts.map((part) =>
                    grow(view.rect(sections[part]), 4),
                );
            case "sticker":
            case "token": {
                const b = view.rect(spots[hole]);
                return [grow(b, 0, true)];
            }
        }
        if (hole === "dialog") {
            const open = modal();
            return open
                ? [{ ...grow(open.getBoundingClientRect(), 0), quiet: true }]
                : [];
        }
        const el = element(hole);
        if (!el) return [];
        const list =
            hole === "tier" && el.querySelector<HTMLElement>('[role="listbox"]');
        if (list) {
            const [a, b] = [
                el.getBoundingClientRect(),
                list.getBoundingClientRect(),
            ];
            return [grow(union([a, b]), 8)];
        }
        const pad = hole === "tier" || hole === "hackatime" ? 8 : 6;
        const error =
            (hole === "create" || hole === "save") &&
            el.closest("form")?.querySelector<HTMLElement>(".error");
        return [
            grow(el.getBoundingClientRect(), pad),
            ...(error ? [grow(error.getBoundingClientRect(), 4)] : []),
        ];
    }

    const item_wait = (step: Step) =>
        step.wait === "sticker-unlocked" ||
        step.wait === "token-unlocked" ||
        step.wait === "sticker-redeemed";

    function measure() {
        const step = current;
        if (!step || phase === "out") return [];
        const open = step.dialog ? null : modal();
        if (open && item_wait(step)) return [grow(open.getBoundingClientRect(), 0)];
        if (open) return [];
        return (step.holes ?? []).flatMap(resolve);
    }

    function guide_side(step: Step): Place["side"] {
        if (step.center) return "center";
        if (narrow) return "bottom";
        const bounds = step.frame && frame_bounds(step.frame);
        if (!bounds) return "right";
        const middle = bounds.left + bounds.width / 2;
        const all = sections.all.left + sections.all.width / 2;
        return middle > all + 1 ? "left" : "right";
    }

    function placement(): Place {
        const step = current;
        const bottom: Place = {
            layout: "row",
            side: "bottom",
            x: margin,
            y: view_height - guide_height - margin - reserve,
            width: view_width - 2 * margin,
        };
        if (!step || step.center)
            return {
                layout: "column",
                side: "center",
                x: (view_width - column_width) / 2,
                y: Math.max(
                    Math.min(
                        (view_height - guide_height) / 2,
                        view_height - guide_height - margin - reserve,
                    ),
                    margin,
                ),
                width: column_width,
            };
        const open = modal();
        if (step.dialog || (item_wait(step) && open)) {
            if (open !== opened) {
                opened = open;
                settle_until = performance.now() + 1500;
            }
            const settling =
                performance.now() < settle_until &&
                open
                    ?.getAnimations({ subtree: true })
                    .some((a) => a.effect?.getComputedTiming().endTime !== Infinity);
            if (settling) return place;
            const box = open?.getBoundingClientRect();
            const anchor = shapes[0];
            if (narrow || !box || !anchor) return bottom;
            const right = view_width - box.right - 2 * margin;
            const left = box.left - 2 * margin;
            const width = Math.min(column_width, Math.max(left, right));
            if (width < 240) return bottom;
            const y = Math.min(
                Math.max(anchor.top + anchor.height / 2 - guide_height / 2, margin),
                view_height - guide_height - margin - reserve,
            );
            return right >= left
                ? { layout: "column", side: "right", x: box.right + margin, y, width }
                : {
                      layout: "column",
                      side: "left",
                      x: box.left - margin - width,
                      y,
                      width,
                  };
        }
        const side = guide_side(step);
        if (side === "bottom") return bottom;
        return {
            layout: "column",
            side,
            x: side === "right" ? view_width - column_width - margin : margin,
            y: Math.max(
                Math.min(
                    (view_height - guide_height) / 2,
                    view_height - guide_height - margin - reserve,
                ),
                margin,
            ),
            width: column_width,
        };
    }

    function frame_bounds(frame: Frame): Bounds | undefined {
        switch (frame) {
            case "projects":
            case "tree":
            case "snapshots":
                return sections[frame];
            case "home":
                return sections.tree;
            case "side":
                return union(side_parts.map((part) => sections[part]));
            case "start": {
                const both = union([spots.sticker, spots.token]);
                const middle = both.left + both.width / 2;
                return {
                    left: middle - 1300,
                    top: both.top - 900,
                    width: 2600,
                    height: both.height + 1500,
                };
            }
            case "tile": {
                const el = document.querySelector<HTMLElement>(
                    `[data-tour-tile="${target?.id}"]`,
                );
                if (!el || !panzoom) return sections.projects;
                el.scrollIntoView({ block: "nearest" });
                const tile = panzoom.world(el.getBoundingClientRect());
                return {
                    left: tile.left - tile.width / 4,
                    top: tile.top - tile.height / 2,
                    width: tile.width * 1.5,
                    height: tile.height * 2,
                };
            }
        }
    }

    async function frame_step(step: Step, camera = false) {
        if (step.dialog && !camera) {
            await tick();
            await new Promise((done) => setTimeout(done, 350));
            const el = step.holes?.[0] && element(step.holes[0]);
            if (el && current === step) {
                el.style.scrollMarginTop = "24px";
                el.scrollIntoView({
                    block: place.layout === "row" ? "start" : "center",
                    behavior: "smooth",
                });
            }
        }
        if (!step.frame) return;
        await tick();
        const bounds = frame_bounds(step.frame);
        if (!bounds || !panzoom) return;
        const side = guide_side(step);
        const room = column_width + 2 * margin;
        const inset: Inset = {
            left: side === "left" ? room : 0,
            top: 0,
            right: side === "right" ? room : 0,
            bottom: side === "bottom" ? guide_height + 2 * margin + reserve : 0,
        };
        panzoom.show(bounds, {
            ms: fly_ms,
            padding: 32,
            cap: Infinity,
            floor: 0.01,
            free: true,
            inset,
        });
    }

    function skipped(step: Step) {
        if (step.project && !target) return true;
        if (replay && step.replay === false) return true;
        return step.skip?.(context()) ?? false;
    }

    const context = () => ({
        editable: !!target,
        granted,
        returning,
        sticker_skipped,
        unlocked,
        purchased,
    });

    function go(i: number) {
        while (i < steps.length && skipped(steps[i])) i += 1;
        if (i >= steps.length) return end();
        clearTimeout(switch_timer);
        switching = false;
        if (i === at) return;
        if (!talking) {
            at = i;
            return;
        }
        talking = false;
        switching = true;
        switch_timer = window.setTimeout(() => {
            switching = false;
            at = i;
        }, 200);
    }

    async function run_grant() {
        busy = true;
        failure = null;
        try {
            await grant_mips();
        } catch (e) {
            failure = {
                text: `Couldn't hand out the Mips: ${message(e)}`,
                retry: run_grant,
            };
        } finally {
            busy = false;
        }
    }

    function next() {
        if (!talking || busy || phase === "out" || blocking) return;
        if (failure) return failure.retry();
        go(at + 1);
    }

    function rewind(i: number) {
        if (busy || phase === "out") return;
        failure = null;
        if (!steps[i].dialog) dialog = null;
        go(i);
    }

    function back() {
        let i = at - 1;
        while (i > first && skipped(steps[i])) i -= 1;
        rewind(Math.max(i, first));
    }

    async function end() {
        if (phase === "out" || busy) return;
        if (!replay) {
            busy = true;
            failure = null;
            try {
                await finish();
            } catch (e) {
                const missing = steps.findIndex(
                    (step) => step.required && !skipped(step),
                );
                if (missing >= 0) go(missing);
                else
                    failure = {
                        text: `Couldn't finish the tutorial: ${message(e)}`,
                        retry: end,
                    };
                return;
            } finally {
                busy = false;
            }
        }
        phase = "out";
        talking = false;
        setTimeout(onfinish, 900);
    }

    export function saved(kind: "new" | "edit") {
        const step = current;
        if (!step) return;
        if (
            (kind === "new" && step.wait === "created") ||
            (kind === "edit" && step.wait === "saved")
        )
            go(at + 1);
    }

    let previous: Step | undefined;
    $effect(() => {
        const i = at;
        if (i < 0) return;
        untrack(() => {
            const step = steps[i];
            const moved =
                !previous ||
                previous.frame !== step.frame ||
                !!previous.dialog !== !!step.dialog ||
                previous.center !== step.center;
            previous = step;
            selected = null;
            if (text) text.scrollTop = 0;
            if (!replay && i > furthest) {
                furthest = i;
                save_step(step.key);
            }
            talking = false;
            clearTimeout(talk_timer);
            talk_timer = window.setTimeout(
                () => (talking = true),
                phase === "in" ? 1150 : moved ? 700 : 200,
            );
            if (step.grant && !replay) run_grant();
            frame_step(step);
        });
    });

    $effect(() => {
        const step = current;
        if (!step?.wait) return;
        const [open, have, bought, project, dropped] = [
            dialog,
            unlocked,
            purchased,
            target,
            sticker_skipped,
        ];
        untrack(() => {
            const done =
                (step.wait === "new-open" && open?.kind === "new") ||
                (step.wait === "edit-open" &&
                    open?.kind === "edit" &&
                    open.id === project?.id) ||
                (step.wait === "sticker-unlocked" && have.has(STICKER)) ||
                (step.wait === "token-unlocked" && have.has(STICKER_TOKEN)) ||
                (step.wait === "sticker-redeemed" &&
                    (bought.has(STICKER) || dropped));
            if (done) go(at + 1);
        });
    });

    $effect(() => {
        const step = current;
        if (!step?.dialog || !step.resume || dialog) return;
        untrack(() => {
            if (!switching) go(Math.max(index(step.resume!), 0));
        });
    });

    function branches() {
        const below = new Map<number, number[]>();
        for (const node of nodes.values())
            for (const id of node.requires)
                below.set(id, [...(below.get(id) ?? []), node.id]);
        const open = (id: number) => id !== STICKER && id !== STICKER_TOKEN;
        const shuffle = (list: number[]) =>
            list
                .map((id) => [Math.random(), id] as const)
                .sort((a, b) => a[0] - b[0])
                .map(([, id]) => id);
        let roots = [...nodes.values()]
            .filter((node) => node.kind === "origin")
            .flatMap((node) => below.get(node.id) ?? []);
        while (roots.some((id) => !open(id)))
            roots = [
                ...new Set(
                    roots.flatMap((id) => (open(id) ? [id] : (below.get(id) ?? []))),
                ),
            ];
        return shuffle(roots)
            .slice(0, 3)
            .map((root) => {
                const path = [root];
                while (path.length < 8) {
                    const next = (below.get(path.at(-1)!) ?? []).filter(open);
                    if (!next.length) break;
                    path.push(shuffle(next)[0]);
                }
                return path;
            });
    }

    const demoing = $derived(!!current?.demo && phase !== "out");
    $effect(() => {
        if (!demoing) return;
        let timer = 0;
        untrack(() => {
            let paths = branches();
            let depth = 0;
            const advance = () => {
                const longest = Math.max(0, ...paths.map((path) => path.length));
                if (depth > longest) {
                    demo = new Set();
                    paths = branches();
                    depth = 0;
                    timer = window.setTimeout(advance, 500);
                    return;
                }
                depth += 1;
                demo = new Set(paths.flatMap((path) => path.slice(0, depth)));
                timer = window.setTimeout(advance, depth > longest ? 1400 : 450);
            };
            timer = window.setTimeout(advance, 600);
        });
        return () => {
            clearTimeout(timer);
            demo = new Set();
        };
    });

    $effect(() => {
        if (!current || !(current.dialog || item_wait(current))) return;
        if (place.layout !== "row") return;
        const root = document.documentElement;
        root.style.setProperty(
            "--tour-space",
            `${guide_height + 2 * margin + reserve}px`,
        );
        return () => root.style.removeProperty("--tour-space");
    });

    function usable(target: Node | null) {
        const step = current;
        if (!target || !step) return false;
        if (controls.contains(target)) return true;
        if (!blocking && !step.interactive) return false;
        const open = modal();
        if (item_wait(step) && open?.contains(target)) return true;
        return (step.holes ?? []).some((hole) =>
            (hole === "dialog" ? open : element(hole))?.contains(target),
        );
    }

    function onkeydown(e: KeyboardEvent) {
        if (at < 0 || phase === "out") return;
        if (e.key === "Escape") {
            if (modal()) return;
        } else if (usable(e.target as Node | null)) return;
        if (
            e.ctrlKey ||
            e.metaKey ||
            e.altKey ||
            /^(Shift|Control|Alt|Meta|Tab|Arrow\w+|Page\w+|Home|End|F\d+)$/.test(e.key)
        )
            return;
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat && (e.key === " " || e.key === "Enter")) next();
    }

    function portal(node: HTMLElement) {
        document.body.appendChild(node);
        return () => node.remove();
    }

    const same_shapes = (a: Shape[], b: Shape[]) =>
        a.length === b.length &&
        a.every(
            (x, i) =>
                x.left === b[i].left &&
                x.top === b[i].top &&
                x.width === b[i].width &&
                x.height === b[i].height &&
                x.round === b[i].round &&
                x.quiet === b[i].quiet,
        );
    const same_place = (a: Place, b: Place) =>
        a.layout === b.layout &&
        a.side === b.side &&
        a.x === b.x &&
        a.y === b.y &&
        a.width === b.width;

    function reframe(camera: boolean, delay: number) {
        clearTimeout(reframe_timer);
        reframe_timer = window.setTimeout(() => {
            if (current && phase !== "out") frame_step(current, camera);
        }, delay);
    }

    const on_camera = (target: EventTarget | null) =>
        target instanceof Element &&
        !!target.closest(".viewport") &&
        !modal()?.contains(target);

    let pressed: { x: number; y: number } | null = null;
    const onwheel = (e: WheelEvent) => {
        if (on_camera(e.target)) reframe(true, 400);
    };
    const onpointerdown = (e: PointerEvent) => {
        pressed = on_camera(e.target) ? { x: e.clientX, y: e.clientY } : null;
    };
    const onpointerup = (e: PointerEvent) => {
        if (pressed && Math.hypot(e.clientX - pressed.x, e.clientY - pressed.y) > 5)
            reframe(true, 150);
        pressed = null;
    };
    const onresize = () => reframe(false, 250);

    onMount(() => {
        touch = matchMedia("(pointer: coarse)").matches;
        const saved = stored ? index(stored) : -1;
        furthest = replay ? steps.length : saved;
        let start = replay ? 0 : Math.max(saved, 0);
        const resume = steps[start].resume;
        if (resume) start = Math.max(index(resume), 0);
        const unfinished = steps.findIndex(
            (step, i) => i < start && step.required && !skipped(step),
        );
        if (!replay && unfinished >= 0) start = unfinished;
        go(start);
        const idle = setTimeout(() => {
            if (phase === "in") phase = "idle";
        }, 1400);
        let frame = 0;
        const loop = () => {
            const next_shapes = measure();
            if (!same_shapes(next_shapes, shapes)) shapes = next_shapes;
            const next_place = placement();
            if (!same_place(next_place, place)) {
                if (ready && Math.abs(next_place.x - place.x) > 40) {
                    tilt = next_place.x > place.x ? 10 : -10;
                    clearTimeout(tilt_timer);
                    tilt_timer = window.setTimeout(() => (tilt = 0), 550);
                }
                place = next_place;
            }
            if (!ready && guide_height) requestAnimationFrame(() => (ready = true));
            frame = requestAnimationFrame(loop);
        };
        loop();
        window.addEventListener("keydown", onkeydown, true);
        window.addEventListener("wheel", onwheel, { capture: true, passive: true });
        window.addEventListener("pointerdown", onpointerdown, true);
        window.addEventListener("pointerup", onpointerup, true);
        window.addEventListener("pointercancel", onpointerup, true);
        window.addEventListener("resize", onresize);
        return () => {
            clearTimeout(reframe_timer);
            cancelAnimationFrame(frame);
            clearTimeout(idle);
            clearTimeout(talk_timer);
            clearTimeout(switch_timer);
            clearTimeout(tilt_timer);
            window.removeEventListener("keydown", onkeydown, true);
            window.removeEventListener("wheel", onwheel, true);
            window.removeEventListener("pointerdown", onpointerdown, true);
            window.removeEventListener("pointerup", onpointerup, true);
            window.removeEventListener("pointercancel", onpointerup, true);
            window.removeEventListener("resize", onresize);
        };
    });
</script>

<svelte:window bind:innerWidth={view_width} bind:innerHeight={view_height} />

<div
    class="onboarding"
    class:out={phase === "out"}
    data-above-dialogs
    {@attach portal}
>
    <svg class="dim" width={view_width} height={view_height} aria-hidden="true">
        <path
            d={dim}
            fill-rule="evenodd"
            role="presentation"
            onpointerdown={() =>
                (picking = !!modal()?.querySelector('[aria-expanded="true"]'))}
            onclick={() => {
                if (current?.interactive && !picking) next();
            }}
        />
    </svg>
    <div class="rings" aria-hidden="true">
        {#each rings as shape, i (i)}
            <div
                class="ring"
                class:round={shape.round}
                class:pulse={blocking}
                style:width="{shape.width}px"
                style:height="{shape.height}px"
                style:transform="translate({shape.left}px, {shape.top}px)"
                style:--sx={(shape.width + 24) / shape.width}
                style:--sy={(shape.height + 24) / shape.height}
            ></div>
        {/each}
    </div>
    {#if !blocking && !current?.interactive}
        <div class="catch" role="presentation" onclick={next}></div>
    {/if}
    <div
        class="guide {place.layout} {place.side}"
        class:ready
        bind:clientHeight={guide_height}
        style:width="{place.width}px"
        style:transform="translate({place.x}px, {place.y}px)"
    >
        <div
            class="bubble"
            class:shown={talking}
            role="status"
            style:max-height="{bubble_max}px"
        >
            {#if current}
                <div
                    class="text"
                    bind:this={text}
                    role="presentation"
                    onpointerdown={() =>
                        (picking = !!modal()?.querySelector('[aria-expanded="true"]'))}
                    onclick={() => {
                        if (!blocking && !picking) next();
                    }}
                >
                    <p>{@render current.say()}</p>
                    {#if current.sub && (!current.sub_when || (!replay && current.sub_when(context())))}
                        <p class="sub">{@render current.sub()}</p>
                    {/if}
                </div>
                {#if failure}
                    <p class="hint">
                        {failure.text}. {touch ? "Tap" : "Click"} to try again
                    </p>
                {:else if !blocking && !busy}
                    <p class="hint">
                        {touch
                            ? "Tap anywhere to continue"
                            : "Click anywhere or press Space/Enter to continue"}
                    </p>
                {/if}
            {/if}
        </div>
        <div class="avatar" style:width="{avatar_size}px">
            <Avatar size={avatar_size} state={phase} {tilt} />
        </div>
    </div>
    <div
        class="controls"
        bind:this={controls}
        bind:clientHeight={controls_height}
    >
        <div class="mips"><span>{mips}</span> Mips</div>
        {#if replay}
            <button type="button" onclick={end}>Close</button>
        {/if}
        <button type="button" disabled={at <= first || busy} onclick={back}>
            Back
        </button>
        <button
            type="button"
            disabled={at <= first || busy}
            onclick={() => rewind(first)}
        >
            Restart
        </button>
    </div>
</div>

<style>
    .onboarding {
        position: fixed;
        inset: 0;
        z-index: 1100;
        pointer-events: none;
        transition: opacity 0.6s ease-in 0.2s;
    }

    .onboarding.out {
        opacity: 0;
    }

    .dim {
        position: absolute;
        inset: 0;
    }

    .dim {
        animation: fade 0.5s ease-out;
    }

    .dim path {
        fill: rgba(0, 0, 0, 0.6);
        pointer-events: visiblePainted;
    }

    .rings {
        position: absolute;
        inset: 0;
    }

    .ring {
        position: absolute;
        top: 0;
        left: 0;
        box-sizing: border-box;
        border: 2px solid rgba(234, 215, 194, 0.7);
        border-radius: 10px;
    }

    .ring.round {
        border-radius: 50%;
    }

    .ring.pulse::after {
        content: "";
        position: absolute;
        inset: -2px;
        border: 2px solid #ead7c2;
        border-radius: inherit;
        will-change: transform, opacity;
        animation: pulse 1.4s ease-out infinite;
    }

    .catch {
        position: absolute;
        inset: 0;
        pointer-events: auto;
        cursor: pointer;
    }

    .guide {
        position: absolute;
        top: 0;
        left: 0;
        box-sizing: border-box;
        display: flex;
        align-items: center;
        gap: 14px;
        pointer-events: none;
    }

    .guide.ready {
        transition: transform 0.8s ease-in-out;
    }

    .guide.column {
        flex-direction: column;
    }

    .guide.row {
        flex-direction: row-reverse;
    }

    .avatar {
        flex: none;
        aspect-ratio: 1;
    }

    .bubble {
        position: relative;
        box-sizing: border-box;
        width: 100%;
        min-width: 0;
        padding: 1rem 1.25rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font-size: clamp(1.05rem, 1.5vw, 1.4rem);
        line-height: 1.35;
        filter: drop-shadow(4px 6px 6px rgba(0, 0, 0, 0.45));
        opacity: 0;
        transform: translateY(6px) scale(0.97);
        transition:
            opacity 0.2s,
            transform 0.2s;
    }

    .row .bubble {
        flex: 1;
    }

    .bubble {
        display: flex;
        flex-direction: column;
    }

    .text {
        flex: 0 1 auto;
        min-height: 0;
        overflow-y: auto;
        pointer-events: auto;
    }

    .hint {
        flex: none;
    }

    .controls {
        position: absolute;
        right: 1rem;
        bottom: 1rem;
        display: flex;
        gap: 0.5rem;
        pointer-events: auto;
    }

    .mips,
    .controls button {
        padding: 0.6rem 1.1rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font: inherit;
        font-size: clamp(1rem, 1.4vw, 1.3rem);
        font-weight: 700;
    }

    .mips span {
        color: #ff9a3d;
    }

    .controls button {
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .controls button:hover:not(:disabled),
    .controls button:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    .controls button:disabled {
        color: rgba(234, 215, 194, 0.4);
        cursor: default;
    }

    .bubble.shown {
        opacity: 1;
        transform: none;
    }

    .bubble::after {
        content: "";
        position: absolute;
        width: 0.9rem;
        height: 0.9rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
    }

    .column .bubble::after {
        left: 50%;
        bottom: calc(-0.45rem - 2px);
        border-top: none;
        border-left: none;
        transform: translateX(-50%) rotate(45deg);
    }

    .row .bubble::after {
        top: 50%;
        left: calc(-0.45rem - 2px);
        border-top: none;
        border-right: none;
        transform: translateY(-50%) rotate(45deg);
    }

    p {
        margin: 0;
    }

    .bubble :global(ul) {
        margin: 0.5rem 0 0;
        padding-left: 1.1em;
    }

    .bubble :global(li + li) {
        margin-top: 0.35rem;
    }

    .sub {
        margin-top: 0.6rem;
        font-size: 0.8em;
        opacity: 0.85;
    }

    .hint {
        margin-top: 0.6rem;
        font-size: 0.7em;
        opacity: 0.6;
    }

    .bubble :global(.mip) {
        color: #ff9a3d;
    }

    .bubble :global(.mipmap) {
        color: #8fcf6b;
    }

    .bubble :global(.tier) {
        color: #ffe066;
    }

    .bubble :global(.tech-tree) {
        color: #7cc4f0;
    }

    .bubble :global(.token) {
        color: #c3a3e6;
    }

    .bubble :global(.prize) {
        color: #f0788a;
    }

    .bubble :global(.snapshot) {
        color: #e8c58c;
    }

    .bubble :global(.hackatime) {
        color: #d6ab70;
    }

    @keyframes fade {
        from {
            opacity: 0;
        }
    }

    @keyframes pulse {
        from {
            transform: none;
            opacity: 0.9;
        }
        to {
            transform: scale(var(--sx), var(--sy));
            opacity: 0;
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .dim,
        .ring.pulse::after {
            animation: none;
        }

        .guide.ready,
        .bubble {
            transition: none;
        }
    }
</style>
