<script lang="ts">
    import { short_time } from "$lib/format";
    import { fetch_audit } from "./api";
    import { Pages } from "./pages.svelte";
    import type { Action } from "./types";
    import Card from "./Card.svelte";
    import PagedList from "./PagedList.svelte";

    const pages = new Pages<Action>((before) => fetch_audit({ before }));
    pages.reset();
</script>

<PagedList {pages} empty="No admin actions yet">
    {#snippet item(action)}
        <Card>
            {#snippet title()}
                {action.summary}
            {/snippet}
            {#snippet meta()}
                {action.admin_name || "Unknown admin"} · {short_time(
                    action.created_at,
                )}
            {/snippet}
            {#if action.reason}
                <p>{action.reason}</p>
            {/if}
        </Card>
    {/snippet}
</PagedList>

<style>
    p {
        margin: 0;
        white-space: pre-wrap;
        font-size: 0.9rem;
    }
</style>
