<script lang="ts">
    import { image_error, image_types } from "$lib/api";
    import Field from "$lib/ui/Field.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import {
        save_username,
        saved_username,
        send_snapshot,
    } from "$lib/snapshots/api";
    import type { SnapshotStatus } from "$lib/snapshots/types";
    import { Task } from "$lib/load.svelte";

    let { status }: { status: SnapshotStatus } = $props();

    let picker = $state<HTMLInputElement>();
    let image = $state<File | null>(null);
    let preview = $state("");
    let caption = $state("");
    let username = $state(saved_username());
    let dragging = $state(false);
    const send = new Task();

    $effect(() => {
        const url = preview;
        return () => {
            if (url) URL.revokeObjectURL(url);
        };
    });

    const ready = $derived(
        !status.sent_today && status.tracked_seconds >= status.required_seconds,
    );
    const filled = $derived(
        Boolean(image && caption.trim() && username.trim()),
    );
    const progress = $derived(
        `${Math.floor(status.tracked_seconds / 60)} / ${Math.floor(status.required_seconds / 60)} minutes`,
    );

    function pick(files: FileList | null | undefined) {
        const file = files?.[0];
        if (!file) return;
        send.error = image_error(file);
        if (send.error) return;
        image = file;
        preview = URL.createObjectURL(file);
    }

    async function submit(e: SubmitEvent) {
        e.preventDefault();
        if (!ready || !image || !filled || send.pending) return;
        const file = image;
        const sent = caption;
        await send.run(
            () => {
                save_username(username.trim());
                return send_snapshot(file, sent.trim(), username.trim());
            },
            () => {
                if (image === file) {
                    image = null;
                    preview = "";
                }
                if (caption === sent) caption = "";
            },
        );
    }
</script>

<form onsubmit={submit}>
    <button
        type="button"
        class="drop"
        class:dragging
        aria-label={image ? "Change image" : undefined}
        onclick={() => picker?.click()}
        ondragover={(e) => {
            e.preventDefault();
            dragging = true;
        }}
        ondragleave={(e) => {
            if (!e.currentTarget.contains(e.relatedTarget as Node | null))
                dragging = false;
        }}
        ondrop={(e) => {
            e.preventDefault();
            dragging = false;
            pick(e.dataTransfer?.files);
        }}
    >
        {#if preview}
            <img src={preview} alt="" />
        {:else}
            <span>Drop an image here or click to upload</span>
        {/if}
    </button>
    <input
        bind:this={picker}
        type="file"
        accept={image_types}
        hidden
        onchange={(e) => {
            pick(e.currentTarget.files);
            e.currentTarget.value = "";
        }}
    />
    <Field
        label="Publicly Displayed Username"
        bind:value={username}
        maxlength={40}
    />
    <div class="caption">
        <Field label="Caption" bind:value={caption} multiline maxlength={500} />
    </div>
    <div class="side">
        <MipmapButton
            type="submit"
            disabled={!ready || !filled || send.pending}
        >
            Send
        </MipmapButton>
        <div class="progress">{progress}</div>
        <div class="note">
            {status.sent_today
                ? "You've gotta wait for tomorrow to send the next one!"
                : "You can still send a snapshot today!"}
        </div>
        {#if send.error}
            <div class="note">{send.error}</div>
        {/if}
    </div>
</form>

<style>
    form {
        display: grid;
        grid-template-columns: minmax(0, 3fr) minmax(0, 4fr);
        grid-template-rows: auto minmax(0, 1fr) auto;
        grid-template-areas:
            "drop username"
            "drop caption"
            "drop side";
        gap: 1.5rem;
        height: 100%;
    }

    form > :global(.field) {
        grid-area: username;
    }

    .caption {
        grid-area: caption;
    }

    .caption,
    .caption :global(.field) {
        display: flex;
        flex-direction: column;
        min-height: 0;
    }

    .caption :global(.field),
    .caption :global(textarea) {
        flex: 1;
        min-height: 0;
        resize: none;
    }

    .drop {
        grid-area: drop;
        display: grid;
        place-items: center;
        min-height: 0;
        padding: 0;
        overflow: hidden;
        background: #4f2f1f;
        color: inherit;
        border: none;
        border-radius: 7px;
        font: inherit;
        font-size: 0.85em;
        text-align: center;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .drop:hover,
    .drop:focus-visible,
    .drop.dragging {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    .drop img {
        width: 100%;
        height: 100%;
        object-fit: contain;
    }

    .side {
        grid-area: side;
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 0.75rem 1.5rem;
    }

    .progress {
        font-weight: 700;
    }

    .note {
        font-size: 0.85em;
    }
</style>
