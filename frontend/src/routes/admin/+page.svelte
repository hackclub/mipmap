<script lang="ts">
    import Overview from "$lib/admin/Overview.svelte";
    import ShipsTab from "$lib/admin/ShipsTab.svelte";
    import SnapshotsTab from "$lib/admin/SnapshotsTab.svelte";
    import UsersTab from "$lib/admin/UsersTab.svelte";
    import OrdersTab from "$lib/admin/OrdersTab.svelte";
    import FeedbackTab from "$lib/admin/FeedbackTab.svelte";
    import AuditTab from "$lib/admin/AuditTab.svelte";

    const tabs = [
        ["ships", "Ships"],
        ["snapshots", "Snapshots"],
        ["users", "Users"],
        ["orders", "Orders"],
        ["feedback", "Feedback"],
        ["audit", "Audit log"],
    ] as const;

    let tab = $state<(typeof tabs)[number][0]>("ships");
    let overview = $state<Overview>();
    const refresh = () => overview?.refresh();

    function keydown(e: KeyboardEvent, i: number) {
        const step = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
        if (!step) return;
        e.preventDefault();
        const next = (i + step + tabs.length) % tabs.length;
        tab = tabs[next][0];
        document.getElementById(`tab-${tab}`)?.focus();
    }
</script>

<svelte:head>
    <title>Admin - Hack Club Mipmap</title>
    <meta name="robots" content="noindex" />
</svelte:head>

<main>
    <header>
        <h1>Mipmap Admin</h1>
        <a href="/dashboard">Dashboard</a>
    </header>

    <div class="layout">
        <section class="tabs">
            <div class="tablist" role="tablist" aria-label="Admin sections">
                {#each tabs as [key, label], i (key)}
                    <button
                        type="button"
                        role="tab"
                        id="tab-{key}"
                        aria-selected={tab === key}
                        aria-controls="panel"
                        tabindex={tab === key ? 0 : -1}
                        class:active={tab === key}
                        onclick={() => (tab = key)}
                        onkeydown={(e) => keydown(e, i)}>{label}</button
                    >
                {/each}
            </div>
            <div
                class="panel"
                id="panel"
                role="tabpanel"
                aria-labelledby="tab-{tab}"
            >
                {#key tab}
                    {#if tab === "ships"}
                        <ShipsTab onchange={refresh} />
                    {:else if tab === "snapshots"}
                        <SnapshotsTab />
                    {:else if tab === "users"}
                        <UsersTab onchange={refresh} />
                    {:else if tab === "orders"}
                        <OrdersTab onchange={refresh} />
                    {:else if tab === "feedback"}
                        <FeedbackTab />
                    {:else}
                        <AuditTab />
                    {/if}
                {/key}
            </div>
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

    .tablist button:focus-visible,
    .tablist button.active {
        background-color: #ead7c2;
        color: #4f2f1f;
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

    @media (max-width: 56rem) {
        .layout {
            grid-template-columns: minmax(0, 1fr);
        }

        .side {
            position: static;
            order: -1;
        }
    }
</style>
