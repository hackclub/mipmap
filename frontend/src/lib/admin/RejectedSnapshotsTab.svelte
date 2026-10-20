<script lang="ts">
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
    let error = $state("");

    async function review(id: number, approve: boolean) {
        busy = id;
        error = "";
        try {
            await review_snapshot(id, approve);
            held.remove(id);
        } catch (e) {
            error = message(e);
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
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <PagedList pages={held} empty="Nothing held">
        {#snippet item(snapshot)}
            <Card>
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
                        <img src={safe_url(snapshot.image)} alt="" loading="lazy" />
                    {/if}
                    <p class="caption">{snapshot.caption}</p>
                </div>
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
            <Card dim>
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
        color: #d6ab70;
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
