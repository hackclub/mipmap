<script lang="ts" module>
    export interface Bounds {
        left: number;
        top: number;
        width: number;
        height: number;
    }

    export interface Inset {
        left: number;
        top: number;
        right: number;
        bottom: number;
    }
</script>

<script lang="ts">
    import { flushSync, untrack, type Snippet } from "svelte";
    import { grid } from "$lib/cog_layout";

    let {
        children,
        home,
        limits,
        keep,
        locked = false,
        targets,
    }: {
        children: Snippet;
        home: Bounds;
        limits?: Bounds;
        keep?: Bounds;
        locked?: boolean;
        targets?: string;
    } = $props();

    const min_scale = 0.1;
    const max_scale = 2;
    const drag_threshold = 6;
    const button_zoom = 1.4;
    const fit_padding = 48;
    const overscroll = 0.25;
    const pan_speed = 900;
    const arrows: Record<string, [number, number]> = {
        ArrowLeft: [-1, 0],
        ArrowRight: [1, 0],
        ArrowUp: [0, -1],
        ArrowDown: [0, 1],
    };

    let viewport: HTMLDivElement;
    let view_width = $state(0);
    let view_height = $state(0);
    let x = $state(0);
    let y = $state(0);
    let scale = $state(1);
    let animating = $state(false);
    let dragging = $state(false);
    let tween = $state({ x: 0, y: 0, scale: 1 });
    let frame = 0;
    let leave_timer = 0;

    const shown = $derived(animating ? tween : { x, y, scale });

    const pointers = new Map<number, { x: number; y: number }>();
    let dragged = false;
    let gesture_start: {
        x: number;
        y: number;
        distance: number;
        mid_x: number;
        mid_y: number;
    } | null = null;

    const clamp = (value: number, low: number, high: number) =>
        Math.min(high, Math.max(low, value));

    function bound(
        position: number,
        start: number,
        size: number,
        view: number,
    ) {
        const pad = view * overscroll;
        const low = view - pad - (start + size) * scale;
        const high = pad - start * scale;
        return low <= high
            ? clamp(position, low, high)
            : clamp(
                  position,
                  view / 2 - (start + size) * scale,
                  view / 2 - start * scale,
              );
    }

    function constrain() {
        if (!limits) return;
        x = bound(x, limits.left, limits.width, view_width);
        y = bound(y, limits.top, limits.height, view_height);
    }

    function zoom_at(factor: number, screen_x: number, screen_y: number) {
        const next = clamp(scale * factor, Math.min(min_scale, scale), max_scale);
        x = screen_x - ((screen_x - x) * next) / scale;
        y = screen_y - ((screen_y - y) * next) / scale;
        scale = next;
        constrain();
    }

    function ease_in_out(p: number) {
        const at = (a: number, b: number, s: number) =>
            3 * a * s * (1 - s) ** 2 + 3 * b * s ** 2 * (1 - s) + s ** 3;
        let [lo, hi] = [0, 1];
        for (let i = 0; i < 20; i++) {
            const mid = (lo + hi) / 2;
            if (at(0.42, 0.58, mid) < p) lo = mid;
            else hi = mid;
        }
        return at(0, 1, (lo + hi) / 2);
    }

    function animate(from: typeof tween, ms = 300) {
        cancelAnimationFrame(frame);
        const start = performance.now();
        tween = from;
        animating = true;
        const step = (now: number) => {
            const t = Math.min(1, (now - start) / ms);
            const e = ease_in_out(t);
            tween = {
                x: from.x + (x - from.x) * e,
                y: from.y + (y - from.y) * e,
                scale: from.scale + (scale - from.scale) * e,
            };
            if (t < 1) frame = requestAnimationFrame(step);
            else animating = false;
        };
        frame = requestAnimationFrame(step);
    }

    function stop() {
        animating = false;
        cancelAnimationFrame(frame);
    }

    const nudge = (start: number, size: number, view: number, pad: number) =>
        start < pad
            ? pad - start
            : start + size > view - pad
              ? view - pad - start - size
              : 0;

    export function show(
        bounds: Bounds,
        {
            smooth = true,
            ms = 300,
            padding = fit_padding,
            cap = 1,
            floor = min_scale,
            free = false,
            within,
            inset = { left: 0, top: 0, right: 0, bottom: 0 },
        }: {
            smooth?: boolean;
            ms?: number;
            padding?: number;
            cap?: number;
            floor?: number;
            free?: boolean;
            within?: Bounds;
            inset?: Inset;
        } = {},
    ) {
        if (!view_width || !view_height) return;
        const from = { ...shown };
        const area_width = view_width - inset.left - inset.right;
        const area_height = view_height - inset.top - inset.bottom;
        const fit = Math.min(
            (area_width - 2 * padding) / bounds.width,
            (area_height - 2 * padding) / bounds.height,
            cap,
        );
        scale = clamp(fit, floor, max_scale);
        x =
            inset.left +
            area_width / 2 -
            (bounds.left + bounds.width / 2) * scale;
        y =
            inset.top +
            (bounds.height * scale <= area_height - 2 * padding
                ? area_height / 2 - (bounds.top + bounds.height / 2) * scale
                : padding - bounds.top * scale);
        if (within) {
            x += nudge(x + within.left * scale, within.width * scale, view_width, padding);
            y += nudge(y + within.top * scale, within.height * scale, view_height, padding);
        }
        if (!free) constrain();
        if (smooth) animate(from, ms);
    }

    export function rect(bounds: Bounds): Bounds {
        return {
            left: shown.x + bounds.left * shown.scale,
            top: shown.y + bounds.top * shown.scale,
            width: bounds.width * shown.scale,
            height: bounds.height * shown.scale,
        };
    }

    export function world(rect: Bounds): Bounds {
        return {
            left: (rect.left - shown.x) / shown.scale,
            top: (rect.top - shown.y) / shown.scale,
            width: rect.width / shown.scale,
            height: rect.height / shown.scale,
        };
    }

    export function settle() {
        const from = { ...shown };
        constrain();
        animate(from);
    }

    function recenter(smooth = true) {
        show(home, { smooth, within: keep });
    }

    function zoom(factor: number) {
        const from = { ...shown };
        zoom_at(factor, view_width / 2, view_height / 2);
        animate(from);
    }

    $effect(() => {
        viewport.addEventListener("wheel", onwheel, { passive: false });
        return () => viewport.removeEventListener("wheel", onwheel);
    });

    $effect(() => {
        if (view_width && view_height) untrack(constrain);
    });

    let homed = false;
    $effect(() => {
        if (!homed && view_width && view_height) {
            homed = true;
            recenter(false);
        }
    });

    const scroller = (e: Event) =>
        (e.target as Element).closest<HTMLElement>("[data-scroll]");

    function onwheel(e: WheelEvent) {
        const own = scroller(e);
        if (
            own &&
            e.deltaY !== 0 &&
            !e.ctrlKey &&
            !e.metaKey &&
            (e.deltaY < 0
                ? own.scrollTop > 0
                : own.scrollTop + own.clientHeight < own.scrollHeight - 1)
        ) {
            return;
        }
        e.preventDefault();
        stop();
        const unit =
            e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? view_height : 1;
        if (e.ctrlKey || e.metaKey) {
            const rect = viewport.getBoundingClientRect();
            zoom_at(
                Math.exp(clamp(-e.deltaY * unit * 0.01, -0.2, 0.2)),
                e.clientX - rect.left,
                e.clientY - rect.top,
            );
        } else if (e.shiftKey && e.deltaX === 0) {
            x -= e.deltaY * unit;
        } else {
            x -= e.deltaX * unit;
            y -= e.deltaY * unit;
        }
        constrain();
    }

    const held = new Set<string>();
    let pan_frame = 0;

    function ignored(e: KeyboardEvent) {
        const target = e.target as Element;
        const own = target.closest<HTMLElement>("[data-scroll]");
        return (
            locked ||
            e.ctrlKey ||
            e.metaKey ||
            e.altKey ||
            !!viewport.closest("[inert]") ||
            !!target.closest("input, textarea, select, [contenteditable]") ||
            (!!own && own.scrollHeight > own.clientHeight)
        );
    }

    function pan() {
        if (pan_frame) return;
        stop();
        let last = performance.now();
        const step = (now: number) => {
            const dt = Math.min(0.05, (now - last) / 1000);
            last = now;
            for (const key of held) {
                x -= arrows[key][0] * pan_speed * dt;
                y -= arrows[key][1] * pan_speed * dt;
            }
            constrain();
            pan_frame = held.size ? requestAnimationFrame(step) : 0;
        };
        pan_frame = requestAnimationFrame(step);
    }

    function jump([dx, dy]: [number, number]) {
        if (!targets) return;
        const centre = (el: Element) => {
            const r = el.getBoundingClientRect();
            return [r.left + r.width / 2, r.top + r.height / 2];
        };
        const active = document.activeElement;
        const current =
            active && viewport.contains(active) && active.matches(targets)
                ? active
                : null;
        const view = viewport.getBoundingClientRect();
        const [ox, oy] = current
            ? centre(current)
            : [view.left + view.width / 2, view.top + view.height / 2];
        let best: HTMLElement | null = null;
        let best_score = Infinity;
        for (const el of viewport.querySelectorAll<HTMLElement>(targets)) {
            if (el === current) continue;
            const [cx, cy] = centre(el);
            const along = (cx - ox) * dx + (cy - oy) * dy;
            if (along <= 1) continue;
            const across = Math.abs((cx - ox) * dy - (cy - oy) * dx);
            const score = along + 2 * across + (across > along ? 1e6 : 0);
            if (score < best_score) {
                best_score = score;
                best = el;
            }
        }
        best?.focus({ focusVisible: true } as FocusOptions);
    }

    function onkeydown(e: KeyboardEvent) {
        if (ignored(e)) return;
        if (e.key in arrows) {
            e.preventDefault();
            if (e.shiftKey) return jump(arrows[e.key]);
            held.add(e.key);
            pan();
        } else if (e.key === "=" || e.key === "+") {
            e.preventDefault();
            zoom(button_zoom);
        } else if (e.key === "-" || e.key === "_") {
            e.preventDefault();
            zoom(1 / button_zoom);
        }
    }

    function onkeyup(e: KeyboardEvent) {
        held.delete(e.key);
    }

    function measure() {
        const points = [...pointers.values()];
        return {
            count: points.length,
            mid_x: points.reduce((sum, p) => sum + p.x, 0) / points.length,
            mid_y: points.reduce((sum, p) => sum + p.y, 0) / points.length,
            distance:
                points.length > 1
                    ? Math.hypot(
                          points[0].x - points[1].x,
                          points[0].y - points[1].y,
                      )
                    : 0,
        };
    }

    function start_gesture() {
        const { mid_x, mid_y, distance } = measure();
        gesture_start = { x: mid_x, y: mid_y, distance, mid_x, mid_y };
    }

    function onpointerdown(e: PointerEvent) {
        if (e.pointerType === "mouse" && e.button !== 0) return;
        if ((e.target as Element).closest("input, textarea, select")) return;
        const own = scroller(e);
        if (
            own &&
            own.scrollHeight > own.clientHeight &&
            (e.pointerType === "touch" ||
                (e.target === own && e.offsetX >= own.clientWidth))
        )
            return;
        stop();
        pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
        if (pointers.size === 1) dragged = false;
        start_gesture();
    }

    function end_drag() {
        clearTimeout(leave_timer);
        leave_timer = 0;
        for (const id of pointers.keys())
            if (viewport.hasPointerCapture(id))
                viewport.releasePointerCapture(id);
        pointers.clear();
        gesture_start = null;
        dragging = false;
        setTimeout(() => (dragged = false));
    }

    function onpointermove(e: PointerEvent) {
        if (!pointers.has(e.pointerId) || !gesture_start) return;
        const mouse = e.pointerType === "mouse";
        if (mouse && !(e.buttons & 1)) return end_drag();
        if (
            e.clientX >= 0 &&
            e.clientY >= 0 &&
            e.clientX <= innerWidth &&
            e.clientY <= innerHeight
        ) {
            clearTimeout(leave_timer);
            leave_timer = 0;
        } else if (!leave_timer) {
            leave_timer = window.setTimeout(end_drag, 500);
        }
        pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
        const { count, mid_x, mid_y, distance } = measure();
        const moved =
            count > 1 ||
            Math.hypot(mid_x - gesture_start.x, mid_y - gesture_start.y) >=
                drag_threshold;
        if (!dragged && !moved && !mouse) return;
        if (!dragging) {
            dragging = true;
            getSelection()?.removeAllRanges();
        }
        if (moved && !dragged) {
            dragged = true;
            for (const id of pointers.keys()) viewport.setPointerCapture(id);
        }
        x += mid_x - gesture_start.mid_x;
        y += mid_y - gesture_start.mid_y;
        if (count > 1 && gesture_start.distance > 0) {
            const rect = viewport.getBoundingClientRect();
            zoom_at(
                distance / gesture_start.distance,
                mid_x - rect.left,
                mid_y - rect.top,
            );
            gesture_start.distance = distance;
        }
        constrain();
        gesture_start.mid_x = mid_x;
        gesture_start.mid_y = mid_y;
    }

    function onpointerup(e: PointerEvent) {
        if (!pointers.delete(e.pointerId)) return;
        if (pointers.size > 0) return start_gesture();
        end_drag();
    }

    function onclickcapture(e: MouseEvent) {
        if (dragged) {
            e.stopPropagation();
            e.preventDefault();
            dragged = false;
        }
    }

    function onscroll() {
        viewport.scrollLeft = viewport.scrollTop = 0;
    }

    function onfocusin(e: FocusEvent) {
        const target = e.target as Element;
        if (target === viewport || !target.matches(":focus-visible")) return;
        onscroll();
        if (animating) {
            stop();
            flushSync();
        }
        const rects = [target, ...target.children].map((el) =>
            el.getBoundingClientRect(),
        );
        const view = viewport.getBoundingClientRect();
        const left = Math.min(...rects.map((r) => r.left)) - view.left;
        const top = Math.min(...rects.map((r) => r.top)) - view.top;
        const right = Math.max(...rects.map((r) => r.right)) - view.left;
        const bottom = Math.max(...rects.map((r) => r.bottom)) - view.top;
        const pad = Math.min(fit_padding, view_width / 4, view_height / 4);
        const dx =
            left < pad
                ? pad - left
                : right > view_width - pad
                  ? view_width - pad - right
                  : 0;
        const dy =
            top < pad
                ? pad - top
                : bottom > view_height - pad
                  ? view_height - pad - bottom
                  : 0;
        if (!dx && !dy) return;
        const from = { ...shown };
        x += dx;
        y += dy;
        constrain();
        animate(from);
    }

    const dot_spacing = $derived(
        grid *
            shown.scale *
            2 ** Math.max(0, Math.ceil(Math.log2(16 / (grid * shown.scale)))),
    );
