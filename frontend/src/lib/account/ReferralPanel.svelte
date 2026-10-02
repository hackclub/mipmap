<script lang="ts">
    import { onMount } from "svelte";
    import Field from "$lib/Field.svelte";
    import LoadError from "$lib/LoadError.svelte";
    import MipmapButton from "$lib/MipmapButton.svelte";
    import Section from "$lib/Section.svelte";
    import type { Bounds } from "$lib/PanZoom.svelte";
    import { Load } from "$lib/load.svelte";
    import { fetch_referral } from "$lib/account/api";

    let { bounds }: { bounds: Bounds } = $props();

    const labels = { idle: "Copy", copied: "Copied!", manual: "Press Ctrl+C" };
    const announcements = {
        idle: "",
        copied: "Link copied.",
        manual: "Press Ctrl+C to copy the link.",
    };

    const referral = new Load(
        async () => `${location.origin}/?ref=${(await fetch_referral()).code}`,
    );
    let copy_state = $state<keyof typeof labels>("idle");
    let row = $state<HTMLElement>();
    let timer: ReturnType<typeof setTimeout>;

    onMount(referral.run);

    async function copy() {
        clearTimeout(timer);
        try {
            await navigator.clipboard.writeText(referral.data ?? "");
            copy_state = "copied";
        } catch {
            const input = row?.querySelector("input");
            input?.focus();
            input?.select();
            copy_state = "manual";
        }
        timer = setTimeout(() => (copy_state = "idle"), 2000);
    }
</script>

<Section title="Referral" {bounds} zoom={3} padded>
    <p>
        Here's your referral link. If someone signs up with it and
        ships a project, you get Fragments!
    </p>
    {#if referral.error}
        <LoadError error={referral.error} onretry={referral.run} />
    {:else}
        <div class="row" bind:this={row}>
            <Field
                aria-label="Referral link"
                value={referral.data ?? ""}
                readonly
                onfocus={(e) => e.currentTarget.select()}
            />
            <MipmapButton
                type="button"
                disabled={!referral.data}
                onclick={copy}
            >
                {labels[copy_state]}
            </MipmapButton>
        </div>
    {/if}
    <div class="status" aria-live="polite">{announcements[copy_state]}</div>
</Section>

<style>
    .row {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        gap: 1rem;
        align-items: stretch;
    }

    .status {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }
</style>
