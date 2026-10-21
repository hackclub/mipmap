<script lang="ts">
    import { SvelteSet } from "svelte/reactivity";
    import { message } from "$lib/api";
    import { short_time } from "$lib/format";
    import HoldButton from "$lib/tree/HoldButton.svelte";
    import {
        display_name,
        fetch_held_snapshots,
        fetch_snapshot_rejections,
        review_snapshot,
        safe_url,
    } from "./api";
    import { Pages } from "./pages.svelte";
    import type { HeldSnapshot, SnapshotRejection } from "./types";
    import Card from "./Card.svelte";
    import PagedList from "./PagedList.svelte";

    const held = new Pages<HeldSnapshot>((before) =>
        fetch_held_snapshots({ before }),
    );
    const rejections = new Pages<SnapshotRejection>((before) =>
        fetch_snapshot_rejections({ before }),
    );
    held.reset();
    rejections.reset();

    let busy = $state<number | null>(null);
    let error = $state<{ id: number; text: string } | null>(null);
    const shown = new SvelteSet<number>();

    async function review(id: number, approve: boolean) {
        busy = id;
        error = null;
        try {
            await review_snapshot(id, approve);
            held.remove(id);
        } catch (e) {
            error = { id, text: message(e) };
        } finally {
            busy = null;
        }
    }
</script>

<section>
    <h3>Held for review</h3>
    <p class="note">
        Only their authors can see these until you approve them. Rejecting
        deletes the Snapshot
    </p>
    <PagedList pages={held} empty="Nothing held">
        {#snippet item(snapshot)}
            <Card level={4}>
                {#snippet title()}
                    {snapshot.author}
                {/snippet}
                {#snippet meta()}
                    {display_name(snapshot.user_name, snapshot.user_id)} · {short_time(
                        snapshot.created_at,
                    )}
                {/snippet}
                <ul class="reasons">
                    {#each snapshot.reasons as reason (reason)}
                        <li>{reason}</li>
                    {/each}
                </ul>
                <div class="body">
                    {#if safe_url(snapshot.image)}
                        <div class="shot">
                            <img
                                src={safe_url(snapshot.image)}
                                alt="Held Snapshot image from {snapshot.author}"
                                loading="lazy"
                                class:blurred={!shown.has(snapshot.id)}
                            />
                            <button
                                type="button"
                                class="reveal"
                                onclick={() =>
                                    shown.has(snapshot.id)
                                        ? shown.delete(snapshot.id)
                                        : shown.add(snapshot.id)}
                                >{shown.has(snapshot.id)
                                    ? "Hide image"
                                    : "Show image"}</button
                            >
                        </div>
                    {/if}
                    <p class="caption">{snapshot.caption}</p>
                </div>
                {#if error?.id === snapshot.id}
                    <p class="error" role="alert">{error.text}</p>
                {/if}
                {#snippet actions()}
                    <HoldButton
                        label="Approve"
                        duration={500}
                        disabled={busy !== null}
                        onconfirm={() => review(snapshot.id, true)}
                    />
                    <HoldButton
                        label="Reject"
                        duration={500}
                        disabled={busy !== null}
                        onconfirm={() => review(snapshot.id, false)}
                    />
                {/snippet}
            </Card>
        {/snippet}
    </PagedList>
</section>

<section>
    <h3>Rejected as sexual/minors</h3>
    <p class="note">These were refused outright. Images aren't kept</p>
    <PagedList pages={rejections} empty="Nothing rejected">
        {#snippet item(rejection)}
            <Card dim level={4}>
                {#snippet title()}
                    {rejection.author}
                {/snippet}
                {#snippet meta()}
                    {display_name(rejection.user_name, rejection.user_id)} · {short_time(
                        rejection.created_at,
                    )}
                {/snippet}
                <ul class="reasons">
                    {#each rejection.categories as category (category)}
                        <li>{category}</li>
                    {/each}
                </ul>
                <p class="caption">{rejection.caption}</p>
            </Card>
        {/snippet}
    </PagedList>
</section>

<style>
    section {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    section + section {
        margin-top: 2rem;
    }

    h3 {
        margin: 0;
        font-size: 1rem;
    }

    .note {
        margin: 0;
        color: #ead7c2;
    }

    .error {
        margin: 0;
        color: #e8c58c;
    }

    .reasons {
        display: flex;
        flex-wrap: wrap;
        gap: 0.4rem;
        margin: 0;
        padding: 0;
        list-style: none;
    }

    .reasons li {
        padding: 0.15rem 0.5rem;
        background: #74472a;
        border-radius: 7px;
        font-size: 0.85rem;
    }

    .body {
        display: flex;
        flex-wrap: wrap;
        gap: 1rem;
    }

    .shot {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 0.5rem;
    }

    .reveal {
        padding: 0.4rem 0.8rem;
        background-color: #4f2f1f;
        border: none;
        color: #ead7c2;
        font: inherit;
        font-size: 0.85rem;
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .reveal:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .reveal:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    img.blurred {
        filter: blur(1.25rem);
        clip-path: inset(0);
    }

    img {
        width: 12rem;
        max-width: 100%;
        aspect-ratio: 16 / 10;
        object-fit: cover;
        border: 2px solid #4f2f1f;
    }

    .caption {
        margin: 0;
        white-space: pre-wrap;
    }

    .body .caption {
        flex: 1 1 12rem;
    }
</style>
