<script lang="ts">
    import { onMount } from "svelte";
    import Onboarding from "$lib/onboarding/Onboarding.svelte";
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
    import OrdersPanel from "$lib/orders/OrdersPanel.svelte";
    import StepsPanel from "$lib/dashboard/StepsPanel.svelte";
    import HelpPanel from "$lib/dashboard/HelpPanel.svelte";
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
        skip_sticker,
        unlock_item,
    } from "$lib/tree/api";
    import { node_color } from "$lib/tree/style";
    import { STICKER, STICKER_TOKEN } from "$lib/tree/tutorial";
    import type { ItemMetadata, TreeNode } from "$lib/tree/types";

    const metadata: ItemMetadata = item_metadata;

    let selected = $state<number | null>(null);
    let panzoom = $state<PanZoom>();
    let onboarding = $state<Onboarding>();
    let touring = $state<"first" | "replay" | null>(null);
    let finished = $state(false);
    let project_dialog = $state<ProjectDialog>(null);
    let demo = $state<Set<number>>(new Set());

    const structure = new Load(() =>
        Promise.all([fetch_tree(), fetch_layout()]),
    );

    const data = $derived(dashboard.data);
    const unlocked = $derived(new Set(data?.unlocked));
    const shown_unlocked = $derived(
        demo.size ? new Set([...unlocked, ...demo]) : unlocked,
    );
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
    const spot = (id: number) => {
        const at = layout?.nodes.find((node) => node.id === id);
        const kind = nodes.get(id)?.kind;
        return at && kind && layout && around(at, layout.shapes[kind].cog * 1.3);
    };
    const spots = $derived.by(() => {
        const [sticker, token] = [spot(STICKER), spot(STICKER_TOKEN)];
        return sticker && token ? { sticker, token } : undefined;
    });
    const go = (bounds: Bounds) =>
        panzoom?.show(bounds, { ms: 700, padding: 24, cap: Infinity });

    const nodes = $derived(new Map(tree?.nodes.map((node) => [node.id, node])));
    const selected_node = $derived(
        selected === null ? undefined : nodes.get(selected),
    );
    const tokens = $derived(
        [...unlocked].filter(
            (id) => id !== STICKER_TOKEN && nodes.get(id)?.kind === "token",
        ).length - [...purchased].filter((id) => id !== STICKER).length,
    );
    const sticker_tokens = $derived(
        unlocked.has(STICKER_TOKEN) &&
            !purchased.has(STICKER) &&
            !data?.user.sticker_skipped
            ? 1
            : 0,
    );
    const tutorial = (id: number) => id === STICKER || id === STICKER_TOKEN;
    const sticker_token_meta = {
        title: "Token",
        description: "This Token can only be spent on the Mipmap sticker",
        image: "",
    };
    const available = $derived(
        selected_node?.requires.some(
            (id) => unlocked.has(id) || nodes.get(id)?.kind === "origin",
        ) ?? false,
    );

    const view = (node: TreeNode) => ({
        node,
        meta:
            node.id === STICKER_TOKEN
                ? sticker_token_meta
                : metadata[node.id],
        unlocked: unlocked.has(node.id),
        purchased: purchased.has(node.id),
        tokens: node.id === STICKER ? sticker_tokens : tokens,
        available,
        blocked: tutorial(node.id) ? null : (data?.user.blocked ?? null),
        skippable: node.id === STICKER && sticker_tokens > 0,
    });
    let kept: ReturnType<typeof view> | undefined;
    const item = $derived(selected_node ? (kept = view(selected_node)) : kept);

    const onboarded = $derived(!!data?.user.onboarded || finished);

    $effect(() => {
        if (data && !onboarded && !touring && spots) touring = "first";
    });

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
    >
        <ProjectsPanel
            bounds={sections.projects}
            projects={data.projects}
            hackatime={data.hackatime_projects}
            bind:dialog={project_dialog}
            touring={!!touring}
            onsaved={(kind) => onboarding?.saved(kind)}
        />
        <StepsPanel bounds={sections.steps} />
        <TechTree
            left={sections.tree.left}
            {nodes}
            {layout}
            {metadata}
            currency={tree.currency}
            unlocked={shown_unlocked}
            {purchased}
            onselect={(id) => (selected = id)}
        />
        <FulfillmentPanel bounds={sections.fulfillment} />
        <OrdersPanel bounds={sections.orders} orders={data.orders} />
        <ReferralPanel
            bounds={sections.referral}
            code={data.user.referral_code}
        />
        <ProfilePanel
            bounds={sections.profile}
            currency={tree.currency}
            user={data.user}
            {tokens}
        />
        <HelpPanel
            bounds={sections.help}
            disabled={!spots}
            onreplay={() => (touring = "replay")}
        />
        <SnapshotsPanel
            bounds={sections.snapshots}
            snapshots={data.snapshots}
        />
    </PanZoom>

    <div class="nav" inert={!!touring}>
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
                { label: "Help", onselect: () => go(sections.help) },
                { label: "Snapshots", onselect: () => go(sections.snapshots) },
            ]}
        />
    </div>

    <div class="banners" class:touring>
        {#if !touring && data.user.blocked}
            <div class="banner">
                {data.user.blocked}
                <a href="https://auth.hackclub.com">Visit Hack Club Auth</a>
            </div>
        {/if}
        {#if !touring && data.user.hackatime_revoked}
            <div class="banner">
                Mipmap lost access to your Hackatime account, so your hours
                aren't being tracked
                <a href="/hackatime/login">Relink Hackatime</a>
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

    {#if touring && spots}
        <Onboarding
            bind:this={onboarding}
            {sections}
            {spots}
            {nodes}
            {panzoom}
            projects={data.projects}
            {unlocked}
            {purchased}
            granted={data.user.onboarding_granted}
            mips={data.user.mips}
            sticker_skipped={data.user.sticker_skipped}
            returning={data.user.saw_old_onboarding}
            stored={data.user.onboarding_step}
            replay={touring === "replay"}
            bind:dialog={project_dialog}
            bind:selected
            bind:demo
            onfinish={() => {
                if (touring === "first") finished = true;
                touring = null;
                project_dialog = null;
                panzoom?.settle();
            }}
        />
    {/if}

    {#if !data.user.hackatime_linked && onboarded && !touring}
        <LinkHackatime />
    {/if}

    {#if selected_node && item}
        <ItemModal
            node={item.node}
            meta={item.meta}
            currency={tree.currency}
            unlocked={item.unlocked}
            purchased={item.purchased}
            tokens={item.tokens}
            available={item.available}
            blocked={item.blocked}
            image={item.node.kind === "token" ? token_image : undefined}
            onclose={() => (selected = null)}
            onunlock={() => act(unlock_item)}
            onpurchase={() => act(purchase_item)}
            onskip={item.skippable
                ? () => act(skip_sticker)
                : undefined}
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
    {#if item}
        <CogArt kind="token" color={node_color(item.node)} size={160} />
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

    .banners.touring {
        z-index: 1101;
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
