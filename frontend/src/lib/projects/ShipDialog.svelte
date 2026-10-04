<script lang="ts">
    import { tick } from "svelte";
    import { message } from "$lib/api";
    import { dashboard } from "$lib/dashboard/state.svelte";
    import Cog from "$lib/ui/Cog.svelte";
    import Dialog, { refocus } from "$lib/ui/Dialog.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { ship_project } from "$lib/projects/api";
    import type { CheckEvent, Project } from "$lib/projects/types";

    let { project, onclose }: { project: Project; onclose: () => void } =
        $props();

    const uid = $props.id();

    let checks = $state<CheckEvent[]>([]);
    let started = $state(false);
    let running = $state(false);
    let shipped = $state(false);
    let error = $state("");
    const submit_failed = $derived(
        checks.some(
            (entry) => entry.check === "submit" && entry.status === "failed",
        ),
    );

    let actions: HTMLDivElement;

    function apply(event: CheckEvent) {
        const index = checks.findIndex((entry) => entry.check === event.check);
        if (index < 0) checks.push(event);
        else checks[index] = { ...checks[index], ...event };
    }

    async function ship() {
        if (running) return;
        started = true;
        running = true;
        error = "";
        checks = [];
        try {
            await ship_project(project.id, apply);
            shipped = true;
        } catch (e) {
            error = message(e);
            dashboard.run();
        } finally {
            running = false;
        }
        await tick();
        refocus(actions);
    }
</script>

<Dialog
    labelledby="{uid}-title"
    corner="bottom-right"
    onclose={running ? undefined : onclose}
>
    <h2 id="{uid}-title">Ship {project.name}</h2>
    {#if !started}
        <p class="note">
            Shipping sends your project for review
        </p>
    {/if}
    {#if checks.length}
        <ol>
            {#each checks as entry (entry.check)}
                <li class={entry.status}>
                    <span class="marker" role="img" aria-label={entry.status}>
                        {#if entry.status === "running"}
                            <span class="spin"><Cog color="#d6ab70" /></span>
                        {:else if entry.status === "passed"}
                            <svg viewBox="0 0 16 16" aria-hidden="true">
                                <path d="M3 8.5 6.5 12 13 4.5" />
                            </svg>
                        {:else if entry.status === "unverified"}
                            <svg viewBox="0 0 16 16" aria-hidden="true">
                                <path d="M5.8 6a2.3 2.3 0 1 1 3.4 2c-.8.5-1.2.9-1.2 1.8M8 12.6v.2" />
                            </svg>
                        {:else}
                            <svg viewBox="0 0 16 16" aria-hidden="true">
                                <path d="M4 4 12 12M12 4 4 12" />
                            </svg>
                        {/if}
                    </span>
                    <span class="label">{entry.label ?? entry.check}</span>
                    {#if (entry.status === "failed" || entry.status === "unverified") && entry.message && entry.check !== "submit"}
                        <span class="note detail">{entry.message}</span>
                    {/if}
                </li>
            {/each}
        </ol>
    {/if}
    {#if shipped}
        <p class="note">Shipped! It's waiting for review</p>
    {/if}
    {#if error}
        <p class="note" role="alert">
            {#if submit_failed}
                Submission failed. Try again once or twice, then bug
                <a href="https://hackclub.enterprise.slack.com/team/U08LEHUPSCR"
                    >me</a
                > about it
            {:else}
                {error}
            {/if}
        </p>
    {/if}
    <div class="actions" bind:this={actions}>
        {#if shipped}
            <MipmapButton type="button" onclick={onclose}>Close</MipmapButton>
        {:else if error}
            <MipmapButton type="button" onclick={ship}>Try again</MipmapButton>
        {:else}
            <MipmapButton
                type="button"
                data-autofocus
                disabled={running}
                onclick={ship}
            >
                Ship
            </MipmapButton>
        {/if}
    </div>
</Dialog>

<style>
    .note {
        margin: 0.75rem 0 0;
        font-size: 0.85em;
    }

    ol {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        margin: 1rem 0 0;
        padding: 0;
        list-style: none;
    }

    li {
        display: grid;
        grid-template-columns: 1em minmax(0, 1fr);
        align-items: center;
        column-gap: 0.75rem;
    }

    li.failed .label {
        font-weight: 700;
    }

    .marker {
        display: block;
        width: 1em;
        height: 1em;
        line-height: 0;
    }

    .marker :global(svg),
    .spin {
        display: block;
        width: 100%;
        height: 100%;
    }

    .marker svg {
        fill: none;
        stroke: currentColor;
        stroke-width: 2;
        stroke-linecap: round;
        stroke-linejoin: round;
    }

    .passed .marker,
    .unverified .marker {
        color: #d6ab70;
    }

    .spin {
        animation: spin 1.5s linear infinite;
    }

    .detail {
        grid-column: 2;
        margin: 0;
    }

    .actions {
        display: flex;
        justify-content: flex-end;
        margin-top: 1.5rem;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .spin {
            animation: none;
        }
    }
</style>
