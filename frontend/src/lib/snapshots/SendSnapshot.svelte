<script lang="ts">
    import { image_error, image_types } from "$lib/api";
    import { dashboard } from "$lib/dashboard/state.svelte";
    import Field from "$lib/ui/Field.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import {
        save_sent_at,
        save_username,
        saved_sent_at,
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
    let sent = $state(false);
    let sent_at = $state(saved_sent_at());
    const send = new Task();
    const cooldown = 16 * 60 * 60 * 1000;
    const recheck = 15 * 60 * 1000;

    const unlocks = $derived(
        status.sent_today && sent_at + cooldown > Date.now()
            ? sent_at + cooldown
            : 0,
    );
    const wait_note = $derived.by(() => {
        if (!unlocks)
            return "You can send your next one 16 hours after your last!";
        const at = new Date(unlocks);
        const time = at.toLocaleTimeString([], {
            hour: "numeric",
            minute: "2-digit",
        });
        return at.toDateString() === new Date().toDateString()
            ? `You can send your next one at ${time}!`
            : `You can send your next one tomorrow at ${time}!`;
    });

    $effect(() => {
        if (!status.sent_today) return;
        const wait = unlocks ? unlocks - Date.now() + 5000 : recheck;
        const timer = setTimeout(dashboard.run, Math.min(wait, recheck));
        return () => clearTimeout(timer);
    });

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
    const missing = $derived(
        [
            !image && "an image",
            !caption.trim() && "a caption",
            !username.trim() && "a username",
        ].filter((part): part is string => !!part),
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
        const text = caption;
        const name = username.trim();
        sent = false;
        await send.run(
            () => send_snapshot(file, text.trim(), name),
            () => {
                save_username(name);
                sent_at = Date.now();
                save_sent_at(sent_at);
                sent = true;
                if (image === file) {
                    image = null;
                    preview = "";
                }
                if (caption === text) caption = "";
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
    <p class="public">
        This will be shown publicly across Mipmap, so make sure there's no
        personal info!
    </p>
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
            {send.pending ? "Sending…" : "Send"}
        </MipmapButton>
        <div class="progress">{progress}</div>
        <div class="note">
            {status.sent_today
                ? wait_note
                : status.tracked_seconds < status.required_seconds
                  ? "Make sure you've got 15 minutes in projects tracked by Hackatime and linked to a Mipmap project first!"
                  : "You can still send a Snapshot today!"}
        </div>
        {#if ready && missing.length}
            <div class="note">
                Make sure you've got {missing.length > 1
                    ? `${missing.slice(0, -1).join(", ")} and ${missing.at(-1)}`
                    : missing[0]} before you can submit
            </div>
        {/if}
        {#if send.error}
            <div class="note" role="alert">{send.error}</div>
        {/if}
        <p class="sr" role="status">
            {send.pending
                ? "Sending your Snapshot"
                : sent
                  ? "Snapshot sent"
                  : ""}
        </p>
    </div>
</form>

<style>
    .public {
        grid-area: public;
        margin: 0;
        font-size: 0.85em;
    }

    form {
        display: grid;
        grid-template-columns: minmax(0, 3fr) minmax(0, 4fr);
        grid-template-rows: auto auto minmax(7em, 1fr) auto;
        grid-template-areas:
            "drop public"
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
        margin: 2.5rem 1.5rem;
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

    .drop:focus-visible,
    .drop.dragging {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .drop:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
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

    .sr {
        position: absolute;
        width: 1px;
        height: 1px;
        margin: 0;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }
</style>
