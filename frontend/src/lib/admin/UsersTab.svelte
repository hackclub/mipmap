<script lang="ts">
    import { untrack } from "svelte";
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

    let { onchange, initial = "" }: { onchange: () => void; initial?: string } =
        $props();

    let search = $state(untrack(() => initial));
    let query = $state(untrack(() => initial));
    const pages = new Pages<User>((before) => fetch_users({ before, q: query }));

    $effect(() => {
        const next = search;
        const timer = setTimeout(() => (query = next), 300);
        return () => clearTimeout(timer);
    });

    $effect(() => {
        void query;
        untrack(pages.reset);
    });

    type Kind = "mips" | "adjust" | "ban";

    let editing = $state<{ user: User; kind: Kind } | null>(null);
    let mips = $state("");
    let reason = $state("");

    const parsed = $derived(Number(mips));
    const total = $derived(
        editing?.kind === "adjust" ? editing.user.mips + parsed : parsed,
    );
    const valid = $derived(
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
    }
</script>

<div class="search">
    <Field
        type="search"
        placeholder="Search name, email, Slack ID, HCA id or user id"
        aria-label="Search users"
        bind:value={search}
    />
</div>

<PagedList {pages} empty="No users found">
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
        warning={valid && parsed < user.mips
            ? `This takes away ${user.mips - parsed} Mips`
            : undefined}
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
        warning={valid && parsed < 0
            ? `This takes away ${-parsed} Mips, leaving ${total}`
            : undefined}
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
            hint="They have {user.mips}. Use a negative number to remove Mips"
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
