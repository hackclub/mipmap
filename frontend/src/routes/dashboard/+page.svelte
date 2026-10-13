<script lang="ts">
    import { onMount } from "svelte";
    import { request } from "$lib/api";
    import GuidesDialog from "$lib/guides/GuidesDialog.svelte";
    import Tour from "$lib/guides/Tour.svelte";
    import { steps, type Part } from "$lib/guides/TourSteps.svelte";
    import LinkHackatime from "$lib/dashboard/LinkHackatime.svelte";
    import LoadError from "$lib/ui/LoadError.svelte";
    import NavMenu from "$lib/dashboard/NavMenu.svelte";
    import PanZoom, { type Bounds } from "$lib/dashboard/PanZoom.svelte";
    import { Load } from "$lib/load.svelte";
    import { account, dashboard } from "$lib/dashboard/state.svelte";
    import { arrange } from "$lib/dashboard/sections";
    import ProfilePanel from "$lib/account/ProfilePanel.svelte";
    import ReferralPanel from "$lib/account/ReferralPanel.svelte";
    import FulfillmentPanel from "$lib/fulfillment/FulfillmentPanel.svelte";
    import FeedbackPanel from "$lib/feedback/FeedbackPanel.svelte";
    import OrdersPanel from "$lib/orders/OrdersPanel.svelte";
    import SnapshotsPanel from "$lib/snapshots/SnapshotsPanel.svelte";
    import CogArt from "$lib/tree/CogArt.svelte";
    import ProjectsPanel, {
        type ProjectDialog,
    } from "$lib/projects/ProjectsPanel.svelte";
    import ItemModal from "$lib/tree/ItemModal.svelte";
    import TechTree from "$lib/tree/TechTree.svelte";
    import item_metadata from "$lib/assets/item_metadata.json";
    import {
        fetch_layout,
        fetch_tree,
        purchase_item,
        unlock_item,
    } from "$lib/tree/api";
    import { node_color } from "$lib/tree/style";
    import type { ItemMetadata } from "$lib/tree/types";

    const metadata: ItemMetadata = item_metadata;

    let selected = $state<number | null>(null);
    let panzoom = $state<PanZoom>();
    let guide = $state<"welcome" | "help" | null>(null);
    let welcomed = $state(false);
    let touring = $state(false);
    let replay = $state(false);
    let tour_step = $state(0);
    let project_dialog = $state<ProjectDialog>(null);

    const structure = new Load(() =>
        Promise.all([fetch_tree(), fetch_layout()]),
    );

    const data = $derived(dashboard.data);
    const unlocked = $derived(new Set(data?.unlocked));
    const purchased = $derived(new Set(data?.purchased));
    const tree = $derived(structure.data?.[0]);
    const layout = $derived(structure.data?.[1]);
    const sections = $derived(layout && arrange(layout));
    const start = $derived.by(() => {
        const id = tree?.nodes.find((node) => node.kind === "origin")?.id;
        return layout?.nodes.find((node) => node.id === id);
    });
    const around = (at: { x: number; y: number }, size: number) =>
        sections && {
            left: sections.tree.left + at.x - size / 2,
            top: sections.tree.top + at.y - size / 2,
            width: size,
            height: size,
        };
    const origin = $derived(
        start && layout && around(start, layout.shapes.origin.cog),
    );
    const nearest_token = $derived.by(() => {
        if (!start || !layout) return;
        return layout.nodes
            .filter((node) => nodes.get(node.id)?.kind === "token")
            .sort(
                (a, b) =>
                    Math.hypot(a.x - start.x, a.y - start.y) -
                    Math.hypot(b.x - start.x, b.y - start.y),
            )[0];
    });
    const token = $derived(
        nearest_token &&
            layout &&
            around(nearest_token, layout.shapes.token.cog * 4),
    );
    const go = (bounds: Bounds) =>
        panzoom?.show(bounds, { ms: 700, padding: 24, cap: Infinity });

    const nodes = $derived(new Map(tree?.nodes.map((node) => [node.id, node])));
    const selected_node = $derived(
        selected === null ? undefined : nodes.get(selected),
    );
    const tokens = $derived(
        [...unlocked].filter((id) => nodes.get(id)?.kind === "token").length -
            purchased.size,
    );
    const available = $derived(
        selected_node?.requires.some(
            (id) => unlocked.has(id) || nodes.get(id)?.kind === "origin",
        ) ?? false,
    );

    const focus = $derived(touring ? steps[tour_step].focus : undefined);
    const faded = (part: Part) => !!focus && !focus.includes(part);

    $effect(() => {
        if (!data || data.user.guides_seen || welcomed) return;
        welcomed = true;
        guide = "welcome";
        request("/api/guides/seen", { method: "POST" }).catch(() => {});
    });

    function start_tour() {
        replay = guide === "help";
        guide = null;
        tour_step = 0;
        touring = true;
    }

    onMount(() => {
        structure.run();
        dashboard.run();
    });

    async function act(action: (id: number) => Promise<void>) {
        if (selected === null) return;
        try {
            await action(selected);
        } catch (e) {
            dashboard.run();
            throw e;
        }
    }
</script>

<svelte:head>
    <title>Dashboard - Hack Club Mipmap</title>
</svelte:head>

