<script lang="ts">
    import { onMount } from "svelte";
    import Onboarding from "$lib/onboarding/Onboarding.svelte";
    import LinkHackatime from "$lib/dashboard/LinkHackatime.svelte";
    import Steve from "$lib/feedback/Steve.svelte";
    import Dialog from "$lib/ui/Dialog.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { unsaved } from "$lib/onboarding/api";
    import LoadError from "$lib/ui/LoadError.svelte";
    import NavMenu from "$lib/dashboard/NavMenu.svelte";
    import PanZoom, { type Bounds } from "$lib/dashboard/PanZoom.svelte";
    import { Load } from "$lib/load.svelte";
    import { account, dashboard } from "$lib/dashboard/state.svelte";
    import { arrange } from "$lib/dashboard/sections";
    import ProfilePanel from "$lib/account/ProfilePanel.svelte";
    import ReferralPanel from "$lib/account/ReferralPanel.svelte";
    import ChallengePanel from "$lib/challenges/ChallengePanel.svelte";
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
    let reset = $state(unsaved());

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
    const sticker_done = $derived(
        purchased.has(STICKER) || !!data?.user.sticker_skipped,
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
        requires: node.requires
            .map((id) => nodes.get(id))
            .filter((req) => req && req.kind !== "origin")
            .map((req) => metadata[req!.id]?.title ?? req!.name),
        blocked: tutorial(node.id) ? null : (data?.user.blocked ?? null),
        skippable: node.id === STICKER && sticker_tokens > 0,
    });
    let kept: ReturnType<typeof view> | undefined;
    const item = $derived(selected_node ? (kept = view(selected_node)) : kept);

    const onboarded = $derived(!!data?.user.onboarded || finished);

    $effect(() => {
        if (data && !onboarded && !touring && spots && !reset)
            touring = "first";
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
    <meta name="robots" content="noindex" />
</svelte:head>

{#if account.suspended}
    <div class="status">Your account has been suspended</div>
{:else if tree && layout && sections && data}
    <main aria-labelledby="dashboard-title">
        <h1 id="dashboard-title" class="title">Dashboard</h1>
        <PanZoom
            bind:this={panzoom}
            home={sections.tree}
            limits={sections.all}
            keep={origin}
            targets="button[data-node]"
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
            <FulfillmentPanel bounds={sections.fulfillment} />
            <OrdersPanel bounds={sections.orders} orders={data.orders} />
            <ReferralPanel
                bounds={sections.referral}
                code={data.user.referral_code}
            />
            <ChallengePanel bounds={sections.challenge} />
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
        </PanZoom>
    </main>

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
                {
                    label: "Challenges",
                    onselect: () => go(sections.challenge),
                },
                { label: "Profile", onselect: () => go(sections.profile) },
                { label: "Help", onselect: () => go(sections.help) },
                { label: "Snapshots", onselect: () => go(sections.snapshots) },
            ]}
        />
    </div>

    <aside class="banners" class:touring aria-label="Notices">
        {#if !touring && sticker_done && data.user.blocked}
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
    </aside>

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

    {#if reset && !onboarded}
        <Dialog labelledby="reset-title" width="28rem">
            <p id="reset-title" class="reset">
                Your onboarding progress couldn't be saved so you were reset to
                the start
            </p>
            <div class="actions">
                <MipmapButton
                    type="button"
                    data-autofocus
                    onclick={() => {
                        unsaved(true);
                        reset = false;
                    }}
                >
                    OK
                </MipmapButton>
            </div>
        </Dialog>
    {/if}

    {#if !data.user.hackatime_linked && onboarded && !touring}
        <LinkHackatime />
    {/if}

    <Steve active={onboarded && !touring && !reset} />

    {#if selected_node && item}
        <ItemModal
            node={item.node}
            meta={item.meta}
            currency={tree.currency}
            unlocked={item.unlocked}
            purchased={item.purchased}
            tokens={item.tokens}
            available={item.available}
            requires={item.requires}
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
    .title {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
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

    .reset {
        margin: 0;
    }

    .actions {
        display: flex;
        justify-content: flex-end;
        margin-top: 1.5rem;
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
