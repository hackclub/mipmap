<script lang="ts">
    import { short_time } from "$lib/format";
    import { display_name, fetch_feedback } from "./api";
    import { Pages } from "./pages.svelte";
    import type { Feedback } from "./types";
    import Card from "./Card.svelte";
    import PagedList from "./PagedList.svelte";

    const pages = new Pages<Feedback>((before) => fetch_feedback({ before }));
    pages.reset();
</script>

<PagedList {pages} empty="No feedback yet">
    {#snippet item(entry)}
        <Card>
            {#snippet title()}
                {entry.score}/10
            {/snippet}
            {#snippet meta()}
                {display_name(entry.user_name, entry.user_id)} · {short_time(
                    entry.created_at,
                )}
            {/snippet}
            {#each [["How did you find out about Mipmap?", entry.source], ["What are we doing well?", entry.positives], ["How can we improve?", entry.improvements]] as [label, text] (label)}
                {#if text}
                    <div>
                        <div class="label">{label}</div>
                        <p>{text}</p>
                    </div>
                {/if}
            {/each}
        </Card>
    {/snippet}
</PagedList>

<style>
    .label {
        color: #ead7c2;
        font-size: 0.75rem;
    }

    p {
        margin: 0;
        white-space: pre-wrap;
    }
</style>
