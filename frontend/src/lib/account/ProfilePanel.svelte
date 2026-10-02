<script lang="ts">
    import { onMount } from "svelte";
    import Section from "$lib/Section.svelte";
    import LoadError from "$lib/LoadError.svelte";
    import type { Bounds } from "$lib/PanZoom.svelte";
    import { Load } from "$lib/load.svelte";
    import { fetch_profile } from "$lib/account/api";

    let {
        bounds,
        currency,
        tokens,
    }: { bounds: Bounds; currency: string; tokens?: number } = $props();

    const profile = new Load(fetch_profile);

    onMount(profile.run);
</script>

<Section title="Profile" {bounds} zoom={3} padded>
    <dl>
        {#if profile.data}
            <dt>First name</dt>
            <dd>{profile.data.first_name}</dd>
            <dt>{currency}</dt>
            <dd>{profile.data.fragments} {currency}</dd>
        {/if}
        {#if tokens !== undefined}
            <dt>Tokens</dt>
            <dd>{tokens} left to spend</dd>
        {/if}
    </dl>
    {#if profile.error}
        <LoadError error={profile.error} onretry={profile.run} />
    {/if}
</Section>

<style>
    dl {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        gap: 0.75rem 1.5rem;
        margin: 0;
    }

    dt {
        font-size: 0.85em;
        font-weight: 700;
    }

    dd {
        margin: 0;
        font-size: 0.85em;
    }
</style>
