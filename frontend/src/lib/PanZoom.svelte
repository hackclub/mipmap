<script lang="ts" module>
    export interface Bounds {
        left: number;
        top: number;
        width: number;
        height: number;
    }
</script>

<script lang="ts">
    import { flushSync, untrack, type Snippet } from "svelte";
    import { grid } from "$lib/cog_layout";

    let {
        children,
        home,
        min_scale = 0.1,
        max_scale = 2,
        limits,
        keep,
    }: {
        children: Snippet;
        home: Bounds;
        min_scale?: number;
        max_scale?: number;
        limits?: Bounds;
        keep?: Bounds;
    } = $props();

    const drag_threshold = 6;
    const button_zoom = 1.4;
    const fit_padding = 48;
    const overscroll = 0.25;

    let viewport: HTMLDivElement;
    let view_width = $state(0);
    let view_height = $state(0);
    let x = $state(0);
    let y = $state(0);
    let scale = $state(1);
    let animating = $state(false);
    let dragging = $state(false);
    let duration = $state(300);
    let animation_timer = 0;
    let leave_timer = 0;

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
        const next = clamp(scale * factor, min_scale, max_scale);
        x = screen_x - ((screen_x - x) * next) / scale;
        y = screen_y - ((screen_y - y) * next) / scale;
        scale = next;
        constrain();
    }

    function animate(ms = 300) {
        duration = ms;
        animating = true;
        clearTimeout(animation_timer);
        animation_timer = window.setTimeout(() => (animating = false), ms);
    }

    function stop() {
        animating = false;
        clearTimeout(animation_timer);
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
            edge = "top",
            smooth = true,
            ms = 300,
            padding = fit_padding,
            cap = 1,
            within,
        }: {
            edge?: "top" | "bottom";
            smooth?: boolean;
            ms?: number;
            padding?: number;
            cap?: number;
            within?: Bounds;
        } = {},
    ) {
        if (!view_width || !view_height) return;
        const fit = Math.min(
            (view_width - 2 * padding) / bounds.width,
            (view_height - 2 * padding) / bounds.height,
            cap,
        );
        scale = clamp(fit, min_scale, max_scale);
        x = view_width / 2 - (bounds.left + bounds.width / 2) * scale;
        y =
            bounds.height * scale <= view_height - 2 * padding
                ? view_height / 2 - (bounds.top + bounds.height / 2) * scale
                : edge === "top"
                  ? padding - bounds.top * scale
                  : view_height -
                    padding -
                    (bounds.top + bounds.height) * scale;
        if (within) {
            x += nudge(x + within.left * scale, within.width * scale, view_width, padding);
            y += nudge(y + within.top * scale, within.height * scale, view_height, padding);
        }
        constrain();
        if (smooth) animate(ms);
    }

    export function recenter(smooth = true) {
        show(home, { smooth, within: keep });
    }

    export function zoom(factor: number) {
        zoom_at(factor, view_width / 2, view_height / 2);
        animate();
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

    function start_gesture() {
        const points = [...pointers.values()];
        const mid_x = points.reduce((sum, p) => sum + p.x, 0) / points.length;
        const mid_y = points.reduce((sum, p) => sum + p.y, 0) / points.length;
        const distance =
            points.length > 1
                ? Math.hypot(
                      points[0].x - points[1].x,
                      points[0].y - points[1].y,
                  )
                : 0;
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
        const points = [...pointers.values()];
        const mid_x = points.reduce((sum, p) => sum + p.x, 0) / points.length;
        const mid_y = points.reduce((sum, p) => sum + p.y, 0) / points.length;
        const moved =
            points.length > 1 ||
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
        if (points.length > 1 && gesture_start.distance > 0) {
            const distance = Math.hypot(
                points[0].x - points[1].x,
                points[0].y - points[1].y,
            );
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
        x += dx;
        y += dy;
        constrain();
        animate();
    }

    const dot_spacing = $derived(
        grid *
            scale *
            2 ** Math.max(0, Math.ceil(Math.log2(16 / (grid * scale)))),
    );
</script>

<div
    class="viewport"
    bind:this={viewport}
    bind:clientWidth={view_width}
    bind:clientHeight={view_height}
    style:background-size="{dot_spacing}px {dot_spacing}px"
    style:background-position="{x + (grid / 2) * scale - dot_spacing / 2}px {y +
        (grid / 2) * scale -
        dot_spacing / 2}px"
    class:animating
    class:dragging
    style:--duration="{duration}ms"
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
        class:animating
        class:dragging
        style:transform="translate({x}px, {y}px) scale({scale})"
    >
        {@render children()}
    </div>
</div>

<div class="controls">
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

    .world.animating,
    .world.dragging {
        will-change: transform;
    }

    .viewport.animating {
        transition:
            background-position var(--duration) ease-in-out,
            background-size var(--duration) ease-in-out;
    }

    .world.animating {
        transition: transform var(--duration) ease-in-out;
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

    .controls button:hover,
    .controls button:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }
</style>
