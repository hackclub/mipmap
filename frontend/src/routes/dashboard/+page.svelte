<script lang="ts">
    import { onMount, tick } from "svelte";
    import { replaceState } from "$app/navigation";
    import { message } from "$lib/api";
    import { log_out } from "$lib/account/logout";
    import Onboarding from "$lib/onboarding/Onboarding.svelte";
    import LinkHackatime from "$lib/dashboard/LinkHackatime.svelte";
    import Steve from "$lib/feedback/Steve.svelte";
    import Dialog from "$lib/ui/Dialog.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { unsaved } from "$lib/onboarding/api";
    import LoadError from "$lib/ui/LoadError.svelte";
    import NavMenu from "$lib/dashboard/NavMenu.svelte";
    import PanZoom, { type Bounds } from "$lib/dashboard/PanZoom.svelte";
    import { Load, Task } from "$lib/load.svelte";
    import { account, dashboard, refresh } from "$lib/dashboard/state.svelte";
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
    import AuthText from "$lib/ui/AuthText.svelte";
    import TechTree from "$lib/tree/TechTree.svelte";
    import item_metadata from "$lib/assets/item_metadata.json";
    import {
        fetch_layout,
        fetch_tree,
        friendly,
        purchase_item,
        skip_sticker,
        unlock_item,
    } from "$lib/tree/api";
    import { node_color, node_names } from "$lib/tree/style";
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
    let reset = $state(false);
    let reset_checked = false;
    let nav = $state<NavMenu>();
    let notices = $state<HTMLElement>();
    let dismissed = $state<string[]>([]);
    let hackatime_error = $state("");
    let late = $state("");
    let opened = 0;

    const logout = new Task("Couldn't log out");

    const hackatime_errors: Record<string, string> = {
        taken: "That Hackatime account is already linked to another Mipmap account. Log in to Hackatime with a different account and try again",
        other: "Your Mipmap account is already linked to a different Hackatime account, so link that one instead",
        failed: "Linking Hackatime didn't work, wait a moment and try again",
    };

    const shown_notice = (key: string) => !dismissed.includes(key);
    const dismiss = (key: string) => close_notice(() => (dismissed = [...dismissed, key]));

    async function close_notice(close: () => void) {
        close();
        await tick();
        if (document.activeElement !== document.body) return;
        const next = notices?.querySelector<HTMLElement>(".dismiss");
        if (next) next.focus();
        else nav?.focus();
    }

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
    async function go(label: string, bounds: Bounds, zoom = 3) {
        const cover = notices?.getBoundingClientRect();
        const inset = {
            left: 0,
            top: cover?.height ? cover.bottom : 0,
            right: 0,
            bottom: 0,
        };
        const arrived = await panzoom?.show(
            bounds,
            innerWidth < 768 || innerHeight < 500
                ? {
                      ms: 700,
                      padding: 16,
                      cap: Infinity,
                      floor: 12 / (20 * 0.85 * zoom),
                      align: "start",
                      inset,
                  }
                : { ms: 700, padding: 24, cap: Infinity, inset },
        );
        const active = document.activeElement;
        if (
            arrived &&
            (!active || active === document.body || active.closest(".nav"))
        )
            document
                .querySelector<HTMLElement>(`[data-heading="${label}"]`)
                ?.focus({ preventScroll: true });
    }

    const nodes = $derived(new Map(tree?.nodes.map((node) => [node.id, node])));
    const names = $derived(
        layout ? node_names(nodes, layout, metadata) : new Map<number, string>(),
    );
    const selected_node = $derived(
        selected === null ? undefined : nodes.get(selected),
    );
    const tokens = $derived(
        Math.max(
            0,
            [...unlocked].filter(
                (id) => id !== STICKER_TOKEN && nodes.get(id)?.kind === "token",
            ).length - [...purchased].filter((id) => id !== STICKER).length,
        ),
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
            .map((req) => names.get(req!.id) ?? req!.name),
        blocked: tutorial(node.id) ? null : (data?.user.blocked ?? null),
        skippable: node.id === STICKER && sticker_tokens > 0,
    });
    let kept: ReturnType<typeof view> | undefined;
    const item = $derived(selected_node ? (kept = view(selected_node)) : kept);

    const onboarded = $derived(!!data?.user.onboarded || finished);

    $effect(() => {
        if (data && !reset_checked) {
            reset_checked = true;
            reset = unsaved(false, data.user.referral_code);
        }
    });

    $effect(() => {
        if (data && !onboarded && !touring && spots && !reset)
            touring = "first";
    });

    $effect(() => {
        if (selected !== null) opened++;
    });

    onMount(() => {
        structure.run();
        dashboard.run();
        const params = new URLSearchParams(location.search);
        const code = params.get("hackatime_error");
        if (code !== null) {
            hackatime_error = hackatime_errors[code] ?? hackatime_errors.failed;
            params.delete("hackatime_error");
            const query = params.toString() ? `?${params}` : "";
            setTimeout(() =>
                replaceState(`${location.pathname}${query}${location.hash}`, {}),
            );
        }
        const visible = () => {
            if (document.visibilityState === "visible" && dashboard.data)
                refresh();
        };
        document.addEventListener("visibilitychange", visible);
        return () => document.removeEventListener("visibilitychange", visible);
    });

    async function act(
        action: (id: number) => Promise<void>,
        verb: "unlock" | "redeem" | "skip",
    ) {
        if (selected === null) return;
        const id = selected;
        const at = opened;
        try {
            await action(id);
        } catch (e) {
            await dashboard.run();
            if (selected === null || opened !== at) {
                const subject =
                    verb === "skip"
                        ? "skip the sticker"
                        : `${verb} ${names.get(id) ?? "that item"}`;
                late = `Couldn't ${subject}: ${friendly(message(e))}`;
            }
            throw e;
        }
    }
