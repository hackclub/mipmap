<script lang="ts">
    import { tick, type Snippet } from "svelte";
    import Dialog, { refocus } from "$lib/ui/Dialog.svelte";
    import { Task } from "$lib/load.svelte";
    import { price_label } from "$lib/tree/style";
    import HoldButton from "$lib/tree/HoldButton.svelte";
    import type { ItemMeta, TreeNode } from "$lib/tree/types";

    let {
        node,
        meta,
        currency,
        unlocked,
        purchased,
        tokens,
        available,
        blocked,
        image,
        onclose,
        onunlock,
        onpurchase,
    }: {
        node: TreeNode;
        meta?: ItemMeta;
        currency: string;
        unlocked: boolean;
        purchased: boolean;
        tokens: number;
        available: boolean;
        blocked: string | null;
        image?: Snippet;
        onclose: () => void;
        onunlock: () => Promise<void>;
        onpurchase: () => Promise<void>;
    } = $props();

    const uid = $props.id();
    const token_description =
        "Gives you one token which can be used to get yourself one prize.";

    const title = $derived(meta?.title ?? node.name);
    const description = $derived(
        node.kind === "token" ? token_description : meta?.description,
    );
    const has_image = $derived(Boolean(image || meta?.image));
    const actionable = $derived(
        !unlocked || (node.kind === "item" && !purchased),
    );
    const label = $derived(
        unlocked ? "Unlocked!" : available ? "Unlock" : "Prerequisite Missing",
    );

    const task = new Task();

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
                        <img src={meta.image} alt="" />
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
            {:else if node.kind === "item" && !purchased}
                <HoldButton
                    label="Purchase with token ({tokens})"
                    disabled={tokens === 0 || !!blocked || task.pending}
                    autofocus
                    onconfirm={() => act(onpurchase)}
                />
            {/if}
            {#if blocked && actionable}
                <div class="note">
                    {blocked}
                    <a href="https://auth.hackclub.com">Visit Hack Club Auth</a>
                </div>
            {/if}
            {#if task.error}
                <div class="note" role="alert">{task.error}</div>
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
        display: grid;
        place-items: center;
        padding: 2px;
    }

    .frame :global(img),
    .frame :global(svg) {
        display: block;
        width: 100%;
        height: 100%;
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
