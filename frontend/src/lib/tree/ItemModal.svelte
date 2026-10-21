<script lang="ts">
    import { tick, type Snippet } from "svelte";
    import Dialog, { refocus } from "$lib/ui/Dialog.svelte";
    import { Task } from "$lib/load.svelte";
    import { node_color, price_label } from "$lib/tree/style";
    import { friendly } from "$lib/tree/api";
    import CogArt from "$lib/tree/CogArt.svelte";
    import HoldButton from "$lib/tree/HoldButton.svelte";
    import AuthText from "$lib/ui/AuthText.svelte";
    import { STICKER } from "$lib/tree/tutorial";
    import type { ItemMeta, TreeNode } from "$lib/tree/types";

    let {
        node,
        meta,
        currency,
        unlocked,
        purchased,
        tokens,
        mips,
        skipped = false,
        available,
        requires,
        blocked,
        image,
        onclose,
        onunlock,
        onpurchase,
        onskip,
    }: {
        node: TreeNode;
        meta?: ItemMeta;
        currency: string;
        unlocked: boolean;
        purchased: boolean;
        tokens: number;
        mips: number;
        skipped?: boolean;
        available: boolean;
        requires: string[];
        blocked: string | null;
        image?: Snippet;
        onclose: () => void;
        onunlock: () => Promise<void>;
        onpurchase: () => Promise<void>;
        onskip?: () => Promise<void>;
    } = $props();

    const uid = $props.id();
    const token_description =
        "Gives you one Token which can be used to get yourself one prize";

    const title = $derived(meta?.title ?? node.name);
    const description = $derived(
        node.kind === "token"
            ? (meta?.description ?? token_description)
            : meta?.description,
    );
    const has_image = $derived(Boolean(image || meta?.image));
    const local = $derived(meta?.image.startsWith("/") ?? false);
    const actionable = $derived(
        !unlocked || (node.kind === "item" && !purchased),
    );
    const price = $derived(node.price ?? 0);
    const affordable = $derived(unlocked || price <= mips);

    const task = new Task();
    const pending_text = {
        unlock: "Unlocking…",
        redeem: "Redeeming…",
        skip: "Skipping…",
    };

    let customs = $state(false);
    let doing = $state<keyof typeof pending_text | null>(null);
    let status = $state("");
    let loaded = $state(false);
    let attempt = $state(0);

    const label = $derived(
        doing === "unlock"
            ? pending_text.unlock
            : unlocked
              ? "Unlocked!"
              : !available
                ? "Prerequisite missing"
                : affordable
                  ? "Unlock"
                  : `Not enough ${currency}`,
    );
    const source = $derived(
        local
            ? meta!.image
            : `/dashboard/item-images/${node.id}${attempt ? `?retry=${attempt}` : ""}`,
    );

    let footer: HTMLDivElement;
    let retry_timer = 0;

    $effect(() => () => clearTimeout(retry_timer));

    function retry() {
        if (local || attempt >= 12) return;
        clearTimeout(retry_timer);
        retry_timer = window.setTimeout(() => attempt++, 5000);
    }

    const done_text = (kind: keyof typeof pending_text) =>
        kind === "unlock"
            ? `Unlocked ${title}`
            : kind === "redeem"
              ? `Redeemed ${title}`
              : "Skipped the sticker";

    async function act(
        kind: keyof typeof pending_text,
        action: () => Promise<void>,
    ) {
        doing = kind;
        status = pending_text[kind];
        let succeeded = false;
        await task.run(action, () => (succeeded = true));
        doing = null;
        status = succeeded ? done_text(kind) : "";
        await tick();
        refocus(footer, true);
    }
</script>

