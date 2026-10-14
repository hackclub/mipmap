<script lang="ts">
    import Section from "$lib/dashboard/Section.svelte";
    import type { Bounds } from "$lib/dashboard/PanZoom.svelte";

    let { bounds }: { bounds: Bounds } = $props();
</script>

<Section title="Steps" {bounds} zoom={4} padded>
    <ol>
        <li>
            <h3>Step 1</h3>
            <p>Make and ship a project</p>
        </li>
        <li>
            <h3>Step 2</h3>
            <p>Get Mips</p>
        </li>
        <li>
            <h3>Step 3</h3>
            <p>
                Use those Mips to advance your Tech Tree, unlocking both
                <span class="prizes">prizes</span> and
                <span class="tokens">Tokens</span>
            </p>
        </li>
        <li>
            <h3>Step 4</h3>
            <p>
                Use a <span class="tokens">Token</span> to redeem a
                <span class="prizes">prize</span> and get it shipped to your
                door!
            </p>
        </li>
    </ol>
</Section>

<style>
    ol {
        display: flex;
        flex-direction: column;
        gap: 1.25rem;
        margin: 0;
        padding: 0;
        list-style: none;
    }

    h3 {
        margin: 0 0 0.25rem;
        font: inherit;
        font-weight: 700;
    }

    .prizes {
        color: #f0788a;
    }

    .tokens {
        color: #c3a3e6;
    }
</style>
