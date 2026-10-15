<script lang="ts" module>
    const open: { node: HTMLElement; restore: HTMLElement[] }[] = [];
    const holds = new Map<Element, number>();

    export function refocus(
        container: HTMLElement | undefined,
        dialog = false,
    ) {
        const active = document.activeElement;
        if (
            active &&
            active !== document.body &&
            !(active instanceof HTMLButtonElement && active.disabled)
        )
            return;
        (
            container?.querySelector<HTMLElement>("button:not(:disabled)") ??
            (dialog
                ? container?.closest<HTMLElement>('[role="dialog"]')
                : undefined)
        )?.focus();
    }
</script>

<script lang="ts">
    import type { Snippet } from "svelte";
    import Cog from "$lib/ui/Cog.svelte";
    import large_cog from "$lib/assets/large-cog.svg";

    let {
        children,
        labelledby,
        corner = "top-left",
        width = "48rem",
        onclose,
    }: {
        children: Snippet;
        labelledby: string;
        corner?: "top-left" | "bottom-right";
        width?: string;
        onclose?: () => void;
    } = $props();

    const big_size = 160;
    const small_size = 80;
    const distance = 110;
    const big_pitch = 360 / 14;
    const small_pitch = 360 / 7;

    const direction = $derived(corner === "top-left" ? -45 : 135);
    const small_x = $derived(distance * Math.sin((direction * Math.PI) / 180));
    const small_y = $derived(-distance * Math.cos((direction * Math.PI) / 180));
    const phase = $derived(
        ((((direction + big_pitch / 2) % big_pitch) + big_pitch) % big_pitch) -
            big_pitch / 2,
    );
    const small_offset = $derived(
        direction + 180 - small_pitch / 2 + phase * (big_size / small_size),
    );

    const fade_ms = 200;
    const spin_ms = 200;
    const reduced = () =>
        matchMedia("(prefers-reduced-motion: reduce)").matches;

    function reversed(x1: number, y1: number, x2: number, y2: number) {
        const at = (a: number, b: number, s: number) =>
            3 * a * s * (1 - s) ** 2 + 3 * b * s ** 2 * (1 - s) + s ** 3;
        return (p: number) => {
            let [lo, hi] = [0, 1];
            for (let i = 0; i < 30; i++) {
                const mid = (lo + hi) / 2;
                if (at(x1, x2, mid) < 1 - p) lo = mid;
                else hi = mid;
            }
            return 1 - at(y1, y2, (lo + hi) / 2);
        };
    }

    const ease_out = reversed(0, 0, 0.58, 1);
    const settle = reversed(0.16, 1, 0.3, 1);

    const fade = (css: (t: number, u: number) => string) => (_: Element) => ({
        delay: reduced() ? 0 : spin_ms - fade_ms,
        duration: reduced() ? 0 : fade_ms,
        easing: ease_out,
        css,
    });
    const dim = fade((t) => `background-color: rgba(0, 0, 0, ${0.45 * t})`);
    const drop = fade(
        (t, u) => `opacity: ${t}; transform: translateY(${-24 * u}px)`,
    );
    const unwind = (_: Element, from: number) => ({
        duration: reduced() ? 0 : spin_ms,
        easing: settle,
        css: (_t: number, u: number) => `transform: rotate(${from * u}deg)`,
    });

    function portal(node: HTMLElement) {
        const active =
            document.activeElement instanceof HTMLElement
                ? document.activeElement
                : null;
        const parent = open.find((entry) => entry.node.contains(active));
        const entry = {
            node,
            restore: [active, ...(parent?.restore ?? [])].filter(
                (el): el is HTMLElement => el !== null,
            ),
        };
        const background = [...document.body.children].filter(
            (el) =>
                !el.hasAttribute("data-above-dialogs") &&
                (holds.has(el) || !el.hasAttribute("inert")),
        );
        document.body.appendChild(node);
        open.push(entry);
        background.forEach((el) => {
            holds.set(el, (holds.get(el) ?? 0) + 1);
            el.setAttribute("inert", "");
        });
        (
            node.querySelector<HTMLElement>(
                "[data-autofocus]:not(:disabled)",
            ) ?? node.querySelector<HTMLElement>(".dialog")
        )?.focus();
        return () => {
            background.forEach((el) => {
                const count = (holds.get(el) ?? 1) - 1;
                if (count > 0) return holds.set(el, count);
                holds.delete(el);
                el.removeAttribute("inert");
            });
            open.splice(open.indexOf(entry), 1);
            node.remove();
            const focused = document.activeElement;
            if (open.some((other) => other.node.contains(focused))) return;
            entry.restore
                .find((el) => el.isConnected && !el.closest("[inert]"))
                ?.focus({ preventScroll: true });
        };
    }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose?.()} />