</script>

<svelte:window {onkeydown} {onkeyup} onblur={() => held.clear()} />

<div
    class="viewport"
    bind:this={viewport}
    bind:clientWidth={view_width}
    bind:clientHeight={view_height}
    style:background-size="{dot_spacing}px {dot_spacing}px"
    style:background-position="{shown.x +
        (grid / 2) * shown.scale -
        dot_spacing / 2}px {shown.y + (grid / 2) * shown.scale - dot_spacing / 2}px"
    class:dragging
    inert={locked}
    {onpointerdown}
    {onpointermove}
    {onpointerup}
    onpointercancel={onpointerup}
    {onclickcapture}
    {onscroll}
    {onfocusin}
    ondragstart={(e) => e.preventDefault()}
    role="presentation"
>
    <div
        class="world"
        class:dragging
        style:transform="translate({shown.x}px, {shown.y}px) scale({shown.scale})"
    >
        {@render children()}
    </div>
</div>

<div
    class="controls"
    class:locked
    inert={locked}
    role="group"
    aria-label="Map controls"
>
    <button type="button" aria-label="Zoom in" onclick={() => zoom(button_zoom)}
        >+</button
    >
    <button
        type="button"
        aria-label="Zoom out"
        onclick={() => zoom(1 / button_zoom)}>−</button
    >
    <button type="button" aria-label="Recenter" onclick={() => recenter()}>
        <svg viewBox="0 0 24 24" width="1em" height="1em" aria-hidden="true">
            <circle
                cx="12"
                cy="12"
                r="7"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
            />
            <circle cx="12" cy="12" r="2.5" fill="currentColor" />
            <path
                d="M12 1v4M12 19v4M1 12h4M19 12h4"
                stroke="currentColor"
                stroke-width="2.5"
            />
        </svg>
    </button>
</div>

<style>
    .viewport {
        position: fixed;
        inset: 0;
        overflow: hidden;
        touch-action: none;
        overscroll-behavior: none;
        background-color: #3d3029;
        background-image: radial-gradient(#5f4838 1.5px, transparent 1.5px);
    }

    .world {
        position: absolute;
        left: 0;
        top: 0;
        transform-origin: 0 0;
    }

    .viewport.dragging {
        user-select: none;
        -webkit-user-select: none;
    }

    .world.dragging {
        will-change: transform;
    }

    .controls {
        position: fixed;
        right: 1rem;
        bottom: 1rem;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        transition: opacity 0.3s;
    }

    .controls.locked {
        opacity: 0.4;
    }

    .controls button {
        display: grid;
        place-items: center;
        width: 3rem;
        height: 3rem;
        padding: 0;
        background: none;
        border: none;
        color: #ead7c2;
        font: inherit;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .controls button + button {
        border-top: 2px solid #4f2f1f;
    }

    .controls button:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .controls button:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }
</style>
