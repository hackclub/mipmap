<script lang="ts">
    import { onMount, tick, untrack } from "svelte";
    import type PanZoom from "$lib/dashboard/PanZoom.svelte";
    import type { Bounds, Inset } from "$lib/dashboard/PanZoom.svelte";
    import type { ProjectDialog } from "$lib/projects/ProjectsPanel.svelte";
    import type { Project } from "$lib/projects/types";
    import type { TreeNode } from "$lib/tree/types";
    import { message } from "$lib/api";
    import { dashboard } from "$lib/dashboard/state.svelte";
    import { STICKER, STICKER_TOKEN } from "$lib/tree/tutorial";
    import { scroll_focus } from "$lib/ui/scroll_focus";
    import { word_at } from "$lib/words";
    import Avatar from "$lib/onboarding/Avatar.svelte";
    import { finish, grant_mips, join_slack, save_step } from "$lib/onboarding/api";
    import {
        index,
        steps,
        tutorial,
        type Context,
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
        | "challenge"
        | "profile"
        | "help";

    type Shape = Bounds & { round: boolean; quiet?: boolean };

    type Place = {
        layout: "column" | "row";
        side: "left" | "right" | "bottom" | "top" | "center";
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
        slack_linked = false,
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
        slack_linked?: boolean;
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
        "challenge",
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
    let root: HTMLDivElement;
    let next_button = $state<HTMLButtonElement>();
    let visual_top = $state(0);
    let visual_height = $state(0);
    let dialog_open = $state(false);
    let text = $state<HTMLDivElement>();
    let choices = $state<HTMLDivElement>();
    let slack_join = $state<Context["slack_join"]>(null);
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
    let creating = false;
    let caught = false;
    let opened: HTMLElement | null = null;
    let settle_until = 0;
    let tilt_timer = 0;
    let reframe_timer = 0;
    let finish_timer = 0;
    let aim_timer = 0;
    let aim_due = 0;
    let furthest = -1;
    let wake: (ms?: number) => void = () => {};

    const reduced = () =>
        matchMedia("(prefers-reduced-motion: reduce)").matches;

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
    const item_wait = (step: Step) =>
        step.wait === "sticker-unlocked" ||
        step.wait === "token-unlocked" ||
        step.wait === "sticker-redeemed";

    const blocking = $derived(!!current?.wait);
    const passthrough = $derived(
        dialog_open && !!current && (!!current.dialog || item_wait(current)),
    );
    const first = $derived(steps.findIndex((step) => !skipped(step)));
    const reserve = $derived(controls_height + margin);
    const narrow = $derived(
        view_width < 760 || view_width < view_height || view_height < 500,
    );
    const squeezed = $derived(
        visual_height > 0 && visual_height < view_height - 80,
    );
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
            Math.min(
                view_height -
                    2 * margin -
                    reserve -
                    (place.layout === "column" ? avatar_size + 14 : 0),
                place.layout === "row" ? view_height * 0.38 : Infinity,
                squeezed ? visual_height * 0.3 : Infinity,
            ),
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
    const bleed = 4000;
    const dim = $derived(
        `M${-bleed} ${-bleed}H${view_width + bleed}V${view_height + bleed}H${-bleed}Z` +
            cuts.map(outline).join(""),
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
            case "sticker":
            case "token":
                return document.querySelector<HTMLElement>(
                    `button[data-node="${hole === "sticker" ? STICKER : STICKER_TOKEN}"]`,
                );
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
            const top: Place = {
                layout: "row",
                side: "top",
                x: margin,
                y: margin + (squeezed ? visual_top : 0),
                width: view_width - 2 * margin,
            };
            if (narrow) return top;
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
            if (!box || !anchor) return top;
            const right = view_width - box.right - 2 * margin;
            const left = box.left - 2 * margin;
            const width = Math.min(column_width, Math.max(left, right));
            if (width < 240) return top;
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
                el.style.scrollMarginTop =
                    place.side === "top"
                        ? `${guide_height + 2 * margin + 24}px`
                        : "24px";
                el.scrollIntoView({
                    block: place.layout === "row" ? "start" : "center",
                    behavior: reduced() ? "instant" : "smooth",
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
            smooth: !reduced(),
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
        slack_linked,
        slack_join,
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

    async function answer(join: boolean) {
        if (!talking || busy) return;
        if (join) {
            busy = true;
            slack_join = (await join_slack()) ? "joined" : "failed";
            busy = false;
        } else slack_join = null;
        go(at + 1);
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
        next_button?.focus({ preventScroll: true });
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
                        text: `Couldn't finish the onboarding: ${message(e)}`,
                        retry: end,
                    };
                return;
            } finally {
                busy = false;
            }
        }
        phase = "out";
        talking = false;
        finish_timer = window.setTimeout(onfinish, 900);
    }

    export async function saved(kind: "new" | "edit") {
        const step = current;
        if (!step) return;
        if (
            !(kind === "new" && step.wait === "created") &&
            !(kind === "edit" && step.wait === "saved")
        )
            return;
        if (kind === "new" && !target) {
            creating = true;
            try {
                for (let tries = 0; tries < 3 && !target; tries++) {
                    await dashboard.run();
                    await tick();
                }
            } finally {
                creating = false;
            }
        }
        if (current === step) go(at + 1);
    }

    $effect(() => {
        tutorial.mips =
            (nodes.get(STICKER)?.price ?? 0) +
            (nodes.get(STICKER_TOKEN)?.price ?? 0);
    });

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
            const delay = phase === "in" ? 1150 : moved ? 700 : 200;
            talk_timer = window.setTimeout(() => (talking = true), delay);
            aim_later(
                step,
                Math.max(delay, moved && !reduced() ? fly_ms + 100 : 0),
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
            if (!switching && !creating) go(Math.max(index(step.resume!), 0));
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
            if (reduced()) {
                demo = new Set(paths.flat());
                return;
            }
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
        const guide_space = guide_height + 2 * margin;
        const top = place.side === "top";
        root.style.setProperty(
            "--tour-space",
            `${top ? reserve : guide_space + reserve}px`,
        );
        if (top) root.style.setProperty("--tour-top", `${guide_space}px`);
        return () => {
            root.style.removeProperty("--tour-space");
            root.style.removeProperty("--tour-top");
        };
    });

    $effect(() => {
        if (!dialog_open || !current || !item_wait(current)) return;
        const timer = setTimeout(() => {
            const open = modal();
            if (!open) return;
            open.style.scrollMarginBottom = `${reserve + margin}px`;
            open.scrollIntoView({
                block: "end",
                behavior: reduced() ? "instant" : "smooth",
            });
        }, 400);
        return () => clearTimeout(timer);
    });

    function usable(target: Node | null) {
        const step = current;
        if (!target || !step) return false;
        if (controls.contains(target) || choices?.contains(target)) return true;
        if (!blocking && !step.interactive) return false;
        const open = modal();
        if (item_wait(step) && open?.contains(target)) return true;
        return (step.holes ?? []).some((hole) =>
            (hole === "dialog" ? open : element(hole))?.contains(target),
        );
    }

    function guard(e: Event) {
        if (!passthrough || phase === "out") return;
        const target = e.target as Node | null;
        if (root.contains(target) || usable(target)) return;
        e.stopPropagation();
        if (e.type === "pointerdown") {
            picking = !!modal()?.querySelector('[aria-expanded="true"]');
            return;
        }
        e.preventDefault();
        if (e.type === "mousedown") {
            const active = document.activeElement;
            if (active instanceof HTMLElement && modal()?.contains(active)) active.blur();
        }
        if (e.type === "click" && current?.interactive && !picking) next();
    }

    function onkeydown(e: KeyboardEvent) {
        if (at < 0 || phase === "out" || e.key === "Escape") return;
        const target = e.target as Element | null;
        if (usable(target)) return;
        if (target && root.contains(target) && target.closest("a, button"))
            return;
        if (
            e.ctrlKey ||
            e.metaKey ||
            e.altKey ||
            /^(Shift|Control|Alt|Meta|Tab|Arrow\w+|Page\w+|Home|End|F\d+|[-+=_])$/.test(
                e.key,
            )
        )
            return;
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat && (e.key === " " || e.key === "Enter")) next();
    }

    function aim_later(step: Step, ms: number) {
        clearTimeout(aim_timer);
        aim_due = performance.now() + ms;
        aim_timer = window.setTimeout(() => aim(step), ms);
    }

    function aim(step: Step) {
        if (current !== step || phase === "out" || modal()) return;
        if (step.wait === "slack-answer") return;
        const active = document.activeElement;
        const hole = step.wait && step.holes?.find((hole) => hole !== "dialog");
        const el = hole && element(hole);
        if (el) {
            if (!el.contains(active)) el.focus({ preventScroll: true });
            return;
        }
        if (
            !active ||
            active === document.body ||
            (root.contains(active) && !active.closest(".text"))
        )
            next_button?.focus({ preventScroll: true });
    }

    const focus = (node: HTMLElement) => node.focus({ preventScroll: true });

    function wait_hint(step: Step) {
        const pick = (tap: string, click: string) => (touch ? tap : click);
        switch (step.wait) {
            case "new-open":
            case "edit-open":
                return pick(
                    "Tap the highlighted button",
                    "Click the highlighted button or press Enter on it",
                );
            case "created":
            case "saved":
                return pick(
                    "Tap the highlighted button when you're ready",
                    "Click the highlighted button or press Enter on it when you're ready",
                );
            case "sticker-unlocked":
            case "token-unlocked":
            case "sticker-redeemed":
                return dialog_open
                    ? pick(
                          "Press and hold the button in the popup",
                          "Press and hold the button in the popup, or hold Space/Enter on it",
                      )
                    : pick(
                          "Tap the highlighted node",
                          "Click the highlighted node or press Enter on it",
                      );
        }
        return "";
    }

    function ontext(e: MouseEvent) {
        const at_point = e.target as Element;
        if (at_point.closest("a")) return;
        if (touch && word_at(e.clientX, e.clientY)) return;
        if (!blocking && !picking) next();
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

    function lost() {
        const edge = 24;
        return measure().some(
            (shape) =>
                shape.left + shape.width < edge ||
                shape.top + shape.height < edge ||
                shape.left > view_width - edge ||
                shape.top > view_height - edge,
        );
    }

    function reframe(camera: boolean, delay: number) {
        clearTimeout(reframe_timer);
        reframe_timer = window.setTimeout(() => {
            if (!current || phase === "out") return;
            if (camera && !lost()) return;
            frame_step(current, camera);
        }, delay);
    }

    const on_camera = (target: EventTarget | null) =>
        target instanceof Element &&
        !!target.closest(".viewport") &&
        !modal()?.contains(target);

    let pressed: { x: number; y: number } | null = null;
    const onwheel = (e: WheelEvent) => {
        wake();
        if (on_camera(e.target)) reframe(true, 400);
    };
    const onpointerdown = (e: PointerEvent) => {
        touch = e.pointerType !== "mouse";
        wake();
        pressed = on_camera(e.target) ? { x: e.clientX, y: e.clientY } : null;
    };
    const onpointerup = (e: PointerEvent) => {
        if (pressed && Math.hypot(e.clientX - pressed.x, e.clientY - pressed.y) > 5)
            reframe(true, 150);
        pressed = null;
    };
    const onresize = () => {
        wake();
        reframe(false, 250);
    };
    const onmotion = (e: Event) => {
        if (e.type !== "pointermove" || (e as PointerEvent).buttons) wake();
    };
    const motion = [
        "keydown",
        "pointermove",
        "pointerup",
        "scroll",
        "transitionrun",
        "animationstart",
    ];

    $effect(() => {
        void [at, phase, dialog, guide_height, controls_height];
        void [view_width, view_height, visual_top, visual_height];
        untrack(() => wake(fly_ms + 400));
    });

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
        const begin = setTimeout(() => go(start), 100);
        const idle = setTimeout(() => {
            if (phase === "in") phase = "idle";
        }, 1400);
        let frame = 0;
        let awake_until = 0;
        const loop = () => {
            frame = 0;
            const open = !!modal();
            if (open !== dialog_open) {
                dialog_open = open;
                if (!open && current)
                    aim_later(current, Math.max(0, aim_due - performance.now()));
            }
            const next_shapes = measure();
            let moving = !same_shapes(next_shapes, shapes);
            if (moving) shapes = next_shapes;
            const next_place = placement();
            if (!same_place(next_place, place)) {
                moving = true;
                if (ready && Math.abs(next_place.x - place.x) > 40) {
                    tilt = next_place.x > place.x ? 10 : -10;
                    clearTimeout(tilt_timer);
                    tilt_timer = window.setTimeout(() => (tilt = 0), 550);
                }
                place = next_place;
            }
            if (moving) awake_until = Math.max(awake_until, performance.now() + 300);
            if (!ready && guide_height && current)
                requestAnimationFrame(() => (ready = true));
            const until = Math.max(awake_until, settle_until);
            if (!ready || performance.now() < until)
                frame = requestAnimationFrame(loop);
        };
        wake = (ms = 600) => {
            awake_until = Math.max(awake_until, performance.now() + ms);
            if (!frame) frame = requestAnimationFrame(loop);
        };
        wake(2000);
        const changes = new MutationObserver((records) => {
            if (
                records.some(
                    (record) =>
                        !root.contains(record.target) &&
                        record.attributeName !== "data-time",
                )
            )
                wake();
        });
        changes.observe(document.body, {
            subtree: true,
            childList: true,
            attributes: true,
            characterData: true,
        });
        const onload = () => wake();
        document.addEventListener("load", onload, true);
        document.fonts.addEventListener("loadingdone", onload);
        const visual = window.visualViewport;
        const onvisual = () => {
            if (!visual) return;
            visual_top = visual.offsetTop;
            visual_height = visual.height;
            const active = document.activeElement;
            if (
                squeezed &&
                current?.dialog &&
                active instanceof HTMLElement &&
                modal()?.contains(active)
            ) {
                active.style.scrollMarginTop = `${guide_height + 2 * margin + 24}px`;
                active.scrollIntoView({ block: "nearest" });
            }
        };
        onvisual();
        visual?.addEventListener("resize", onvisual);
        visual?.addEventListener("scroll", onvisual);
        window.addEventListener("keydown", onkeydown, true);
        for (const type of ["pointerdown", "mousedown", "click"])
            window.addEventListener(type, guard, true);
        window.addEventListener("wheel", onwheel, { capture: true, passive: true });
        window.addEventListener("pointerdown", onpointerdown, true);
        window.addEventListener("pointerup", onpointerup, true);
        window.addEventListener("pointercancel", onpointerup, true);
        window.addEventListener("resize", onresize);
        for (const type of motion)
            window.addEventListener(type, onmotion, { capture: true, passive: true });
        return () => {
            clearTimeout(begin);
            clearTimeout(reframe_timer);
            clearTimeout(finish_timer);
            clearTimeout(aim_timer);
            cancelAnimationFrame(frame);
            changes.disconnect();
            document.removeEventListener("load", onload, true);
            document.fonts.removeEventListener("loadingdone", onload);
            wake = () => {};
            clearTimeout(idle);
            clearTimeout(talk_timer);
            clearTimeout(switch_timer);
            clearTimeout(tilt_timer);
            visual?.removeEventListener("resize", onvisual);
            visual?.removeEventListener("scroll", onvisual);
            window.removeEventListener("keydown", onkeydown, true);
            for (const type of ["pointerdown", "mousedown", "click"])
                window.removeEventListener(type, guard, true);
            window.removeEventListener("wheel", onwheel, true);
            window.removeEventListener("pointerdown", onpointerdown, true);
            window.removeEventListener("pointerup", onpointerup, true);
            window.removeEventListener("pointercancel", onpointerup, true);
            window.removeEventListener("resize", onresize);
            for (const type of motion)
                window.removeEventListener(type, onmotion, true);
        };
    });
