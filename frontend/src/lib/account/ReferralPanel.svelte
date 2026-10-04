<script lang="ts">
    import Field from "$lib/ui/Field.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import Section from "$lib/dashboard/Section.svelte";
    import type { Bounds } from "$lib/dashboard/PanZoom.svelte";

    let { bounds, code }: { bounds: Bounds; code: string } = $props();

    const labels = { idle: "Copy", copied: "Copied!", manual: "Press Ctrl+C" };
    const announcements = {
        idle: "",
        copied: "Link copied.",
        manual: "Press Ctrl+C to copy the link.",
    };

    const link = $derived(`${location.origin}/?ref=${code}`);
    let copy_state = $state<keyof typeof labels>("idle");
    let row = $state<HTMLElement>();
    let timer: ReturnType<typeof setTimeout>;

    async function copy() {
        clearTimeout(timer);
        try {
            await navigator.clipboard.writeText(link);
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
        gets a project approved, you get 50 Mips!
    </p>
    <div class="row" bind:this={row}>
        <Field
            aria-label="Referral link"
            value={link}
            readonly
            onfocus={(e) => e.currentTarget.select()}
        />
        <MipmapButton type="button" onclick={copy}>
            {labels[copy_state]}
        </MipmapButton>
    </div>
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