</script>

<svelte:head>
    <title>Dashboard - Hack Club Mipmap</title>
    <meta name="robots" content="noindex" />
</svelte:head>

{#if account.suspended}
    <div class="status">
        <div class="suspended">
            <p>Your account has been suspended</p>
            <MipmapButton
                type="button"
                disabled={logout.pending}
                onclick={() => logout.run(() => log_out())}
            >
                Log out
            </MipmapButton>
            {#if logout.error}
                <p class="small" role="alert">{logout.error}</p>
            {/if}
        </div>
    </div>
{:else}
    {#if tree && layout && sections && data && !touring}
        <a
            class="skip"
            href="#sections"
            onclick={(e) => {
                e.preventDefault();
                nav?.focus();
            }}>Skip to sections menu</a
        >
    {/if}

    <div
        class="banners"
        class:touring
        bind:this={notices}
        role="status"
        data-overlay
    >
        {#if data}
            {#if !touring && sticker_done && data.user.blocked && shown_notice("blocked")}
                <div class="banner">
                    <span>
                        <AuthText text={data.user.blocked} />
                        <a href="https://auth.hackclub.com" target="_blank" rel="noopener">
                            Visit Hack Club Auth and verify your ID or add an address
                        </a>
                    </span>
                    {@render dismiss_button("blocked")}
                </div>
            {/if}
            {#if !touring && data.user.hackatime_revoked && shown_notice("revoked")}
                <div class="banner">
                    <span>
                        Mipmap lost access to your Hackatime account, so your hours
                        aren't being tracked
                        <a href="/hackatime/login">Relink Hackatime</a>
                    </span>
                    {@render dismiss_button("revoked")}
                </div>
            {/if}
            {#if !touring && hackatime_error && data.user.hackatime_linked && shown_notice("hackatime")}
                <div class="banner">
                    <span>{hackatime_error}</span>
                    {@render dismiss_button("hackatime")}
                </div>
            {/if}
            {#if late}
                <div class="banner">
                    <span>{late}</span>
                    <button
                        type="button"
                        class="dismiss"
                        aria-label="Dismiss notice"
                        onclick={() => close_notice(() => (late = ""))}
                    >
                        {@render cross()}
                    </button>
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
        {/if}
    </div>

    {#if tree && layout && sections && data}
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
                    linked={data.user.hackatime_linked}
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
                    {names}
                    currency={tree.currency}
                    unlocked={shown_unlocked}
                    {purchased}
                    onselect={(id) => (selected = id)}
                />
            </PanZoom>
        </main>

        <div class="nav" id="sections" inert={!!touring}>
            <NavMenu
                bind:this={nav}
                items={(
                    [
                        ["Projects", sections.projects],
                        ["Fulfillment Rounds", sections.fulfillment],
                        ["Orders", sections.orders],
                        ["Referrals", sections.referral],
                        ["Challenges", sections.challenge],
                        ["Profile", sections.profile],
                        ["Help", sections.help],
                        ["Snapshots", sections.snapshots, 2.5],
                    ] as [string, Bounds, number?][]
                ).map(([label, bounds, zoom]) => ({
                    label,
                    onselect: () => go(label, bounds, zoom),
                }))}
            />
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
                slack_linked={data.user.slack_linked}
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
                    Some of your onboarding progress couldn't be saved, so you'll
                    pick up from the last step that was saved. Unlocks have been
                    kept.
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
            <LinkHackatime error={hackatime_error} />
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
                mips={data.user.mips}
                skipped={data.user.sticker_skipped}
                available={item.available}
                requires={item.requires}
                blocked={item.blocked}
                image={item.node.kind === "token" ? token_image : undefined}
                onclose={() => (selected = null)}
                onunlock={() => act(unlock_item, "unlock")}
                onpurchase={() => act(purchase_item, "redeem")}
                onskip={item.skippable
                    ? () => act(skip_sticker, "skip")
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
    {:else}
        <div class="status" role="status">Loading your dashboard…</div>
    {/if}
{/if}

{#snippet cross()}
    <svg viewBox="0 0 10 10" aria-hidden="true">
        <path
            d="M5 3.94 8.47.47A.75.75 0 0 1 9.53 1.53L6.06 5 9.53 8.47A.75.75 0 0 1 8.47 9.53L5 6.06 1.53 9.53A.75.75 0 0 1 .47 8.47L3.94 5 .47 1.53A.75.75 0 0 1 1.53.47Z"
        />
    </svg>
{/snippet}

{#snippet dismiss_button(key: string)}
    <button
        type="button"
        class="dismiss"
        aria-label="Dismiss notice"
        onclick={() => dismiss(key)}
    >
        {@render cross()}
    </button>
{/snippet}

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

    .suspended {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 1rem;
        text-align: center;
    }

    .suspended p {
        margin: 0;
    }

    .suspended .small {
        font-size: 0.85em;
    }

    .skip,
    .skip:visited {
        position: fixed;
        top: 1rem;
        left: 1rem;
        z-index: 2;
        padding: 0.75rem 1.5rem;
        background: #4f2f1f;
        color: #ead7c2;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
        font-weight: 700;
        text-decoration: none;
        transform: translateY(calc(-100% - 2rem));
    }

    .skip:focus {
        transform: none;
    }

    .banners {
        position: fixed;
        top: 1rem;
        left: 50%;
        z-index: 1;
        transform: translateX(-50%);
        display: flex;
        flex-direction: column;
        gap: 1rem;
        width: max-content;
        max-width: calc(100vw - 2rem);
        pointer-events: none;
    }

    .banners.touring {
        z-index: 1101;
    }

    .banner {
        display: flex;
        align-items: flex-start;
        gap: 1rem;
        box-sizing: border-box;
        padding: 1rem 1.5rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
    }

    .banner > :global(*) {
        pointer-events: auto;
    }

    .dismiss {
        flex: none;
        display: grid;
        place-items: center;
        width: 2.5rem;
        height: 2.5rem;
        margin: -0.5rem -1rem -0.5rem 0;
        padding: 0;
        background: none;
        border: none;
        border-radius: 7px;
        color: inherit;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .dismiss svg {
        width: 0.75em;
        height: 0.75em;
        fill: currentColor;
    }

    .dismiss:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .dismiss:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }
</style>