</script>

<div
    class="onboarding"
    class:out={phase === "out"}
    role="region"
    aria-label="Onboarding"
    bind:this={root}
    bind:clientWidth={view_width}
    bind:clientHeight={view_height}
    data-above-dialogs
    {@attach portal}
>
    <svg
        class="dim"
        class:passthrough
        width={view_width}
        height={view_height}
        aria-hidden="true"
    >
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
        <div
            class="catch"
            role="presentation"
            onpointerdown={() => (caught = true)}
            onclick={() => {
                if (caught) next();
                caught = false;
            }}
        ></div>
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
            style:max-height="{bubble_max}px"
        >
            <div
                class="text"
                bind:this={text}
                role="presentation"
                data-scroll
                {@attach scroll_focus}
                onpointerdown={() =>
                    (picking = !!modal()?.querySelector('[aria-expanded="true"]'))}
                onclick={ontext}
            >
                <div role="status">
                    {#if current}
                        <p>{@render current.say()}</p>
                        {#if current.sub && (!current.sub_when || (!replay && current.sub_when(context())))}
                            <p class="sub">{@render current.sub()}</p>
                        {/if}
                    {/if}
                </div>
            </div>
            {#if current}
                {#if failure}
                    <p class="hint">
                        {failure.text}. {touch
                            ? "Tap"
                            : "Click or press Space/Enter"} to try again
                    </p>
                {:else if current.wait === "slack-answer"}
                    <div class="choices" bind:this={choices}>
                        <button
                            type="button"
                            disabled={busy}
                            onclick={() => answer(true)}
                            {@attach focus}
                        >
                            Yay
                        </button>
                        <button type="button" disabled={busy} onclick={() => answer(false)}>
                            Nay
                        </button>
                    </div>
                {:else if blocking && !busy}
                    <p class="hint">{wait_hint(current)}</p>
                {:else if !busy}
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
        class:narrow
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
            aria-label={narrow ? "Restart Onboarding" : undefined}
            onclick={() => {
                rewind(first);
                next_button?.focus({ preventScroll: true });
            }}
        >
            {narrow ? "Restart" : "Restart Onboarding"}
        </button>
        <button
            type="button"
            disabled={blocking || busy || phase === "out"}
            bind:this={next_button}
            onclick={next}
        >
            {failure ? "Try again" : "Next"}
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
        overflow: visible;
    }

    .dim {
        animation: fade 0.5s ease-out;
    }

    .dim path {
        fill: rgba(0, 0, 0, 0.6);
        pointer-events: visiblePainted;
    }

    .dim.passthrough path {
        pointer-events: none;
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

    .choices {
        flex: none;
        display: flex;
        gap: 0.5rem;
        margin-top: 0.75rem;
        pointer-events: auto;
    }

    .controls {
        position: absolute;
        right: 1rem;
        bottom: 1rem;
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        gap: 0.5rem;
        max-width: calc(100% - 2rem);
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

    .controls button {
        min-height: 44px;
    }

    .mips span {
        color: #ffb978;
    }

    .controls.narrow .mips,
    .controls.narrow button {
        padding: 0.45rem 0.8rem;
    }

    .choices button {
        padding: 0.4rem 1.1rem;
        background: #5f4838;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font: inherit;
        font-size: 0.9em;
        font-weight: 700;
        min-height: 44px;
    }

    .controls button,
    .choices button {
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .controls button:focus-visible,
    .choices button:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .controls button:hover:not(:disabled),
        .choices button:hover:not(:disabled) {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    .controls button:disabled,
    .choices button:disabled {
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
        opacity: 0.85;
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
