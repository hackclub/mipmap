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

    function parts(ends: number) {
        const seconds = Math.max(0, Math.floor((ends - now) / 1000));
        return [
            Math.floor(seconds / 86400),
            Math.floor(seconds / 3600) % 24,
            Math.floor(seconds / 60) % 60,
            seconds % 60,
        ];
    }

    function countdown(ends: number) {
        const [days, hours, minutes, seconds] = parts(ends);
        const pad = (n: number) => String(n).padStart(2, "0");
        return `${days}d ${pad(hours)}h ${pad(minutes)}m ${pad(seconds)}s`;
    }

    function spoken(ends: number) {
        const units = ["day", "hour", "minute"];
        const said = parts(ends)
            .slice(0, 3)
            .map((n, i) => (n ? `${n} ${units[i]}${n === 1 ? "" : "s"}` : ""))
            .filter(Boolean);
        return said.length ? said.join(", ") : "less than a minute";
    }
</script>

<Section title="Challenges" {bounds} zoom={3} padded>
    {#each running as challenge}
        <article>
            <h3>{challenge.name}</h3>
            <p class="countdown">
                <span aria-hidden="true" data-time={countdown(challenge.ends)}
                ></span>
                <span class="spoken">Ends in {spoken(challenge.ends)}</span>
            </p>
            <p class="description">{challenge.description}</p>
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

    .description {
        white-space: pre-line;
    }

    .fine {
        margin-top: 0.5rem;
        font-size: 0.75em;
        opacity: 0.85;
    }

    .countdown {
        margin-bottom: 0.5rem;
        color: #ead7c2;
        font-size: 0.85em;
        font-variant-numeric: tabular-nums;
    }

    .countdown [data-time]::before {
        content: "Ends in " attr(data-time);
    }

    .spoken {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }
</style>
