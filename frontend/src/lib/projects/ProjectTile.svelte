<script lang="ts">
    import { image_error, image_types } from "$lib/api";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { upload_screenshot } from "$lib/projects/api";
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

    const status_line = $derived.by(() => {
        if (project.status === "draft") return "";
        const shipped_on = project.shipped_at?.slice(0, 10) ?? "";
        const round = rounds.find((round) => round.ship_by >= shipped_on);
        const parts =
            project.status === "rejected"
                ? ["Rejected"]
                : project.status === "shipped"
                  ? ["In review"]
                  : ["Approved", `${project.mips_awarded} Mips earned`];
        if (round) {
            parts.unshift(`Round ${rounds.indexOf(round) + 1}`);
            if (project.status === "shipped") {
                const started = new Date().toISOString().slice(0, 10) >= round.ship_by;
                parts.push(
                    `reviewed in ${started ? days_left(round.review_by) : "--"}`,
                );
            }
        }
        return parts.join(" · ");
    });

    let picker = $state<HTMLInputElement>();
    const upload = new Task("Upload failed");

    async function pick(e: Event & { currentTarget: HTMLInputElement }) {
        const file = e.currentTarget.files?.[0];
        e.currentTarget.value = "";
        if (!file || upload.pending) return;
        upload.error = image_error(file);
        if (upload.error) return;
        await upload.run(() => upload_screenshot(project.id, file));
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
            {#if project.status === "shipped"}
                <span class="note">
                    Reviews start once the round's ship-by date passes
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
        <span>{hours(seconds)} tracked</span>
    </div>
    <button
        type="button"
        class="shot"
        aria-label="Upload screenshot for {project.name}"
        disabled={upload.pending || project.status === "shipped"}
        onclick={() => picker?.click()}
    >
        {#if project.screenshot}
            <img src={project.screenshot} alt="" />
        {:else}
            <span>Add screenshot</span>
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
            <MipmapButton type="button" disabled>Reviewing</MipmapButton>
        {:else}
            <MipmapButton type="button" onclick={onship}>
                {project.status === "draft" ? "Ship" : "Re-ship"}
            </MipmapButton>
        {/if}
    </div>
    {#if upload.error}
        <div class="error">{upload.error}</div>
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
        opacity: 0.75;
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
    .shot:focus-visible span {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .edit:hover,
        .shot:hover:not(:disabled) span {
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

    .shot span {
        display: grid;
        place-items: center;
        height: 100%;
        background: #4f2f1f;
        color: #ead7c2;
        font-size: 0.85em;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .ship {
        grid-area: ship;
        display: flex;
        justify-content: flex-end;
        align-self: end;
    }

    .error {
        grid-area: error;
        font-size: 0.85em;
    }
</style>
