<script lang="ts">
    import { tick } from "svelte";
    import { short_time } from "$lib/format";
    import { Load, Task } from "$lib/load.svelte";
    import Field from "$lib/ui/Field.svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import {
        fetch_thread,
        fetch_threads,
        mark_read,
        reply,
        send_key,
        threads_cap,
    } from "./api";
    import type { SlackMessage, Thread } from "./types";
    import SlackText from "./SlackText.svelte";
    import { decode, mrkdwn } from "./slack.svelte";

    const limit = 3000;

    const threads = new Load(fetch_threads);
    threads.run();

    let open = $state<Thread | null>(null);
    let conversation = $state.raw<Load<SlackMessage[]>>();
    let text = $state("");
    let scroller = $state<HTMLElement>();
    let section = $state<HTMLElement>();
    const sending = new Task();

    const length = $derived([...mrkdwn(text.trim())].length);
    const key = $derived(send_key(text, open?.slack_id));
    const name = (thread: Thread) => thread.user_name || thread.slack_id;

    async function read(slack_id: string) {
        try {
            await mark_read(slack_id);
        } catch {
            return;
        }
        if (threads.data)
            threads.set(
                threads.data.map((thread) =>
                    thread.slack_id === slack_id
                        ? { ...thread, unread: 0 }
                        : thread,
                ),
            );
    }

    async function select(thread: Thread) {
        if (open?.slack_id !== thread.slack_id) {
            text = "";
            sending.error = "";
        }
        open = thread;
        const load = new Load(() => fetch_thread(thread.slack_id));
        conversation = load;
        if (matchMedia("(max-width: 48rem)").matches) {
            await tick();
            section?.scrollIntoView({ block: "start" });
        }
        await load.run();
        if (!load.error && conversation === load) read(thread.slack_id);
    }

    const send = (e: SubmitEvent) => {
        e.preventDefault();
        const thread = open;
        const load = conversation;
        if (sending.pending || !thread || !load || !length || length > limit)
            return;
        const message = text;
        sending.run(async () => {
            load.set(await reply(thread.slack_id, message, key));
            if (text === message) text = "";
            threads.run();
        });
    };

    $effect(() => {
        if (!conversation?.data || !scroller) return;
        scroller.scrollTop = scroller.scrollHeight;
    });
</script>

<div class="layout">
    <section class="threads" aria-label="Conversations">
        {#if threads.error}
            <LoadError error={threads.error} onretry={threads.run} />
        {:else if !threads.data}
            <p class="note" role="status">Loading…</p>
        {:else}
            {#each threads.data as thread (thread.slack_id)}
                {@const active = open?.slack_id === thread.slack_id}
                <button
                    type="button"
                    class="thread"
                    class:active
                    aria-current={active || undefined}
                    class:unread={thread.unread > 0}
                    disabled={sending.pending}
                    onclick={() => select(thread)}
                >
                    <span class="who">
                        {name(thread)}
                        {#if thread.unread}<span class="badge"
                                >{thread.unread}<span class="sr"> unread</span
                                ></span
                            >{/if}
                    </span>
                    <span class="last">{decode(thread.last_text)}</span>
                    <span class="when">{short_time(thread.last_at)}</span>
                </button>
            {:else}
                <p class="note">Nobody has replied to the bot yet</p>
            {/each}
            {#if threads.data.length >= threads_cap}
                <p class="note">
                    Showing {threads_cap} conversations, unread first, then the most
                    recent. Older ones aren't listed
                </p>
            {/if}
        {/if}
    </section>

    <section class="conversation" aria-label="Messages" bind:this={section}>
        {#if !open}
            <p class="note">Choose a conversation</p>
        {:else}
            <h3>{name(open)}{open.user_name ? ` · ${open.slack_id}` : ""}</h3>
            {#if conversation?.error}
                <p class="error" role="alert">{conversation.error}</p>
            {/if}
            <p class="note" role="status">
                {!conversation?.error && !conversation?.data ? "Loading…" : ""}
            </p>
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <div
                class="scroller"
                tabindex="0"
                role="region"
                aria-label="Messages with {name(open)}"
                bind:this={scroller}
            >
                <ol>
                    {#each conversation?.data ?? [] as message (message.id)}
                        <li class:incoming={message.incoming}>
                            <SlackText text={message.text} />
                            <span class="when"
                                >{short_time(message.at)} · {message.incoming
                                    ? name(open)
                                    : (message.admin_name ?? "bot")}</span
                            >
                        </li>
                    {/each}
                </ol>
            </div>
            <form onsubmit={send}>
                <Field
                    label="Reply as the bot"
                    multiline
                    rows={3}
                    readonly={sending.pending}
                    bind:value={text}
                />
                <div class="send-row">
                    <span class="note" class:over={length > limit}
                        >{length} / {limit}</span
                    >
                    <button
                        type="submit"
                        aria-disabled={sending.pending ||
                            !length ||
                            length > limit}>Send reply</button
                    >
                </div>
                {#if sending.error}
                    <p class="error" role="alert">{sending.error}</p>
                {/if}
            </form>
        {/if}
    </section>
</div>

<style>
    .layout {
        display: grid;
        grid-template-columns: minmax(0, 18rem) minmax(0, 1fr);
        align-items: start;
        gap: 1rem;
    }

    .threads,
    .conversation,
    form {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        min-width: 0;
    }

    h3,
    p {
        margin: 0;
    }

    h3 {
        font-size: 1rem;
        overflow-wrap: anywhere;
    }

    .note,
    .when {
        color: #ead7c2;
        font-size: 0.8rem;
    }

    .sr {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }

    .over,
    .error {
        color: #ead7c2;
        font-weight: 700;
    }

    .thread {
        display: flex;
        flex-direction: column;
        gap: 0.2rem;
        padding: 0.6rem 0.75rem;
        background: #5f4838;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font: inherit;
        text-align: left;
        cursor: pointer;
    }

    .thread:disabled {
        cursor: default;
    }

    .thread.active,
    .thread:focus-visible {
        border-color: #ead7c2;
    }

    @media (hover: hover) {
        .thread:not(:disabled):hover {
            border-color: #d6ab70;
        }
    }

    .who {
        display: flex;
        justify-content: space-between;
        gap: 0.5rem;
        font-size: 0.9rem;
        overflow-wrap: anywhere;
    }

    .unread .who {
        font-weight: 700;
    }

    .badge {
        padding: 0 0.4rem;
        background: #d6ab70;
        border-radius: 999px;
        color: #4f2f1f;
        font-size: 0.75rem;
    }

    .last {
        overflow: hidden;
        font-size: 0.8rem;
        white-space: nowrap;
        text-overflow: ellipsis;
    }

    .scroller {
        max-height: 32rem;
        overflow: auto;
    }

    .scroller:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: 2px;
    }

    ol {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        margin: 0;
        padding: 0;
        list-style: none;
    }

    li {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        max-width: 85%;
        align-self: flex-end;
        padding: 0.6rem 0.75rem;
        background: #5f4838;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    li.incoming {
        align-self: flex-start;
        background: #3d3029;
    }

    .send-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    button[type="submit"] {
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

    button[type="submit"][aria-disabled="true"] {
        opacity: 0.5;
        cursor: default;
    }

    button[type="submit"]:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        button[type="submit"]:not([aria-disabled="true"]):hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    @media (max-width: 48rem) {
        .layout {
            grid-template-columns: minmax(0, 1fr);
        }

        .threads {
            max-height: 40vh;
            overflow: auto;
        }
    }
</style>