<div
    class="overlay {corner}"
    {@attach portal}
    out:dim
    role="presentation"
    onpointerdown={(e) => {
        if (e.target === e.currentTarget) onclose?.();
    }}
>
    <div
        class="dialog"
        tabindex="-1"
        out:drop
        style:width="min(90vw, {width})"
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelledby}
    >
        <div class="cogs" aria-hidden="true">
            <div
                class="cog"
                style:left="{-big_size / 2}px"
                style:top="{-big_size / 2}px"
            >
                <div class="spin big" out:unwind={540}>
                    <img
                        src={large_cog}
                        width={big_size}
                        height={big_size}
                        alt=""
                    />
                </div>
            </div>
            <div
                class="cog"
                style:left="{small_x - small_size / 2}px"
                style:top="{small_y - small_size / 2}px"
            >
                <div style:transform="rotate({small_offset}deg)">
                    <div class="spin small" out:unwind={-1080}>
                        <Cog size={small_size} />
                    </div>
                </div>
            </div>
        </div>

        <div class="box">
            {#if onclose}
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
            {@render children()}
        </div>
    </div>
</div>

<style>
    .overlay {
        position: fixed;
        inset: 0;
        z-index: 1000;
        display: flex;
        overflow-y: auto;
        box-sizing: border-box;
        padding: max(5rem, var(--tour-top, 0px)) 1.5rem calc(2rem + var(--tour-space, 0px));
        background-color: rgba(0, 0, 0, 0.45);
        animation: dim 0.4s ease-out;
    }

    .overlay.bottom-right {
        padding: max(2rem, var(--tour-top, 0px)) 1.5rem calc(5rem + var(--tour-space, 0px));
    }

    .dialog {
        position: relative;
        margin: auto;
        outline: none;
        animation: drop 0.4s ease-out;
    }

    .cogs {
        position: absolute;
        top: 0;
        left: 0;
        pointer-events: none;
    }

    .bottom-right .cogs {
        top: 100%;
        left: 100%;
    }

    .cog {
        position: absolute;
        line-height: 0;
    }

    .cog img {
        display: block;
    }

    .spin {
        animation: settle 1.2s cubic-bezier(0.16, 1, 0.3, 1);
    }

    .spin.big {
        --from: 540deg;
        background: radial-gradient(
            circle closest-side,
            color-mix(in srgb, #7b4b2a 72%, black) 54%,
            transparent calc(54% + 0.5px)
        );
    }

    .spin.small {
        --from: -1080deg;
    }

    .box {
        position: relative;
        box-sizing: border-box;
        padding: 1.5rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
        font-weight: 400;
        line-height: 1.35;
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

    .box :global(h2) {
        margin: 0;
        font: inherit;
        font-size: calc(clamp(2rem, 4vw, 3.5rem) * 0.75);
        font-weight: 700;
        line-height: 1.15;
    }

    @keyframes dim {
        from {
            background-color: rgba(0, 0, 0, 0);
        }
    }

    @keyframes drop {
        from {
            opacity: 0;
            transform: translateY(-24px);
        }
    }

    @keyframes settle {
        from {
            transform: rotate(var(--from));
        }
        to {
            transform: rotate(0deg);
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .overlay,
        .dialog,
        .spin {
            animation: none;
        }
    }
</style>