<Dialog labelledby="{uid}-title" {onclose}>
    <div class="content" class:has-image={has_image}>
        <h2 id="{uid}-title">{title}</h2>
        {#if description || meta?.facts}
            <div class="description">
                {#if description}
                    <p>
                        {#if node.kind === "item"}<em>{description}</em>{:else}{description}{/if}
                    </p>
                {/if}
                {#if meta?.facts}
                    <p>{meta.facts}</p>
                {/if}
            </div>
        {/if}
        {#if has_image}
            <div class="image">
                <div class="frame">
                    {#if image}
                        {@render image()}
                    {:else if meta}
                        {#if !loaded}
                            <CogArt
                                kind="item"
                                color={node_color(node)}
                                size={160}
                            />
                        {/if}
                        <img
                            src={source}
                            alt={title}
                            class:waiting={!loaded}
                            onload={() => (loaded = true)}
                            onerror={() => {
                                loaded = false;
                                retry();
                            }}
                        />
                    {/if}
                </div>
            </div>
        {/if}
        <div class="footer" bind:this={footer}>
            <div class="price">
                {price_label(node, currency, unlocked, purchased)}
            </div>
            {#if !unlocked || node.kind === "token"}
                <HoldButton
                    {label}
                    disabled={unlocked ||
                        !available ||
                        !affordable ||
                        !!blocked ||
                        task.pending}
                    autofocus
                    onconfirm={() => act("unlock", onunlock)}
                />
                {#if !unlocked && available && !affordable}
                    <p class="skip">
                        You have {mips}
                        {currency}, so you need {price - mips} more to unlock
                        this
                    </p>
                {/if}
                {#if !unlocked && !available && requires.length}
                    <div class="prereqs">
                        Unlock one of the following first:
                        <ul>
                            {#each requires as name}
                                <li>{name}</li>
                            {/each}
                        </ul>
                    </div>
                {/if}
            {:else if node.kind === "item" && !purchased}
                <label class="customs">
                    <input type="checkbox" bind:checked={customs} />
                    I understand that Hack Club does not pay any customs or duties
                    fees that I may be charged to receive this item
                </label>
                <p class="skip">
                    This prize will be sent to the primary address set in
                    <a href="https://auth.hackclub.com" target="_blank" rel="noopener">Hack Club Auth</a>
                </p>
                <HoldButton
                    label={doing === "redeem"
                        ? pending_text.redeem
                        : `Redeem with Token (${tokens} ${tokens === 1 ? "token" : "tokens"})`}
                    disabled={tokens <= 0 ||
                        mips < 0 ||
                        !customs ||
                        !!blocked ||
                        task.pending}
                    autofocus
                    onconfirm={() => act("redeem", onpurchase)}
                />
                {#if mips < 0}
                    <p class="skip">
                        Your {currency} balance is negative, so you can't redeem
                        prizes right now
                    </p>
                {:else if tokens <= 0}
                    <p class="skip">
                        {#if node.id === STICKER && skipped}
                            You skipped the sticker, and regular Tokens can't be
                            used to redeem it
                        {:else if node.id === STICKER}
                            You need the sticker Token to redeem this. Unlock it
                            in the tree!
                        {:else}
                            You need a Token to redeem this item. Unlock them in
                            the tree!
                        {/if}
                    </p>
                {/if}
                {#if onskip}
                    <p class="skip">
                        If you don't want to actually receive the sticker, use
                        this button. The Token will still be lost for the
                        purposes of this onboarding.
                    </p>
                    <HoldButton
                        label={doing === "skip"
                            ? pending_text.skip
                            : "Skip getting sticker"}
                        disabled={tokens <= 0 || task.pending}
                        onconfirm={() => act("skip", onskip)}
                    />
                {/if}
            {/if}
            {#if blocked && actionable}
                <div class="note">
                    <AuthText text={blocked} />
                    <a href="https://auth.hackclub.com" target="_blank" rel="noopener">
                        Visit Hack Club Auth and verify your ID or add an address
                    </a>
                </div>
            {/if}
            <p class="visually-hidden" role="status">{status}</p>
            {#if task.error}
                <div class="note" role="alert">
                    <AuthText text={friendly(task.error)} />
                    {#if task.error.includes("Hack Club Auth account")}
                        <a href="https://auth.hackclub.com" target="_blank" rel="noopener">
                            Visit Hack Club Auth in order to receive prizes{node.id ===
                            STICKER
                                ? " such as this sticker"
                                : ""}
                        </a>
                    {/if}
                </div>
            {/if}
        </div>
    </div>
</Dialog>

<style>
    .content {
        display: grid;
        grid-template-columns: minmax(0, 1fr);
        grid-template-areas:
            "title"
            "description"
            "footer";
        column-gap: 1.5rem;
    }

    .content.has-image {
        grid-template-columns: minmax(0, 2fr) minmax(0, 1fr);
        grid-template-rows: auto 1fr auto;
        grid-template-areas:
            "title image"
            "description image"
            "footer image";
    }

    h2 {
        grid-area: title;
    }

    .description {
        grid-area: description;
        margin: 0.75rem 0 0;
        font-size: 0.85em;
    }

    .description p {
        margin: 0;
    }

    .description p + p {
        margin-top: 0.5rem;
    }

    .image {
        grid-area: image;
        display: grid;
        place-items: center;
        container-type: size;
    }

    .frame {
        box-sizing: border-box;
        width: min(80cqh, 100cqw);
        aspect-ratio: 1;
        position: relative;
        padding: 2px;
    }

    .frame img.waiting {
        display: none;
    }

    .visually-hidden {
        position: absolute;
        width: 1px;
        height: 1px;
        margin: 0;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }

    .frame :global(img),
    .frame :global(svg) {
        position: absolute;
        inset: 2px;
        display: block;
        width: calc(100% - 4px);
        height: calc(100% - 4px);
        object-fit: contain;
        filter: drop-shadow(2px 0 0 #4f2f1f) drop-shadow(-2px 0 0 #4f2f1f)
            drop-shadow(0 2px 0 #4f2f1f) drop-shadow(0 -2px 0 #4f2f1f);
    }

    .footer {
        grid-area: footer;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.75rem;
        margin-top: 1.5rem;
        text-align: center;
    }

    .price {
        font-weight: 700;
    }

    .note {
        font-size: 0.85em;
    }

    .skip {
        max-width: 28rem;
        margin: 0.5rem 0 0;
        font-size: 0.85em;
    }

    .prereqs {
        max-width: 28rem;
        font-size: 0.85em;
        text-align: left;
    }

    .prereqs ul {
        margin: 0.25rem 0 0;
        padding-left: 1.25rem;
    }

    .customs {
        display: flex;
        align-items: flex-start;
        gap: 0.5rem;
        max-width: 28rem;
        font-size: 0.85em;
        text-align: left;
        cursor: pointer;
    }

    .customs input {
        flex: none;
        box-sizing: border-box;
        width: 1.25em;
        height: 1.25em;
        margin: 0;
        appearance: none;
        background: #4f2f1f;
        border: 2px solid #d6ab70;
        cursor: pointer;
    }

    @media (forced-colors: active) {
        .customs input {
            appearance: auto;
        }
    }

    .customs input:checked {
        background: #d6ab70;
        box-shadow: inset 0 0 0 calc(0.25em - 2px) #4f2f1f;
    }

    .customs input:focus-visible {
        outline: 2px solid #ead7c2;
    }

    @media (max-width: 560px) {
        .content.has-image {
            grid-template-columns: minmax(0, 1fr);
            grid-template-rows: none;
            grid-template-areas:
                "title"
                "image"
                "description"
                "footer";
        }

        .image {
            container-type: normal;
            margin-top: 0.75rem;
        }

        .frame {
            width: 80%;
        }
    }
</style>
