<script lang="ts">
    import { tick, type Snippet } from "svelte";
    import Dialog, { refocus } from "$lib/ui/Dialog.svelte";
    import { Task } from "$lib/load.svelte";
    import { price_label } from "$lib/tree/style";
    import HoldButton from "$lib/tree/HoldButton.svelte";
    import { STICKER } from "$lib/tree/tutorial";
    import type { ItemMeta, TreeNode } from "$lib/tree/types";

    let {
        node,
        meta,
        currency,
        unlocked,
        purchased,
        tokens,
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
    const actionable = $derived(
        !unlocked || (node.kind === "item" && !purchased),
    );
    const label = $derived(
        unlocked ? "Unlocked!" : available ? "Unlock" : "Prerequisite Missing",
    );

    const task = new Task();

    let customs = $state(false);

    let footer: HTMLDivElement;

    async function act(action: () => Promise<void>) {
        await task.run(action);
        await tick();
        refocus(footer, true);
    }
</script>

<Dialog labelledby="{uid}-title" {onclose}>
    <div class="content" class:has-image={has_image}>
        <h2 id="{uid}-title">{title}</h2>
        {#if description}
            <p class="description">{description}</p>
        {/if}
        {#if has_image}
            <div class="image">
                <div class="frame">
                    {#if image}
                        {@render image()}
                    {:else if meta}
                        <img
                            src="/dashboard/item-images/{node.id}"
                            alt=""
                            onerror={(e) => {
                                const img = e.currentTarget as HTMLImageElement;
                                const fallback = new URL(meta.image, location.href).href;
                                if (img.src !== fallback) img.src = fallback;
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
                    disabled={unlocked || !available || !!blocked || task.pending}
                    autofocus
                    onconfirm={() => act(onunlock)}
                />
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
                    This prize will ship to the primary address set in Hack Club
                    Auth
                </p>
                <HoldButton
                    label="Purchase with Token ({tokens})"
                    disabled={tokens === 0 || !customs || !!blocked || task.pending}
                    autofocus
                    onconfirm={() => act(onpurchase)}
                />
                {#if tokens === 0}
                    <p class="skip">You need a Token to redeem this item</p>
                {/if}
                {#if onskip}
                    <p class="skip">
                        If you don't want to actually receive the sticker, use
                        this button. The Token will still be lost for the
                        purposes of this tutorial.
                    </p>
                    <HoldButton
                        label="Skip"
                        disabled={tokens === 0 || task.pending}
                        onconfirm={() => act(onskip)}
                    />
                {/if}
            {/if}
            {#if blocked && actionable}
                <div class="note">
                    {blocked}
                    <a href="https://auth.hackclub.com">Visit Hack Club Auth</a>
                </div>
            {/if}
            {#if task.error}
                <div class="note" role="alert">
                    {task.error}
                    {#if task.error.includes("Hack Club account")}
                        <a href="https://auth.hackclub.com">
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
        width: 1.25em;
        height: 1.25em;
        margin: 0;
        appearance: none;
        background: #4f2f1f;
        cursor: pointer;
    }

    .customs input:checked {
        background: #d6ab70;
        box-shadow: inset 0 0 0 0.25em #4f2f1f;
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