{#if account.suspended}
    <div class="status">Your account has been suspended</div>
{:else if tree && layout && sections && data}
    <PanZoom
        bind:this={panzoom}
        home={sections.tree}
        limits={sections.all}
        keep={origin}
        locked={touring}
    >
        <div class="part" class:faded={faded("projects")}>
            <ProjectsPanel
                bounds={sections.projects}
                projects={data.projects}
                hackatime={data.hackatime_projects}
                bind:dialog={project_dialog}
            />
        </div>
        <div class="part" class:faded={faded("tree")}>
            <TechTree
                left={sections.tree.left}
                {nodes}
                {layout}
                {metadata}
                currency={tree.currency}
                {unlocked}
                {purchased}
                onselect={(id) => (selected = id)}
            />
        </div>
        <div class="part" class:faded={faded("fulfillment")}>
            <FulfillmentPanel bounds={sections.fulfillment} />
        </div>
        <div class="part" class:faded={faded("orders")}>
            <OrdersPanel bounds={sections.orders} orders={data.orders} />
        </div>
        <div class="part" class:faded={faded("feedback")}>
            <FeedbackPanel bounds={sections.feedback} />
        </div>
        <div class="part" class:faded={faded("referral")}>
            <ReferralPanel
                bounds={sections.referral}
                code={data.user.referral_code}
            />
        </div>
        <div class="part" class:faded={faded("profile")}>
            <ProfilePanel
                bounds={sections.profile}
                currency={tree.currency}
                user={data.user}
                {tokens}
            />
        </div>
        <div class="part" class:faded={faded("snapshots")}>
            <SnapshotsPanel
                bounds={sections.snapshots}
                snapshots={data.snapshots}
            />
        </div>
    </PanZoom>

    <button
        type="button"
        class="help"
        class:locked={touring}
        inert={touring}
        aria-label="Help"
        onclick={() => (guide = "help")}>?</button
    >

    <div class="nav" class:locked={touring} inert={touring}>
        <NavMenu
            items={[
                { label: "Project", onselect: () => go(sections.projects) },
                {
                    label: "Fulfillment Rounds",
                    onselect: () => go(sections.fulfillment),
                },
                { label: "Orders", onselect: () => go(sections.orders) },
                { label: "Referrals", onselect: () => go(sections.referral) },
                { label: "Profile", onselect: () => go(sections.profile) },
                { label: "Feedback", onselect: () => go(sections.feedback) },
                { label: "Snapshots", onselect: () => go(sections.snapshots) },
            ]}
        />
    </div>

    <div class="banners">
        {#if data.user.blocked}
            <div class="banner">
                {data.user.blocked}
                <a href="https://auth.hackclub.com">Visit Hack Club Auth</a>
            </div>
        {/if}
        {#if dashboard.error}
            <div class="banner">
                <LoadError
                    error="Couldn't load your dashboard: {dashboard.error}"
                    onretry={dashboard.run}
                />
            </div>
        {/if}
    </div>

    {#if guide}
        <GuidesDialog
            title={guide === "welcome" ? "Check out Mipmap!" : "Mipmap Guides"}
            ontour={start_tour}
            onclose={() => (guide = null)}
        />
    {/if}

    {#if touring}
        <Tour
            sections={{ ...sections, token: token ?? sections.tree }}
            {panzoom}
            projects={data.projects}
            token={nearest_token?.id}
            {replay}
            bind:step={tour_step}
            bind:dialog={project_dialog}
            onclose={() => {
                touring = false;
                panzoom?.settle();
            }}
        />
    {/if}

    {#if !data.user.hackatime_linked &&
        (data.user.guides_seen || welcomed) &&
        !guide &&
        !touring}
        <LinkHackatime />
    {/if}

    {#if selected_node}
        <ItemModal
            node={selected_node}
            meta={metadata[selected_node.id]}
            currency={tree.currency}
            unlocked={unlocked.has(selected_node.id)}
            purchased={purchased.has(selected_node.id)}
            {tokens}
            {available}
            blocked={data.user.blocked}
            image={selected_node.kind === "token" ? token_image : undefined}
            onclose={() => (selected = null)}
            onunlock={() => act(unlock_item)}
            onpurchase={() => act(purchase_item)}
        />
    {/if}
{:else if structure.error || dashboard.error}
    <div class="status">
        <LoadError
            error={structure.error || dashboard.error}
            onretry={() => {
                if (structure.error) structure.run();
                if (dashboard.error) dashboard.run();
            }}
        />
    </div>
{/if}

{#snippet token_image()}
    {#if selected_node}
        <CogArt kind="token" color={node_color(selected_node)} size={160} />
    {/if}
{/snippet}

<style>
    .part.faded {
        opacity: 0.15;
    }

    .help {
        position: fixed;
        right: calc(1rem + 3rem + 4px + 0.5rem);
        bottom: 1rem;
        display: grid;
        place-items: center;
        width: calc(3rem + 4px);
        height: calc(3rem + 4px);
        padding: 0;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font: inherit;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s,
            opacity 0.3s;
    }

    .help:hover,
    .help:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    .help.locked,
    .nav.locked {
        opacity: 0.4;
    }

    .nav {
        transition: opacity 0.3s;
    }

    .status {
        position: fixed;
        inset: 0;
        display: grid;
        place-items: center;
        padding: 1.5rem;
        color: #ead7c2;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
    }

    .banners {
        position: fixed;
        top: 1rem;
        left: 50%;
        transform: translateX(-50%);
        display: flex;
        flex-direction: column;
        gap: 1rem;
        width: max-content;
        max-width: calc(100vw - 2rem);
    }

    .banner {
        box-sizing: border-box;
        padding: 1rem 1.5rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
    }
</style>
