<script lang="ts">
    import { short_time } from "$lib/format";
    import { display_name, fetch_snapshots, safe_url, set_shadowban } from "./api";
    import { Pages } from "./pages.svelte";
    import type { Snapshot } from "./types";
    import ActionDialog from "./ActionDialog.svelte";
    import Card from "./Card.svelte";
    import PagedList from "./PagedList.svelte";

    const pages = new Pages<Snapshot>((before) => fetch_snapshots({ before }));
    pages.reset();

    let editing = $state<Snapshot | null>(null);
    let reason = $state("");
</script>

<PagedList {pages} empty="No Snapshots yet">
    {#snippet item(snapshot)}
        <Card dim={snapshot.shadowbanned}>
            {#snippet title()}
                {snapshot.author}{snapshot.shadowbanned ? " · Shadowbanned" : ""}
            {/snippet}
            {#snippet meta()}
                {display_name(snapshot.user_name, snapshot.user_id)} · {short_time(
                    snapshot.created_at,
                )}
            {/snippet}
            <div class="body">
                {#if safe_url(snapshot.image)}
                    <img
                        src={safe_url(snapshot.image)}
                        alt="Snapshot image from {snapshot.author}"
                        loading="lazy"
                    />
                {/if}
                <p>{snapshot.caption}</p>
            </div>
            {#snippet actions()}
                <button
                    type="button"
                    onclick={() => {
                        editing = snapshot;
                        reason = "";
                    }}
                    >{snapshot.shadowbanned ? "Unshadowban" : "Shadowban"}</button
                >
            {/snippet}
        </Card>
    {/snippet}
</PagedList>

{#if editing}
    {@const snapshot = editing}
    <ActionDialog
        title={snapshot.shadowbanned
            ? "Unshadowban this Snapshot?"
            : "Shadowban this Snapshot?"}
        submit={snapshot.shadowbanned ? "Unshadowban" : "Shadowban"}
        warning={snapshot.shadowbanned
            ? "It will show up in everyone's feed again"
            : "Only its author will still see it. It keeps counting toward their multiplier"}
        bind:reason
        onsubmit={async () =>
            pages.replace(
                await set_shadowban(snapshot.id, {
                    shadowbanned: !snapshot.shadowbanned,
                    reason,
                }),
            )}
        onclose={() => (editing = null)}
    />
{/if}

<style>
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

    p {
        flex: 1 1 12rem;
        margin: 0;
        white-space: pre-wrap;
    }
</style>
