<script lang="ts">
    import { tick, untrack } from "svelte";
    import { message } from "$lib/api";
    import Field from "$lib/ui/Field.svelte";
    import { short_time } from "$lib/format";
    import {
        adjust_mips,
        display_name,
        fetch_users,
        set_banned,
        set_mips,
    } from "./api";
    import { Pages } from "./pages.svelte";
    import type { User } from "./types";
    import ActionDialog from "./ActionDialog.svelte";
    import Card from "./Card.svelte";
    import PagedList from "./PagedList.svelte";

    let {
        onchange,
        lookup = null,
    }: { onchange: () => void; lookup?: { hca_id: string } | null } = $props();

    let search = $state("");
    let query = "";
    let list = $state<PagedList<User>>();
    const pages = new Pages<User>((before) => fetch_users({ before, q: query }));
    pages.reset();

    function find(next: string) {
        if (next === query) return;
        query = next;
        return pages.reset();
    }

    $effect(() => {
        const next = search;
        const timer = setTimeout(() => find(next), 300);
        return () => clearTimeout(timer);
    });

    $effect(() => {
        const target = lookup;
        if (target) untrack(() => show(target.hca_id));
    });

    async function show(hca_id: string) {
        search = hca_id;
        await find(hca_id);
        await tick();
        const user = pages.items.find((u) => u.hca_id === hca_id);
        if (user) list?.focus_item(user.id);
    }

    type Kind = "mips" | "adjust" | "ban";

    let editing = $state<{ user: User; kind: Kind } | null>(null);
    let mips = $state("");
    let reason = $state("");
    let current = $state<number | null>(null);
    let check_error = $state("");

    const parsed = $derived(Number(mips));
    const checking = $derived(
        current === null && !check_error ? "Checking their current Mips…" : "",
    );
    const balance = $derived(current ?? editing?.user.mips ?? 0);
    const total = $derived(
        editing?.kind === "adjust" ? balance + parsed : parsed,
    );
    const valid = $derived(
        !checking &&
            `${mips ?? ""}`.trim() !== "" &&
            Number.isInteger(parsed) &&
            (editing?.kind !== "adjust" || parsed !== 0) &&
            total >= 0 &&
            total <= 10_000_000,
    );

    function open(user: User, kind: Kind) {
        editing = { user, kind };
        mips = kind === "mips" ? String(user.mips) : "";
        reason = "";
        current = null;
        check_error = "";
        if (kind !== "ban") recheck(user, kind);
    }

    async function recheck(user: User, kind: Kind) {
        try {
            const fresh = (
                await fetch_users({ before: user.id + 1, q: user.hca_id })
            ).find((u) => u.id === user.id);
            if (editing?.user.id !== user.id || editing.kind !== kind) return;
            if (!fresh) throw new Error("They weren't found");
            if (kind === "mips" && mips === String(user.mips))
                mips = String(fresh.mips);
            current = fresh.mips;
            pages.replace(fresh);
        } catch (e) {
            if (editing?.user.id === user.id)
                check_error = `Couldn't check their current Mips (${message(e)}), so this may be out of date`;
        }
    }
</script>

<div class="search">
    <Field
        type="search"
        label="Search users"
        placeholder="Name, email, Slack ID, HCA id or user id"
        bind:value={search}
    />
</div>

<PagedList {pages} empty="No users found" bind:this={list}>
    {#snippet item(user)}
        <Card dim={user.banned}>
            {#snippet title()}
                {display_name(user.name, user.id)}{user.admin
                    ? " · Admin"
                    : ""}{user.banned ? " · Banned" : ""}
            {/snippet}
            {#snippet meta()}
                #{user.id}
            {/snippet}
            <dl>
                <div><dt>Email</dt><dd>{user.email ?? "–"}</dd></div>
                <div><dt>Slack ID</dt><dd>{user.slack_id ?? "–"}</dd></div>
                <div><dt>HCA id</dt><dd>{user.hca_id}</dd></div>
                <div><dt>Mips</dt><dd>{user.mips}</dd></div>
                <div>
                    <dt>Verification</dt><dd>{user.verification_status ?? "–"}</dd>
                </div>
                <div><dt>Projects</dt><dd>{user.projects}</dd></div>
                <div><dt>Signed up</dt><dd>{short_time(user.signup_at)}</dd></div>
                <div>
                    <dt>Last active</dt>
                    <dd>
                        {user.last_activity_at
                            ? short_time(user.last_activity_at)
                            : "–"}
                    </dd>
                </div>
            </dl>
            {#snippet actions()}
                <button type="button" onclick={() => open(user, "mips")}
                    >Set Mips</button
                >
                <button type="button" onclick={() => open(user, "adjust")}
                    >Add or remove Mips</button
                >
                <button type="button" onclick={() => open(user, "ban")}
                    >{user.banned ? "Unban" : "Ban"}</button
                >
            {/snippet}
        </Card>
    {/snippet}
</PagedList>

{#if editing?.kind === "mips"}
    {@const user = editing.user}
    <ActionDialog
        title="Set Mips for {display_name(user.name, user.id)}"
        submit="Set Mips"
        warning={checking ||
            check_error ||
            (valid && parsed < balance
                ? `They have ${balance} Mips, so this takes away ${balance - parsed}`
                : `They have ${balance} Mips`)}
        {valid}
        bind:reason
        onsubmit={async () => {
            pages.replace(await set_mips(user.id, { mips: parsed, reason }));
            onchange();
        }}
        onclose={() => (editing = null)}
    >
        <Field
            label="Mips"
            type="number"
            min="0"
            max="10000000"
            step="1"
            focus
            bind:value={mips}
        />
    </ActionDialog>
{:else if editing?.kind === "adjust"}
    {@const user = editing.user}
    <ActionDialog
        title="Add or remove Mips for {display_name(user.name, user.id)}"
        submit={valid && parsed < 0 ? "Remove Mips" : "Add Mips"}
        warning={checking ||
            check_error ||
            (valid && parsed < 0
                ? `This takes away ${-parsed} Mips, leaving ${total}`
                : undefined)}
        {valid}
        bind:reason
        onsubmit={async () => {
            pages.replace(await adjust_mips(user.id, { change: parsed, reason }));
            onchange();
        }}
        onclose={() => (editing = null)}
    >
        <Field
            label="Mips to add"
            hint="They have {balance}. Use a negative number to remove Mips"
            type="number"
            step="1"
            focus
            bind:value={mips}
        />
    </ActionDialog>
{:else if editing?.kind === "ban"}
    {@const user = editing.user}
    <ActionDialog
        title="{user.banned ? 'Unban' : 'Ban'} {display_name(user.name, user.id)}?"
        submit={user.banned ? "Unban" : "Ban"}
        warning={user.banned
            ? "They'll be able to use Mipmap again"
            : "They'll be locked out of Mipmap and their Snapshots will be hidden"}
        bind:reason
        onsubmit={async () =>
            pages.replace(
                await set_banned(user.id, { banned: !user.banned, reason }),
            )}
        onclose={() => (editing = null)}
    />
{/if}

<style>
    .search {
        max-width: 28rem;
    }
</style>
