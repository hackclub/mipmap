<script lang="ts">
    let {
        label,
        disabled = false,
        autofocus = false,
        onconfirm,
    }: {
        label: string;
        disabled?: boolean;
        autofocus?: boolean;
        onconfirm: () => void;
    } = $props();

    const uid = $props.id();

    const duration = 1500;
    const drain_time = 300;
    const max_step = 100;
    const max_shake = 4;

    let progress = $state(0);
    let shake = $state("");
    let width = $state(0);
    let height = $state(0);
    let holding = false;
    let frame_id = 0;
    let last = 0;

    function tick(now: number) {
        const step = Math.max(0, Math.min(now - last, max_step));
        last = now;
        if (holding) {
            progress = Math.min(1, progress + step / duration);
            if (progress >= 1) {
                holding = false;
                onconfirm();
            }
        } else {
            progress = Math.max(0, progress - step / drain_time);
        }
        const amplitude = progress * max_shake;
        shake =
            progress > 0
                ? `translate(${amplitude * Math.sin(now * 0.09)}px, ${amplitude * Math.sin(now * 0.13) * 0.5}px) rotate(${amplitude * 0.4 * Math.sin(now * 0.11)}deg)`
                : "";
        frame_id = holding || progress > 0 ? requestAnimationFrame(tick) : 0;
    }

    function run() {
        if (frame_id) return;
        last = performance.now();
        frame_id = requestAnimationFrame(tick);
    }

    function press() {
        if (disabled) return;
        holding = true;
        run();
    }

    function release() {
        holding = false;
        run();
    }

    type Strip = {
        x: number;
        y: number;
        w: number;
        h: number;
        grow: "right" | "down" | "left" | "up";
    };

    const strips = $derived.by(() => {
        const band = height / 4;
        const result: Strip[] = [];
        let [x0, y0, x1, y1] = [0, 0, width, height];
        while (band > 0 && x1 > x0 && y1 > y0) {
            result.push({
                x: x0,
                y: y0,
                w: x1 - x0,
                h: Math.min(band, y1 - y0),
                grow: "right",
            });
            y0 += band;
            if (y1 <= y0) break;
            result.push({
                x: Math.max(x1 - band, x0),
                y: y0,
                w: Math.min(band, x1 - x0),
                h: y1 - y0,
                grow: "down",
            });
            x1 -= band;
            if (x1 <= x0) break;
            result.push({
                x: x0,
                y: Math.max(y1 - band, y0),
                w: x1 - x0,
                h: Math.min(band, y1 - y0),
                grow: "left",
            });
            y1 -= band;
            if (y1 <= y0) break;
            result.push({
                x: x0,
                y: y0,
                w: Math.min(band, x1 - x0),
                h: y1 - y0,
                grow: "up",
            });
            x0 += band;
        }
        return result;
    });

    const spiral = $derived.by(() => {
        let remaining = progress * width * height;
        let d = "M0 0Z";
        for (const { x, y, w, h, grow } of strips) {
            if (remaining <= 0) break;
            const part = Math.min(1, remaining / (w * h));
            remaining -= w * h;
            const [pw, ph] =
                grow === "right" || grow === "left"
                    ? [w * part, h]
                    : [w, h * part];
            const px = grow === "left" ? x + w - pw : x;
            const py = grow === "up" ? y + h - ph : y;
            d += `M${px} ${py}h${pw}v${ph}h${-pw}Z`;
        }
        return `path('${d}')`;
    });

    function is_activation_key(e: KeyboardEvent) {
        return e.key === " " || e.key === "Enter";
    }

    $effect(() => {
        if (disabled) {
            holding = false;
            progress = 0;
        }
    });

    $effect(() => () => cancelAnimationFrame(frame_id));
</script>

<button
    type="button"
    {disabled}
    data-autofocus={autofocus || undefined}
    aria-describedby="{uid}-hint"
    onpointerdown={(e) => {
        if (e.button === 0) press();
    }}
    onpointerup={release}
    onpointercancel={release}
    onpointerleave={release}
    onkeydown={(e) => {
        if (!is_activation_key(e)) return;
        e.preventDefault();
        if (!e.repeat) press();
    }}
    onkeyup={(e) => {
        if (!is_activation_key(e)) return;
        e.preventDefault();
        release();
    }}
    onclick={(e) => {
        if (e.detail === 0 && !disabled) onconfirm();
    }}
    onblur={release}
    oncontextmenu={(e) => e.preventDefault()}
    style:transform={shake}
    bind:clientWidth={width}
    bind:clientHeight={height}
>
    <span>{label}</span>
    <span class="fill" aria-hidden="true" style:clip-path={spiral}>
        <span>{label}</span>
    </span>
</button>
<span id="{uid}-hint" class="hint">Press and hold to confirm</span>

<style>
    button {
        position: relative;
        display: inline-grid;
        place-items: center;
        background-color: #4f2f1f;
        color: #ead7c2;
        border: none;
        border-radius: 0;
        padding: 0.75rem 1.5rem;
        font: inherit;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
        font-weight: 700;
        white-space: nowrap;
        cursor: pointer;
        touch-action: none;
        user-select: none;
        -webkit-user-select: none;
        -webkit-touch-callout: none;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    button:not(:disabled):hover,
    button:not(:disabled):focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    button:disabled {
        cursor: not-allowed;
        opacity: 0.5;
    }

    .fill {
        position: absolute;
        inset: 0;
        display: grid;
        place-items: center;
        background-color: #d6ab70;
        color: #4f2f1f;
        pointer-events: none;
    }

    .hint {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }
</style>
