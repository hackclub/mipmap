<script lang="ts">
    import Field from "$lib/Field.svelte";
    import MipmapButton from "$lib/MipmapButton.svelte";
    import { send_snapshot } from "$lib/snapshots/api";
    import type { Snapshot, SnapshotStatus } from "$lib/snapshots/types";

    let {
        status,
        onsend,
    }: {
        status?: SnapshotStatus;
        onsend: (snapshot: Snapshot) => Promise<void>;
    } = $props();

    let picker = $state<HTMLInputElement>();
    let image = $state<File | null>(null);
    let preview = $state("");
    let caption = $state("");
    let dragging = $state(false);
    let pending = $state(false);
    let error = $state("");

    $effect(() => {
        const url = preview;
        return () => {
            if (url) URL.revokeObjectURL(url);
        };
    });

    const ready = $derived(
        status !== undefined &&
            !status.sent_today &&
            status.tracked_seconds >= status.required_seconds,
    );
    const progress = $derived(
        status &&
            `${Math.floor(status.tracked_seconds / 60)} / ${Math.floor(status.required_seconds / 60)} minutes`,
    );

    function pick(files: FileList | null | undefined) {
        const file = files?.[0];
        if (!file) return;
        if (!file.type.startsWith("image/")) {
            error = "Please choose an image file.";
            return;
        }
        error = "";
        image = file;
        preview = URL.createObjectURL(file);
    }

    async function submit(e: SubmitEvent) {
        e.preventDefault();
        if (!ready || !image || !caption.trim() || pending) return;
        pending = true;
        error = "";
        try {
            const snapshot = await send_snapshot(image, caption.trim());
            image = null;
            preview = "";
            caption = "";
            await onsend(snapshot);
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            pending = false;
        }
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
        accept="image/*"
        hidden
        onchange={(e) => {
            pick(e.currentTarget.files);
            e.currentTarget.value = "";
        }}
    />
    <div class="caption">
        <Field label="Caption" bind:value={caption} multiline />
    </div>
    <div class="side">
        <MipmapButton
            type="submit"
            disabled={!ready || !image || !caption.trim() || pending}
        >
            Send
        </MipmapButton>
        {#if status}
            <div class="progress">{progress}</div>
            <div class="note">
                {status.sent_today
                    ? "You've gotta wait for tomorrow to send the next one!"
                    : "You can still send a snapshot today!"}
            </div>
        {/if}
        {#if error}
            <div class="note">{error}</div>
        {/if}
    </div>
</form>

<style>
    form {
        display: grid;
        grid-template-columns: minmax(0, 3fr) minmax(0, 4fr);
        grid-template-rows: minmax(0, 1fr) auto;
        grid-template-areas:
            "drop caption"
            "drop side";
        gap: 1.5rem;
        height: 100%;
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
