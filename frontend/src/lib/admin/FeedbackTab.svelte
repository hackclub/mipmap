<script lang="ts">
    import { onMount } from "svelte";
    import { short_time } from "$lib/format";
    import { Load, Task } from "$lib/load.svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import {
        display_name,
        fetch_feedback,
        fetch_push,
        force_survey,
        start_push,
        stop_push,
    } from "./api";
    import { Pages } from "./pages.svelte";
    import type { Feedback } from "./types";
    import ActionDialog from "./ActionDialog.svelte";
    import Card from "./Card.svelte";
    import PagedList from "./PagedList.svelte";

    const pages = new Pages<Feedback>((before) => fetch_feedback({ before }));
    pages.reset();

    const push = new Load(fetch_push);
    push.run();
    const pushing = new Task();
    const forcing = new Task();
    let confirming = $state(false);
    let forced = $state<number | null>(null);

    onMount(() => {
        const timer = setInterval(() => {
            if (push.data?.running) push.run();
        }, 2000);
        return () => clearInterval(timer);
    });

    const responses = (count: number) =>
        `${count} response${count === 1 ? "" : "s"}`;
</script>

<div class="tools">
    <div class="tool">
        <button type="button" onclick={() => (confirming = true)}
            >Show survey to everyone</button
        >
        <span class="note" role="status">
            {#if forcing.error}{forcing.error}{:else if forced !== null}Steve
                will greet {forced} user{forced === 1 ? "" : "s"} on their
                next visit{/if}
        </span>
    </div>
    <div class="tool">
        {#if push.error}
            <LoadError error={push.error} onretry={push.run} />
        {:else if push.data}
            {@const status = push.data}
            {#if status.running}
                <button
                    type="button"
                    aria-disabled={status.stopping || pushing.pending}
                    onclick={() => {
                        if (!status.stopping)
                            pushing.run(async () => push.set(await stop_push()));
                    }}>{status.stopping ? "Stopping…" : "Stop"}</button
                >
            {:else}
                <button
                    type="button"
                    aria-disabled={!status.configured ||
                        !status.pending ||
                        pushing.pending}
                    onclick={() => {
                        if (status.configured && status.pending)
                            pushing.run(async () => push.set(await start_push()));
                    }}>Push to Unified</button
                >
            {/if}
            <span class="note" role="status">
                {#if pushing.error}
                    {pushing.error}
                {:else if status.running}
                    Sent {status.sent} of {responses(status.total)}{status.retries
                        ? `, ${status.retries} retr${status.retries === 1 ? "y" : "ies"}`
                        : ""}{status.error ? ` · ${status.error}` : ""}
                {:else if !status.configured}
                    Unified isn't configured
                {:else if status.error}
                    Stopped after {status.sent}: {status.error}
                {:else}
                    {status.pending
                        ? `${responses(status.pending)} not yet in Unified`
                        : "Everything is in Unified"}
                {/if}
            </span>
        {/if}
    </div>
</div>

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

{#if confirming}
    <ActionDialog
        title="Show Steve's survey to everyone?"
        submit="Show survey"
        warning="Every onboarded user who hasn't answered in the last 3 days will be asked on their next dashboard visit, skipping the usual gates"
        onsubmit={() =>
            forcing.run(async () => {
                forced = (await force_survey()).users;
            })}
        onclose={() => (confirming = false)}
    />
{/if}

<style>
    .tools {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        margin-bottom: 1rem;
    }

    .tool {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 0.5rem 1rem;
        font-size: 0.85rem;
    }

    .tool button {
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

    .tool button[aria-disabled="true"] {
        opacity: 0.5;
        cursor: default;
    }

    .tool button:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .tool button:not([aria-disabled="true"]):hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    .note,
    .label {
        color: #ead7c2;
    }

    .label {
        font-size: 0.75rem;
    }

    p {
        margin: 0;
        white-space: pre-wrap;
    }
</style>
