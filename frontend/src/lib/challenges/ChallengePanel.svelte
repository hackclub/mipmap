<script lang="ts">
    import Section from "$lib/dashboard/Section.svelte";
    import type { Bounds } from "$lib/dashboard/PanZoom.svelte";
    import { challenges } from "$lib/challenges/challenges";

    let { bounds }: { bounds: Bounds } = $props();

    let now = $state(Date.now());
    $effect(() => {
        const timer = setInterval(() => (now = Date.now()), 1000);
        return () => clearInterval(timer);
    });

    const running = $derived(
        challenges.filter((c) => c.starts <= now && now < c.ends),
    );

    function countdown(ends: number) {
        const seconds = Math.max(0, Math.floor((ends - now) / 1000));
        return [
            Math.floor(seconds / 86400),
            Math.floor(seconds / 3600) % 24,
            Math.floor(seconds / 60) % 60,
            seconds % 60,
        ]
            .map((part) => String(part).padStart(2, "0"))
            .join(":");
    }
</script>

<Section title="Challenges" {bounds} zoom={3} padded>
    {#each running as challenge}
        <article>
            <h3>{challenge.name}</h3>
            <p class="countdown">Ends in {countdown(challenge.ends)}</p>
            <p>{challenge.description}</p>
            <p class="fine">
                Project must be shipped before the Challenge's countdown deadline. Reviewers
                have discretion on whether or not this project meets the criteria
            </p>
        </article>
    {:else}
        <p>Nothing's running at the moment, check back soon!</p>
    {/each}
</Section>

<style>
    article + article {
        margin-top: 1rem;
        padding-top: 1rem;
        border-top: 2px solid #4f2f1f;
    }

    h3,
    p {
        margin: 0;
    }

    h3 {
        font: inherit;
        font-weight: 700;
    }

    .fine {
        margin-top: 0.5rem;
        font-size: 0.75em;
        opacity: 0.75;
    }

    .countdown {
        margin-bottom: 0.5rem;
        color: #d6ab70;
        font-size: 0.85em;
        font-variant-numeric: tabular-nums;
    }
</style>
