<script lang="ts">
    let { text }: { text: string } = $props();

    const parts = $derived(text.split("Hack Club Auth"));
</script>

{#each parts as part, i (i)}{#if i}<a
            href="https://auth.hackclub.com"
            target="_blank"
            rel="noopener">Hack Club Auth</a
        >{/if}{part}{/each}
