<script lang="ts">
    import { tick } from "svelte";
    import { short_time } from "$lib/format";
    import { Load, Task } from "$lib/load.svelte";
    import Field from "$lib/ui/Field.svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import HoldButton from "$lib/tree/HoldButton.svelte";
    import {
        dismiss_draft,
        fetch_drafts,
        fetch_slack_admins,
        fetch_slack_log,
        fetch_slack_templates,
        refresh_draft,
        send_announcement,
        send_direct,
        send_draft,
        send_key,
    } from "./api";
    import { Pages } from "./pages.svelte";
    import type { Draft, LogEntry } from "./types";
    import ActionDialog from "./ActionDialog.svelte";
    import Card from "./Card.svelte";
    import Filters from "./Filters.svelte";
    import PagedList from "./PagedList.svelte";
    import SlackText from "./SlackText.svelte";
    import { mrkdwn } from "./slack.svelte";

    const limit = 3000;
    const kinds: Record<string, string> = {
        announcement: "Announcement",
        challenge: "New challenge",
        round: "Fulfillment round",
        snapshot: "Snapshot reminder",
        ship: "Ship update",
        inactivity: "Inactivity reminder",
        welcome: "Welcome",
        direct: "Direct message",
        reply: "Reply",
        admin: "Admin alert",
        message: "Received",
    };

    let text = $state("");
    let slack_id = $state("");
    let notice = $state("");
    let sent = $state(0);
    let template = $state("");
    let replacing = $state<string | null>(null);
    let message_box = $state<HTMLElement>();
    let drafts_heading = $state<HTMLElement>();
    const key = $derived(send_key(text, slack_id, sent));
    const task = new Task();
    const drafts = new Load(fetch_drafts);
    drafts.run();
    const admins = new Load(fetch_slack_admins);
    admins.run();
    const templates = new Load(fetch_slack_templates);
    templates.run();
    const log = new Pages<LogEntry>((before) => fetch_slack_log({ before }));
    log.reset();

    const length = $derived([...mrkdwn(text.trim())].length);
    const ready = $derived(length > 0 && length <= limit);

    const people = (count: number) =>
        `${count} ${count === 1 ? "person" : "people"}`;

    const run = (action: () => Promise<string>) =>
        task.run(async () => {
            notice = "";
            notice = await action();
            sent++;
            log.refresh();
        });

    function to_all() {
        if (task.pending || !ready) return;
        const message = text;
        run(async () => {
            const result = await send_announcement(message, key);
            if (text === message) text = "";
            await tick();
            message_box?.querySelector("textarea")?.focus();
            return result.recipients
                ? `Announcement queued for ${people(result.recipients)}`
                : "Nobody gets announcements right now, so it wasn't sent to anyone";
        });
    }

    function to_one() {
        if (task.pending || !ready || !slack_id.trim()) return;
        const to = slack_id.trim();
        run(async () => {
            await send_direct(slack_id, text, key);
            return `Sent to ${to}`;
        });
    }

    function use_template() {
        const chosen = templates.data?.[Number(template)];
        if (!chosen || template === "" || task.pending) return;
        if (text.trim() && text !== chosen.text) replacing = chosen.text;
        else text = chosen.text;
    }

    const draft_action = (action: () => Promise<Draft[]>) =>
        task.run(async () => {
            notice = "";
            drafts.set(await action());
            log.refresh();
            await tick();
            const active = document.activeElement;
            if (!active || active === document.body || !active.isConnected)
                (
                    drafts_heading ??
                    message_box?.querySelector<HTMLElement>("textarea")
                )?.focus();
        });

    const refresh = (draft: Draft) =>
        draft_action(async () => {
            const updated = await refresh_draft(draft.id);
            const found = updated.find((other) => other.id === draft.id);
            notice = !found
                ? "That message isn't waiting any more"
                : found.text === draft.text
                  ? "That message already uses the current wording"
                  : "Updated to the current wording";
            return updated;
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
                <h3 tabindex="-1" bind:this={drafts_heading}>Waiting for you</h3>
                {#each drafts.data as draft (draft.id)}
                    <Card level={4}>
                        {#snippet title()}
                            {kinds[draft.kind] ?? draft.kind}
                        {/snippet}
                        {#snippet meta()}
                            {short_time(draft.created_at)}
                        {/snippet}
                        <SlackText text={draft.text} footer />
                        <p class="note">Skipping can't be undone</p>
                        {#snippet actions()}
                            <HoldButton
                                label="Send"
                                duration={2000}
                                disabled={task.pending}
                                onconfirm={() =>
                                    draft_action(() => send_draft(draft.id))}
                            />
                            <button
                                type="button"
                                aria-disabled={task.pending}
                                onclick={() => {
                                    if (!task.pending) refresh(draft);
                                }}>Refresh</button
                            >
                            <HoldButton
                                label="Skip"
                                disabled={task.pending}
                                onconfirm={() =>
                                    draft_action(() => dismiss_draft(draft.id))}
                            />
                        {/snippet}
                    </Card>
                {/each}
            </section>
        {/if}

        <div class="message" bind:this={message_box}>
            <Field
                label="Message"
                hint="Slack formatting: *bold*, _italic_, ~strike~, `code`, ```blocks```, > quotes, <https://link|text>, <@U0123ABCD>, #mipmap, :emoji:"
                multiline
                rows={8}
                readonly={task.pending}
                bind:value={text}
            />
        </div>
        <p class="count" class:over={length > limit}>{length} / {limit}</p>

        <section class="preview" aria-label="Preview">
            <h3>Preview</h3>
            {#if length}
                <SlackText {text} footer />
            {:else}
                <p class="note">Nothing to preview yet</p>
            {/if}
        </section>

        {#if templates.data?.length}
            <div class="premade">
                <label for="premade">Premade message</label>
                <div class="premade-row">
                    <select id="premade" bind:value={template}>
                        <option value="" disabled>Choose a template…</option>
                        {#each templates.data as option, i (i)}
                            <option value={String(i)}>{option.label}</option>
                        {/each}
                    </select>
                    <button
                        type="button"
                        class="send"
                        aria-disabled={template === "" || task.pending}
                        onclick={use_template}>Insert template</button
                    >
                </div>
            </div>
        {/if}
        <div class="direct">
            <Field label="Slack ID" placeholder="U0123ABCD" bind:value={slack_id} />
            <button
                type="button"
                class="send"
                aria-disabled={!ready || !slack_id.trim() || task.pending}
                onclick={to_one}>Send to Slack ID</button
            >
        </div>
        {#if admins.data?.length}
            <div class="admins">
                <span class="note">Admins</span>
                <Filters
                    label="Admins"
                    options={admins.data.map((admin) => [
                        admin.slack_id,
                        admin.name,
                    ])}
                    bind:value={slack_id}
                />
            </div>
        {/if}

        <div class="all">
            <HoldButton
                label="Send to All"
                duration={2000}
                disabled={!ready || task.pending}
                onconfirm={to_all}
            />
        </div>

        <p class="note" role="status">{task.error ? "" : notice}</p>
        {#if task.error}
            <p class="error" role="alert">{task.error}</p>
        {/if}
    </div>

    <section class="log" aria-label="Message log">
        <div class="log-head">
            <h3>Message log</h3>
            <button type="button" class="send" onclick={log.refresh}
                >Refresh</button
            >
        </div>
        <PagedList pages={log} empty="No messages yet">
            {#snippet item(entry)}
                <Card dim={entry.incoming} level={4}>
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
                    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                    <div
                        class="clip"
                        tabindex="0"
                        role="region"
                        aria-label="Message text"
                    >
                        <SlackText text={entry.text} />
                    </div>
                    {#if status(entry)}
                        <p class="status">{status(entry)}</p>
                    {/if}
                </Card>
            {/snippet}
        </PagedList>
    </section>
</div>

{#if replacing !== null}
    {@const chosen = replacing}
    <ActionDialog
        title="Replace your message?"
        submit="Replace"
        warning="The message you've written will be replaced with this template"
        onsubmit={async () => (text = chosen)}
        onclose={() => (replacing = null)}
    />
{/if}

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
        color: #ead7c2;
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
        color: #ead7c2;
        font-size: 0.85rem;
    }

    .premade {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    .premade label {
        font-size: 0.85em;
        font-weight: 700;
    }

    .premade-row {
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem;
    }

    .premade select {
        flex: 1 1 12rem;
        min-width: 0;
        padding: 0.5rem 0.75rem;
        background: #4f2f1f;
        color: #ead7c2;
        border: 1px solid #d6ab70;
        border-radius: 0;
        font: inherit;
        font-size: 0.85em;
        cursor: pointer;
    }

    .premade select:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: -2px;
    }

    .direct {
        display: flex;
        align-items: flex-end;
        gap: 0.5rem;
    }

    .direct :global(.field) {
        flex: 1;
    }

    .admins {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 0.5rem;
        margin-top: -0.25rem;
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

    .clip:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: 2px;
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

    .send[aria-disabled="true"] {
        opacity: 0.5;
        cursor: default;
    }

    .send:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .send:not([aria-disabled="true"]):hover {
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
