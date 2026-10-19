<script lang="ts">
    import { short_time } from "$lib/format";
    import { Load, Task } from "$lib/load.svelte";
    import Field from "$lib/ui/Field.svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import HoldButton from "$lib/tree/HoldButton.svelte";
    import {
        dismiss_draft,
        fetch_drafts,
        fetch_slack_log,
        send_announcement,
        send_direct,
        send_draft,
    } from "./api";
    import { Pages } from "./pages.svelte";
    import type { Draft, LogEntry } from "./types";
    import Card from "./Card.svelte";
    import PagedList from "./PagedList.svelte";
    import SlackText from "./SlackText.svelte";
    import { escape } from "./slack.svelte";

    const limit = 3000;
    const kinds: Record<string, string> = {
        announcement: "Announcement",
        challenge: "New challenge",
        round: "Fulfillment round",
        snapshot: "Snapshot reminder",
        ship: "Ship update",
        inactivity: "Inactivity reminder",
        direct: "Direct message",
        reply: "Reply",
        admin: "Admin alert",
        message: "Received",
    };

    let text = $state("");
    let slack_id = $state("");
    let notice = $state("");
    const task = new Task();
    const drafts = new Load(fetch_drafts);
    drafts.run();
    const log = new Pages<LogEntry>((before) => fetch_slack_log({ before }));
    log.reset();

    const length = $derived([...escape(text.trim())].length);
    const ready = $derived(length > 0 && length <= limit && !task.pending);

    const people = (count: number) =>
        `${count} ${count === 1 ? "person" : "people"}`;

    const run = (action: () => Promise<unknown>, done: string) =>
        task.run(async () => {
            notice = "";
            await action();
            notice = done;
            log.reset();
        });

    const to_all = () =>
        run(async () => {
            await send_announcement(text);
            text = "";
        }, "Announcement queued for everyone who gets announcements");

    const to_one = () =>
        run(() => send_direct(slack_id, text), `Sent to ${slack_id.trim()}`);

    const draft_action = (action: () => Promise<Draft[]>) =>
        task.run(async () => {
            notice = "";
            drafts.set(await action());
            log.reset();
        });

    function who(entry: LogEntry) {
        if (entry.recipients > 1) return people(entry.recipients);
        return entry.user_name || entry.slack_id || "";
    }

    function status(entry: LogEntry) {
        if (entry.incoming) return "";
        if (entry.recipients > 1)
            return [
                `${entry.sent} sent`,
                entry.pending && `${entry.pending} waiting`,
                entry.failed && `${entry.failed} failed`,
                entry.skipped && `${entry.skipped} skipped`,
            ]
                .filter(Boolean)
                .join(" · ");
        if (entry.sent) return "Sent";
        if (entry.failed) return `Failed${entry.error ? `: ${entry.error}` : ""}`;
        if (entry.skipped) return "Skipped, they turned these off";
        return "Waiting to send";
    }
</script>

<div class="layout">
    <div class="compose">
        {#if drafts.error}
            <LoadError error={drafts.error} onretry={drafts.run} />
        {:else if drafts.data?.length}
            <section class="drafts">
                <h3>Waiting for you</h3>
                {#each drafts.data as draft (draft.id)}
                    <Card>
                        {#snippet title()}
                            {kinds[draft.kind] ?? draft.kind}
                        {/snippet}
                        {#snippet meta()}
                            {short_time(draft.created_at)}
                        {/snippet}
                        <SlackText text={draft.text} footer />
                        {#snippet actions()}
                            <button
                                type="button"
                                disabled={task.pending}
                                onclick={() =>
                                    draft_action(() => send_draft(draft.id))}
                                >Send</button
                            >
                            <button
                                type="button"
                                disabled={task.pending}
                                onclick={() =>
                                    draft_action(() => dismiss_draft(draft.id))}
                                >Not now</button
                            >
                        {/snippet}
                    </Card>
                {/each}
            </section>
        {/if}

        <Field
            label="Message"
            hint="Slack formatting: *bold*, _italic_, ~strike~, `code`, ```blocks```, > quotes, <https://link|text>, <@U0123ABCD>, :emoji:"
            multiline
            rows={8}
            bind:value={text}
        />
        <p class="count" class:over={length > limit}>{length} / {limit}</p>

        <section class="preview" aria-label="Preview">
            <h3>Preview</h3>
            {#if length}
                <SlackText {text} footer />
            {:else}
                <p class="note">Nothing to preview yet</p>
            {/if}
        </section>

        <div class="direct">
            <Field label="Slack ID" placeholder="U0123ABCD" bind:value={slack_id} />
            <button
                type="button"
                class="send"
                disabled={!ready || !slack_id.trim()}
                onclick={to_one}>Send to Slack ID</button
            >
        </div>

        <div class="all">
            <HoldButton
                label="Send to All"
                duration={2000}
                disabled={!ready}
                onconfirm={to_all}
            />
        </div>

        {#if task.error}
            <p class="error" role="alert">{task.error}</p>
        {:else if notice}
            <p class="note" role="status">{notice}</p>
        {/if}
    </div>

    <section class="log" aria-label="Message log">
        <div class="log-head">
            <h3>Message log</h3>
            <button type="button" class="send" onclick={log.reset}
                >Refresh</button
            >
        </div>
        <PagedList pages={log} empty="No messages yet">
            {#snippet item(entry)}
                <Card dim={entry.incoming}>
                    {#snippet title()}
                        {entry.incoming
                            ? `From ${who(entry)}`
                            : `${kinds[entry.kind] ?? entry.kind} · ${who(entry)}`}
                    {/snippet}
                    {#snippet meta()}
                        {short_time(entry.at)}{entry.admin_name
                            ? ` · by ${entry.admin_name}`
                            : ""}
                    {/snippet}
                    <div class="clip"><SlackText text={entry.text} /></div>
                    {#if status(entry)}
                        <p class="status">{status(entry)}</p>
                    {/if}
                </Card>
            {/snippet}
        </PagedList>
    </section>
</div>

<style>
    .layout {
        display: grid;
        grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
        align-items: start;
        gap: 1.5rem;
    }

    .compose,
    .drafts,
    .log {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        min-width: 0;
    }

    h3 {
        margin: 0;
        font-size: 1rem;
    }

    p {
        margin: 0;
    }

    .count {
        align-self: flex-end;
        margin-top: -0.5rem;
        color: #d6ab70;
        font-size: 0.8rem;
    }

    .count.over,
    .error {
        color: #ead7c2;
        font-weight: 700;
    }

    .preview {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        padding: 1rem;
        background: #5f4838;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    .note,
    .status {
        color: #d6ab70;
        font-size: 0.85rem;
    }

    .direct {
        display: flex;
        align-items: flex-end;
        gap: 0.5rem;
    }

    .direct :global(.field) {
        flex: 1;
    }

    .all {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
    }

    .log-head {
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    .clip {
        max-height: 8rem;
        overflow: auto;
    }

    .send {
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

    .send:disabled {
        opacity: 0.5;
        cursor: default;
    }

    .send:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .send:not(:disabled):hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    @media (max-width: 72rem) {
        .layout {
            grid-template-columns: minmax(0, 1fr);
        }
    }
</style>
