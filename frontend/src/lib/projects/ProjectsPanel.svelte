<script lang="ts">
    import { onMount } from "svelte";
    import LoadError from "$lib/LoadError.svelte";
    import Section from "$lib/Section.svelte";
    import type { Bounds } from "$lib/PanZoom.svelte";
    import { Load } from "$lib/load.svelte";
    import EditProjectDialog from "$lib/projects/EditProjectDialog.svelte";
    import NewProjectDialog from "$lib/projects/NewProjectDialog.svelte";
    import ProjectTile from "$lib/projects/ProjectTile.svelte";
    import ShipDialog from "$lib/projects/ShipDialog.svelte";
    import {
        fetch_hackatime_projects,
        fetch_projects,
    } from "$lib/projects/api";
    import type { Project } from "$lib/projects/types";

    let { bounds }: { bounds: Bounds } = $props();

    const projects = new Load(fetch_projects);
    const hackatime = new Load(fetch_hackatime_projects);
    let dialog = $state<
        { kind: "new" } | { kind: "edit" | "ship"; id: number } | null
    >(null);

    const seconds = $derived(
        new Map(
            (hackatime.data ?? []).map((entry) => [entry.name, entry.seconds]),
        ),
    );
    const active = $derived.by(() => {
        const current = dialog;
        return current && current.kind !== "new"
            ? projects.data?.find((project) => project.id === current.id)
            : undefined;
    });

    onMount(() => {
        projects.run();
        hackatime.run();
    });

    function replace(updated: Project) {
        projects.data = projects.data?.map((project) =>
            project.id === updated.id ? updated : project,
        );
    }

    const tracked = (project: Project) =>
        project.hackatime.reduce(
            (total, name) => total + (seconds.get(name) ?? 0),
            0,
        );
</script>

<Section title="Projects" {bounds} zoom={3}>
    {#snippet actions()}
        <button
            type="button"
            aria-label="New project"
            onclick={() => (dialog = { kind: "new" })}>+</button
        >
    {/snippet}
    <div class="tiles">
        {#if projects.error}
            <LoadError error={projects.error} onretry={projects.run} />
        {:else if projects.data}
            {#each projects.data as project (project.id)}
                <ProjectTile
                    {project}
                    seconds={tracked(project)}
                    onedit={() => (dialog = { kind: "edit", id: project.id })}
                    onship={() => (dialog = { kind: "ship", id: project.id })}
                    onupdate={replace}
                />
            {:else}
                <p class="empty">No projects yet.</p>
            {/each}
        {/if}
    </div>
</Section>

{#if dialog?.kind === "new"}
    <NewProjectDialog
        onclose={() => (dialog = null)}
        oncreate={(project) =>
            (projects.data = [...(projects.data ?? []), project])}
    />
{:else if dialog?.kind === "edit" && active}
    <EditProjectDialog
        project={active}
        hackatime={hackatime.data ?? []}
        onclose={() => (dialog = null)}
        onsave={replace}
    />
{:else if dialog?.kind === "ship" && active}
    <ShipDialog project={active} onclose={() => (dialog = null)} />
{/if}

<style>
    .tiles {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(42rem, 1fr));
        align-items: start;
        gap: 1.5rem;
        padding: 1.5rem;
    }

    .empty {
        margin: 0;
        font-size: 0.85em;
    }
</style>
