<script lang="ts">
    import { image_types } from "$lib/api";
    import { dashboard } from "$lib/dashboard/state.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { upload_screenshot, uploading } from "$lib/projects/api";
    import type { Project } from "$lib/projects/types";
    import { days_left, hours } from "$lib/format";
    import { Task } from "$lib/load.svelte";
    import { rounds } from "$lib/fulfillment/rounds";

    let {
        project,
        seconds,
        onedit,
        onship,
    }: {
        project: Project;
        seconds: number;
        onedit: () => void;
        onship: () => void;
    } = $props();

    const round = $derived.by(() => {
        const shipped_on = project.shipped_at?.slice(0, 10) ?? "";
        return rounds.find((round) => round.ship_by >= shipped_on);
    });

    const queued = $derived(
        project.status === "shipped" && project.awaiting_id,
    );
    const slack = $derived(dashboard.data?.user.slack_linked ?? true);

    const status_line = $derived.by(() => {
        if (project.status === "draft") return "";
        if (project.status === "rejected")
            return slack
                ? "Rejected, check your Slack DMs for more details"
                : "Rejected";
        if (queued) return "Waiting for ID verification";
        const parts =
            project.status === "shipped"
                ? ["In review"]
                : ["Approved", `${project.mips_awarded} Mips earned`];
        if (round) {
            parts.unshift(`Round ${rounds.indexOf(round) + 1}`);
            if (project.status === "shipped") {
                const today = new Date().toISOString().slice(0, 10);
                parts.push(
                    today > round.review_by
                        ? "review running late"
                        : `reviewed in ${today >= round.ship_by ? days_left(round.review_by) : "--"}`,
                );
            }
        } else if (project.status === "shipped") {
            parts.push("Next round TBA");
        }
        return parts.join(" · ");
    });

    let picker = $state<HTMLInputElement>();
    const upload = new Task("Upload failed");
    let uploaded = $state(false);
    const busy = $derived(upload.pending || uploading.has(project.id));

    function autoscroll(text: HTMLElement) {
        const frame = text.parentElement!;
        const still = matchMedia("(prefers-reduced-motion: reduce)");
        let animation: Animation | undefined;
        const shot = frame.closest("button")!;
        const held = { hover: false, focus: false };
        const sync = () => {
            if (held.hover || held.focus) {
                if (animation?.playState === "running") animation.pause();
            } else if (animation?.playState === "paused") animation.play();
        };
        const track = (key: keyof typeof held, value: boolean) => () => {
            held[key] = value;
            sync();
        };
        const listeners: [string, () => void][] = [
            ["pointerenter", track("hover", true)],
            ["pointerleave", track("hover", false)],
            ["focus", track("focus", true)],
            ["blur", track("focus", false)],
        ];
        const update = () => {
            animation?.cancel();
            const distance = text.offsetHeight - frame.clientHeight;
            if (distance <= 0 || still.matches) return;
            const hold = 1000;
            const scroll = Math.max(1500, (distance / 25) * 1000);
            const total = 2 * hold + scroll;
            const down = `translateY(${-distance}px)`;
            animation = text.animate(
                [
                    { transform: "translateY(0)", offset: 0 },
                    { transform: "translateY(0)", offset: hold / total, easing: "ease-in-out" },
                    { transform: down, offset: (hold + scroll) / total },
                    { transform: down, offset: 1 },
                ],
                { duration: total, iterations: 2 },
            );
            sync();
        };
        const observer = new ResizeObserver(update);
        observer.observe(frame);
        observer.observe(text);
        still.addEventListener("change", update);
        for (const [type, listener] of listeners)
            shot.addEventListener(type, listener);
        return {
            destroy() {
                observer.disconnect();
                still.removeEventListener("change", update);
                for (const [type, listener] of listeners)
                    shot.removeEventListener(type, listener);
                animation?.cancel();
            },
        };
    }

    async function pick(e: Event & { currentTarget: HTMLInputElement }) {
        const file = e.currentTarget.files?.[0];
        e.currentTarget.value = "";
        if (!file || busy) return;
        uploaded = false;
        await upload.run(
            () => upload_screenshot(project.id, file),
            () => (uploaded = true),
        );
    }
</script>

<article
    class="tile"
    class:with-status={status_line}
    data-tour-tile={project.id}
