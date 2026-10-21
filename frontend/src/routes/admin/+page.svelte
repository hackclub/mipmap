<script lang="ts">
    import { onMount, tick } from "svelte";
    import { pushState } from "$app/navigation";
    import Overview from "$lib/admin/Overview.svelte";
    import ShipsTab from "$lib/admin/ShipsTab.svelte";
    import SnapshotsTab from "$lib/admin/SnapshotsTab.svelte";
    import RejectedSnapshotsTab from "$lib/admin/RejectedSnapshotsTab.svelte";
    import UsersTab from "$lib/admin/UsersTab.svelte";
    import OrdersTab from "$lib/admin/OrdersTab.svelte";
    import StickersTab from "$lib/admin/StickersTab.svelte";
    import FeedbackTab from "$lib/admin/FeedbackTab.svelte";
    import SurveysTab from "$lib/admin/SurveysTab.svelte";
    import AuditTab from "$lib/admin/AuditTab.svelte";
    import AnnouncementsTab from "$lib/admin/AnnouncementsTab.svelte";
    import InboxTab from "$lib/admin/InboxTab.svelte";
    import LocationTab from "$lib/admin/location/LocationTab.svelte";
    import { session } from "$lib/admin/session.svelte";

    const tabs = [
        ["ships", "Ships"],
        ["snapshots", "Snapshots"],
        ["rejected", "Rejected Snapshots"],
        ["users", "Users"],
        ["orders", "Orders"],
        ["stickers", "Stickers"],
        ["feedback", "Feedback"],
        ["surveys", "Surveys"],
        ["location", "Location"],
        ["announcements", "Announcements"],
        ["inbox", "Slack DMs"],
        ["audit", "Audit log"],
    ] as const;

    type Key = (typeof tabs)[number][0];

    let tab = $state<Key>("ships");
    let visited = $state<Partial<Record<Key, boolean>>>({});
    let lookup = $state<{ hca_id: string } | null>(null);
    let overview = $state<Overview>();
    const refresh = () => overview?.refresh();

    const valid = (key: string | null): key is Key =>
        tabs.some(([k]) => k === key);

    function show(key: Key) {
        tab = key;
        visited[key] = true;
    }

    function select(key: Key) {
        if (key === tab) return;
        show(key);
        pushState(`${location.pathname}?tab=${key}`, {});
    }

    function from_url() {
        if (!/^\/admin\/?$/.test(location.pathname)) return;
        const key = new URLSearchParams(location.search).get("tab");
        show(valid(key) ? key : "ships");
    }

    onMount(from_url);

    function keydown(e: KeyboardEvent, i: number) {
        const target =
            e.key === "ArrowRight"
                ? (i + 1) % tabs.length
                : e.key === "ArrowLeft"
                  ? (i - 1 + tabs.length) % tabs.length
                  : e.key === "Home"
                    ? 0
                    : e.key === "End"
                      ? tabs.length - 1
                      : -1;
        if (target < 0) return;
        e.preventDefault();
        document.getElementById(`tab-${tabs[target][0]}`)?.focus();
    }

    async function open_user(hca_id: string) {
        select("users");
        await tick();
        lookup = { hca_id };
    }
</script>

<svelte:head>
    <title>Admin - Hack Club Mipmap</title>
    <meta name="robots" content="noindex" />
</svelte:head>

<svelte:window onpopstate={from_url} />

<main>
    <header>
        <h1>Mipmap Admin</h1>
        <a href="/dashboard">Dashboard</a>
    </header>

    <div role="alert">
        {#if session.ended}
            <p class="ended">
                Your admin session ended. <a
                    href="/login?next={encodeURIComponent(`/admin?tab=${tab}`)}"
                    rel="external">Log in again</a
                >
            </p>
        {/if}
    </div>

    <div class="layout">
        <section class="tabs">
            <div class="tablist" role="tablist" aria-label="Admin sections">
                {#each tabs as [key, label], i (key)}
                    <button
                        type="button"
                        role="tab"
                        id="tab-{key}"
                        aria-selected={tab === key}
                        aria-controls="panel-{key}"
                        tabindex={tab === key ? 0 : -1}
                        class:active={tab === key}
                        onclick={() => select(key)}
                        onkeydown={(e) => keydown(e, i)}>{label}</button
                    >
                {/each}
            </div>
            {#each tabs as [key, label] (key)}
                {#if visited[key]}
                    <div
                        class="panel"
                        id="panel-{key}"
                        role="tabpanel"
                        aria-labelledby="tab-{key}"
                        hidden={tab !== key}
                    >
                        <h2 class="sr">{label}</h2>
                        {#if key === "ships"}
                            <ShipsTab onchange={refresh} />
                        {:else if key === "snapshots"}
                            <SnapshotsTab />
                        {:else if key === "rejected"}
                            <RejectedSnapshotsTab />
                        {:else if key === "users"}
                            <UsersTab onchange={refresh} {lookup} />
                        {:else if key === "orders"}
                            <OrdersTab onchange={refresh} />
                        {:else if key === "stickers"}
                            <StickersTab onchange={refresh} />
                        {:else if key === "feedback"}
                            <FeedbackTab />
                        {:else if key === "surveys"}
                            <SurveysTab />
                        {:else if key === "location"}
                            <LocationTab onopenuser={open_user} />
                        {:else if key === "announcements"}
                            <AnnouncementsTab />
                        {:else if key === "inbox"}
                            <InboxTab />
                        {:else}
                            <AuditTab />
                        {/if}
                    </div>
                {/if}
            {/each}
        </section>
        <div class="side">
            <Overview bind:this={overview} />
        </div>
    </div>
</main>

<style>
    main {
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: 1rem;
        max-width: 80rem;
        margin: 0 auto;
        padding: 2rem 1rem;
        color: #ead7c2;
    }

    header {
        display: flex;
        flex-wrap: wrap;
        justify-content: space-between;
        align-items: baseline;
        gap: 0.5rem 1rem;
    }

    h1 {
        margin: 0;
        font-size: 2rem;
    }

    .ended {
        margin: 0;
        padding: 0.75rem 1rem;
        background: #4f2f1f;
        border: 2px solid #d6ab70;
        border-radius: 7px;
        font-weight: 700;
    }

    .sr {
        position: absolute;
        width: 1px;
        height: 1px;
        margin: 0;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }

    .layout {
        display: grid;
        grid-template-columns: minmax(0, 1fr) 18rem;
        align-items: start;
        gap: 1rem;
    }

    .side {
        position: sticky;
        top: 1rem;
    }

    .tabs {
        min-width: 0;
        padding: 1.25rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    .tablist {
        display: flex;
        flex-wrap: wrap;
        gap: 0.25rem;
        margin-bottom: 1rem;
    }

    .tablist button {
        padding: 0.5rem 1rem;
        background-color: #4f2f1f;
        border: none;
        color: #ead7c2;
        font: inherit;
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .tablist button.active {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    .tablist button:focus-visible {
        outline: 2px dashed #ead7c2;
        outline-offset: 2px;
    }

    @media (hover: hover) {
        .tablist button:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    .panel {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .panel[hidden] {
        display: none;
    }

    @media (max-width: 56rem) {
        .layout {
            grid-template-columns: minmax(0, 1fr);
        }

        .side {
            position: static;
        }
    }

    @media (max-width: 40rem) {
        .tabs {
            padding: 1rem 0.75rem;
        }

        .tablist {
            flex-wrap: nowrap;
            overflow-x: auto;
            padding: 4px;
            scrollbar-width: thin;
        }

        .tablist button {
            flex: none;
            white-space: nowrap;
        }
    }
</style>
