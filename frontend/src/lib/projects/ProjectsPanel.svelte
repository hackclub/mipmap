<script lang="ts" module>
    export type ProjectDialog =
        | { kind: "new" }
        | { kind: "edit" | "ship"; id: number }
        | null;
</script>

<script lang="ts">
    import Section from "$lib/dashboard/Section.svelte";
    import type { Bounds } from "$lib/dashboard/PanZoom.svelte";
    import EditProjectDialog from "$lib/projects/EditProjectDialog.svelte";
    import NewProjectDialog from "$lib/projects/NewProjectDialog.svelte";
    import ProjectTile from "$lib/projects/ProjectTile.svelte";
    import ShipDialog from "$lib/projects/ShipDialog.svelte";
    import type { HackatimeProject, Project } from "$lib/projects/types";

    let {
        bounds,
        projects,
        hackatime,
        dialog = $bindable(null),
        touring = false,
        onsaved,
    }: {
        bounds: Bounds;
        projects: Project[];
        hackatime: HackatimeProject[];
        dialog?: ProjectDialog;
        touring?: boolean;
        onsaved?: (kind: "new" | "edit") => void;
    } = $props();

    const seconds = $derived(
        new Map(hackatime.map((entry) => [entry.name, entry.seconds])),
    );
    const active = $derived.by(() => {
        const current = dialog;
        return current && current.kind !== "new"
            ? projects.find((project) => project.id === current.id)
            : undefined;
    });

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
            data-tour="new"
            onclick={() => (dialog = { kind: "new" })}>+</button
        >
    {/snippet}
    <div class="tiles">
        {#each projects as project (project.id)}
            <ProjectTile
                {project}
                seconds={tracked(project)}
                onedit={() => (dialog = { kind: "edit", id: project.id })}
                onship={() => (dialog = { kind: "ship", id: project.id })}
            />
        {:else}
            <p class="empty">No projects yet</p>
        {/each}
    </div>
</Section>

{#if dialog?.kind === "new"}
    <NewProjectDialog
        {touring}
        onsaved={() => onsaved?.("new")}
        onclose={() => (dialog = null)}
    />
{:else if dialog?.kind === "edit" && active}
    <EditProjectDialog
        project={active}
        {hackatime}
        {touring}
        onsaved={() => onsaved?.("edit")}
        onclose={() => (dialog = null)}
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
