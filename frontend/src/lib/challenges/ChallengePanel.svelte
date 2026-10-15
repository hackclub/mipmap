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

    const words =
        /\b(mipmap|mips?|tech trees?|tiers?(?: (?:quebec|romeo|sierra))?|tokens?|prizes?|snapshots?|hackatime)\b/gi;

    function kind(word: string) {
        const lower = word.toLowerCase();
        if (lower === "mipmap") return "mipmap";
        if (lower.startsWith("mip")) return "mip";
        if (lower.startsWith("tech")) return "tech-tree";
        if (lower.startsWith("tier")) return "tier";
        if (lower.startsWith("token")) return "token";
        if (lower.startsWith("prize")) return "prize";
        if (lower.startsWith("snapshot")) return "snapshot";
        return "hackatime";
    }

    function colour(text: string) {
        return text
            .split(words)
            .map((part, i) => ({ text: part, kind: i % 2 ? kind(part) : null }));
    }
</script>

{#snippet coloured(text: string)}
    {#each colour(text) as part}{#if part.kind}<span class={part.kind}
                >{part.text}</span
            >{:else}{part.text}{/if}{/each}
{/snippet}

<Section title="Limited Time Challenge" {bounds} zoom={3} padded>
    {#each running as challenge}
        <article>
            <h3>{@render coloured(challenge.name)}</h3>
            <p class="countdown">Ends in {countdown(challenge.ends)}</p>
            <p>{@render coloured(challenge.description)}</p>
            <p class="fine">
                Project must be shipped before the Challenge deadline. Reviewers
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

    article .mip {
        color: #ff9a3d;
    }

    article .mipmap {
        color: #8fcf6b;
    }

    article .tier {
        color: #ffe066;
    }

    article .tech-tree {
        color: #7cc4f0;
    }

    article .token {
        color: #c3a3e6;
    }

    article .prize {
        color: #f0788a;
    }

    article .snapshot {
        color: #e8c58c;
    }

    article .hackatime {
        color: #d6ab70;
    }
</style>
