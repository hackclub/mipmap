<script lang="ts">
    import MipmapButton from "$lib/MipmapButton.svelte";
    import { hours, upload_screenshot } from "$lib/projects/api";
    import type { Project } from "$lib/projects/types";

    let {
        project,
        seconds,
        onedit,
        onship,
        onupdate,
    }: {
        project: Project;
        seconds: number;
        onedit: () => void;
        onship: () => void;
        onupdate: (project: Project) => void;
    } = $props();

    let picker = $state<HTMLInputElement>();
    let pending = $state(false);
    let error = $state("");

    async function upload(e: Event & { currentTarget: HTMLInputElement }) {
        const file = e.currentTarget.files?.[0];
        e.currentTarget.value = "";
        if (!file || pending) return;
        if (!file.type.startsWith("image/")) {
            error = "Please choose an image file.";
            return;
        }
        pending = true;
        error = "";
        try {
            onupdate(await upload_screenshot(project.id, file));
        } catch (e) {
            error = e instanceof Error ? e.message : "Upload failed";
        } finally {
            pending = false;
        }
    }
</script>

<article class="tile">
    <h3>{project.name}</h3>
    {#if project.description}
        <p class="description">{project.description}</p>
    {/if}
    <div class="footer">
        <button
            type="button"
            class="edit"
            aria-label="Edit {project.name}"
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
        disabled={pending}
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
        accept="image/*"
        hidden
        onchange={upload}
    />
    <div class="ship">
        <MipmapButton type="button" onclick={onship}>Ship</MipmapButton>
    </div>
    {#if error}
        <div class="error">{error}</div>
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

    .edit:hover,
    .edit:focus-visible,
    .shot:hover:not(:disabled) span,
    .shot:focus-visible span {
        background-color: #ead7c2;
        color: #4f2f1f;
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
