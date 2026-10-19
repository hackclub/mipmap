<script lang="ts">
    import { short_time } from "$lib/format";
    import { Load, Task } from "$lib/load.svelte";
    import Field from "$lib/ui/Field.svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import { fetch_thread, fetch_threads, reply } from "./api";
    import type { SlackMessage, Thread } from "./types";
    import SlackText from "./SlackText.svelte";
    import { decode, escape } from "./slack.svelte";

    const limit = 3000;

    const threads = new Load(fetch_threads);
    threads.run();

    let open = $state<Thread | null>(null);
    let conversation = $state.raw<Load<SlackMessage[]>>();
    let text = $state("");
    const sending = new Task();

    const length = $derived([...escape(text.trim())].length);
    const name = (thread: Thread) => thread.user_name || thread.slack_id;

    const select = (thread: Thread) => {
        open = thread;
        text = "";
        sending.error = "";
        const load = new Load(() => fetch_thread(thread.slack_id));
        conversation = load;
        load.run().then(threads.run);
    };

    const send = (e: SubmitEvent) => {
        e.preventDefault();
        const thread = open;
        const load = conversation;
        if (!thread || !load || !length || length > limit) return;
        sending.run(async () => {
            load.set(await reply(thread.slack_id, text));
            text = "";
            threads.run();
        });
    };
</script>

<div class="layout">
    <section class="threads" aria-label="Conversations">
        {#if threads.error}
            <LoadError error={threads.error} onretry={threads.run} />
        {:else if !threads.data}
            <p class="note">Loading…</p>
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
                                >{thread.unread}</span
                            >{/if}
                    </span>
                    <span class="last">{decode(thread.last_text)}</span>
                    <span class="when">{short_time(thread.last_at)}</span>
                </button>
            {:else}
                <p class="note">Nobody has replied to the bot yet</p>
            {/each}
        {/if}
    </section>

    <section class="conversation" aria-label="Messages">
        {#if !open}
            <p class="note">Choose a conversation</p>
        {:else}
            <h3>{name(open)}{open.user_name ? ` · ${open.slack_id}` : ""}</h3>
            {#if conversation?.error}
                <p class="error" role="alert">{conversation.error}</p>
            {:else if !conversation?.data}
                <p class="note">Loading…</p>
            {/if}
            <ol>
                {#each conversation?.data ?? [] as message (message.id)}
                    <li class:incoming={message.incoming}>
                        <SlackText text={message.text} />
                        <span class="when"
                            >{short_time(message.at)}{message.admin_name
                                ? ` · ${message.admin_name}`
                                : message.incoming
                                  ? ""
                                  : " · bot"}</span
                        >
                    </li>
                {/each}
            </ol>
            <form onsubmit={send}>
                <Field
                    label="Reply as the bot"
                    multiline
                    rows={3}
                    bind:value={text}
                />
                <div class="send-row">
                    <span class="note" class:over={length > limit}
                        >{length} / {limit}</span
                    >
                    <button
                        type="submit"
                        disabled={sending.pending || !length || length > limit}
                        >Send reply</button
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
        color: #d6ab70;
        font-size: 0.8rem;
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

    ol {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        max-height: 32rem;
        margin: 0;
        padding: 0;
        overflow: auto;
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

    button[type="submit"]:disabled {
        opacity: 0.5;
        cursor: default;
    }

    button[type="submit"]:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        button[type="submit"]:not(:disabled):hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    @media (max-width: 48rem) {
        .layout {
            grid-template-columns: minmax(0, 1fr);
        }
    }
</style>
