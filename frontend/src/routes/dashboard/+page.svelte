<script lang="ts">
    import { onMount } from "svelte";
    import LoadError from "$lib/LoadError.svelte";
    import NavMenu from "$lib/NavMenu.svelte";
    import PanZoom, { type Bounds } from "$lib/PanZoom.svelte";
    import { Load } from "$lib/load.svelte";
    import { arrange } from "$lib/sections";
    import ProfilePanel from "$lib/account/ProfilePanel.svelte";
    import ReferralPanel from "$lib/account/ReferralPanel.svelte";
    import FulfillmentPanel from "$lib/fulfillment/FulfillmentPanel.svelte";
    import OrdersPanel from "$lib/orders/OrdersPanel.svelte";
    import SnapshotsPanel from "$lib/snapshots/SnapshotsPanel.svelte";
    import CogArt from "$lib/tree/CogArt.svelte";
    import ProjectsPanel from "$lib/projects/ProjectsPanel.svelte";
    import ItemModal from "$lib/tree/ItemModal.svelte";
    import TechTree from "$lib/tree/TechTree.svelte";
    import item_metadata from "$lib/assets/item_metadata.json";
    import {
        fetch_layout,
        fetch_progress,
        fetch_tree,
        purchase_item,
        unlock_item,
        type Progress,
    } from "$lib/tree/api";
    import { node_color } from "$lib/tree/style";
    import type { ItemMetadata } from "$lib/tree/types";

    const metadata: ItemMetadata = item_metadata;

    let unlocked = $state.raw<Set<number>>();
    let purchased = $state.raw(new Set<number>());
    let selected = $state<number | null>(null);
    let panzoom = $state<PanZoom>();

    function merge(progress: Progress) {
        unlocked = new Set([...(unlocked ?? []), ...progress.unlocked]);
        purchased = new Set([...purchased, ...progress.purchased]);
    }

    const structure = new Load(() =>
        Promise.all([fetch_tree(), fetch_layout()]),
    );
    const progress = new Load(() => fetch_progress().then(merge));

    const tree = $derived(structure.data?.[0]);
    const layout = $derived(structure.data?.[1]);
    const sections = $derived(layout && arrange(layout));
    const go = (bounds: Bounds) =>
        panzoom?.show(bounds, { ms: 700, floor: 0, padding: 24, cap: Infinity });

    const nodes = $derived(new Map(tree?.nodes.map((node) => [node.id, node])));
    const selected_node = $derived(
        selected === null ? undefined : nodes.get(selected),
    );
    const tokens = $derived(
        [...(unlocked ?? [])].filter((id) => nodes.get(id)?.kind === "token")
            .length - purchased.size,
    );
    const available = $derived(
        selected_node?.requires.some(
            (id) => unlocked?.has(id) || nodes.get(id)?.kind === "origin",
        ) ?? false,
    );

    onMount(() => {
        structure.run();
        progress.run();
    });

    async function act(action: (id: number) => Promise<Progress>) {
        if (selected === null) return;
        try {
            merge(await action(selected));
        } catch (e) {
            progress.run();
            throw e;
        }
    }
</script>

<svelte:head>
    <title>Dashboard - Hack Club Mipmap</title>
</svelte:head>

{#if tree && layout && sections}
    <PanZoom
        bind:this={panzoom}
        home={sections.tree}
        limits={sections.all}
        align="bottom"
        min_home_scale={0.6}
    >
        <ProjectsPanel bounds={sections.projects} />
        {#if unlocked}
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
        {/if}
        <FulfillmentPanel bounds={sections.fulfillment} />
        <OrdersPanel bounds={sections.orders} />
        <ReferralPanel bounds={sections.referral} />
        <ProfilePanel
            bounds={sections.profile}
            currency={tree.currency}
            tokens={unlocked ? tokens : undefined}
        />
        <SnapshotsPanel bounds={sections.snapshots} />
    </PanZoom>

    <NavMenu
        items={[
            { label: "Project", onselect: () => go(sections.projects) },
            { label: "Snapshots", onselect: () => go(sections.snapshots) },
            {
                label: "Fulfillment Rounds",
                onselect: () =>
                    go(sections.fulfillment),
            },
            {
                label: "Referrals",
                onselect: () => go(sections.referral),
            },
            {
                label: "Profile",
                onselect: () => go(sections.profile),
            },
        ]}
    />

    {#if progress.error}
        <div class="banner">
            <LoadError
                error="Couldn't load your progress: {progress.error}"
                onretry={progress.run}
            />
        </div>
    {/if}

    {#if selected_node && unlocked}
        <ItemModal
            node={selected_node}
            meta={metadata[selected_node.id]}
            currency={tree.currency}
            unlocked={unlocked.has(selected_node.id)}
            purchased={purchased.has(selected_node.id)}
            {tokens}
            {available}
            image={selected_node.kind === "token" ? token_image : undefined}
            onclose={() => (selected = null)}
            onunlock={() => act(unlock_item)}
            onpurchase={() => act(purchase_item)}
        />
    {/if}
{:else if structure.error}
    <div class="status">
        <LoadError
            error={structure.error}
            onretry={() => {
                structure.run();
                if (progress.error) progress.run();
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
    .status {
        position: fixed;
        inset: 0;
        display: grid;
        place-items: center;
        padding: 1.5rem;
        color: #ead7c2;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
    }

    .banner {
        position: fixed;
        top: 1rem;
        left: 50%;
        transform: translateX(-50%);
        box-sizing: border-box;
        max-width: calc(100vw - 2rem);
        padding: 1rem 1.5rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
    }
</style>