>
    <h3>{project.name}</h3>
    {#if project.description}
        <p class="description">{project.description}</p>
    {/if}
    {#if status_line}
        <p class="status">
            {status_line}
            {#if queued}
                <span class="note">
                    It's sent for review as soon as your ID is verified. Ships
                    wait up to 30 days, then get cancelled
                </span>
            {:else if project.status === "shipped" && round}
                <span class="note">
                    Reviews start once the round's ship-by date passes
                </span>
            {:else if project.status === "rejected"}
                <span class="note">
                    {#if slack}
                        Not seeing a DM? Make sure <a href="/notifications"
                            >Slack DMs</a
                        > are turned on
                    {:else}
                        Reviewers send the reason as a Slack DM, but your Hack
                        Club account has no Slack linked. Link one in <a
                            href="https://auth.hackclub.com"
                            target="_blank"
                            rel="noopener">Hack Club Auth</a
                        >, then log out and back in to get DMs
                    {/if}
                </span>
            {/if}
        </p>
    {/if}
    <div class="footer">
        <button
            type="button"
            class="edit"
            aria-label="Edit {project.name}"
            data-tour-edit={project.id}
            onclick={onedit}
        >
            <svg
                viewBox="0 0 24 24"
                width="1em"
                height="1em"
                aria-hidden="true"
            >
                <path
                    fill="currentColor"
                    d="M20.052,3.948a3.234,3.234,0,0,1,0,4.575L18.471,10.1,13.9,5.529l1.581-1.581A3.234,3.234,0,0,1,20.052,3.948ZM8.438,20.138l8.619-8.62L12.482,6.943l-8.62,8.619L3,21Z"
                />
            </svg>
        </button>
        <span>{hours(seconds)} tracked in total</span>
    </div>
    <button
        type="button"
        class="shot"
        aria-label="Upload screenshot for {project.name}"
        disabled={busy || project.status === "shipped"}
        onclick={() => picker?.click()}
    >
        {#if busy}
            <span class="placeholder" aria-hidden="true">
                <span>Uploading…</span>
            </span>
        {:else if project.screenshot}
            <img src={project.screenshot} alt="" />
        {:else}
            <span class="placeholder">
                <span use:autoscroll>
                    Upload a screenshot that demonstrates your project working.
                    It will only be visible to Hack Club reviewers
                </span>
            </span>
        {/if}
    </button>
    <input
        bind:this={picker}
        type="file"
        accept={image_types}
        hidden
        onchange={pick}
    />
    <div class="ship" data-tour-ship={project.id}>
        {#if project.status === "shipped"}
            <MipmapButton type="button" disabled>In review</MipmapButton>
        {:else}
            <MipmapButton type="button" disabled={busy} onclick={onship}>
                {project.status === "draft" ? "Ship" : "Re-ship"}
            </MipmapButton>
            {#if project.status !== "draft"}
                <span class="reship">Pays out hours since last ship</span>
            {/if}
        {/if}
    </div>
    <p class="sr" role="status">
        {busy
            ? `Uploading screenshot for ${project.name}`
            : uploaded
              ? `Screenshot for ${project.name} uploaded`
              : ""}
    </p>
    {#if upload.error}
        <div class="error" role="alert">{upload.error}</div>
    {/if}
</article>

<style>
    .tile {
        display: grid;
        grid-template-columns: minmax(0, 1fr) 16rem;
        grid-template-rows: auto 1fr auto auto;
        grid-template-areas:
            "title shot"
            "description shot"
            "footer ship"
            "error error";
        column-gap: 1.5rem;
        row-gap: 0.75rem;
        padding: 1.5rem;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    .tile.with-status {
        grid-template-rows: auto 1fr auto auto auto;
        grid-template-areas:
            "title shot"
            "description shot"
            "status shot"
            "footer ship"
            "error error";
    }

    h3 {
        grid-area: title;
        margin: 0;
        font: inherit;
        font-weight: 700;
    }

    .description {
        grid-area: description;
        margin: 0;
        font-size: 0.85em;
    }

    .note {
        display: block;
        font-size: 0.85em;
        opacity: 0.85;
    }

    .status {
        grid-area: status;
        margin: 0;
        font-size: 0.85em;
    }

    .footer {
        grid-area: footer;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        align-self: end;
        font-size: 0.85em;
    }

    .edit {
        display: grid;
        place-items: center;
        flex: none;
        width: 3rem;
        height: 3rem;
        padding: 0;
        background: #4f2f1f;
        color: #ead7c2;
        border: none;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .edit:focus-visible,
    .shot:focus-visible .placeholder {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .edit:hover,
        .shot:hover:not(:disabled) .placeholder {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    .shot {
        grid-area: shot;
        aspect-ratio: 16 / 10;
        padding: 0;
        overflow: hidden;
        background: none;
        border: none;
        font: inherit;
        cursor: pointer;
    }

    .shot:disabled {
        cursor: not-allowed;
        opacity: 0.5;
    }

    .shot img {
        display: block;
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    .placeholder {
        display: flex;
        flex-direction: column;
        justify-content: safe center;
        box-sizing: border-box;
        height: 100%;
        overflow: hidden;
        background: #4f2f1f;
        color: #ead7c2;
        font-size: 0.85em;
        text-align: center;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .placeholder span {
        padding: 1rem;
    }

    .ship {
        grid-area: ship;
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        gap: 0.25rem;
        align-self: end;
    }

    .reship {
        font-size: 0.75em;
        opacity: 0.85;
    }

    .sr {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }

    .error {
        grid-area: error;
        font-size: 0.85em;
    }
</style>
