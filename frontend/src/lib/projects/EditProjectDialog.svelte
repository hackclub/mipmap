<script lang="ts">
    import { untrack } from "svelte";
    import Dialog from "$lib/Dialog.svelte";
    import Field from "$lib/Field.svelte";
    import MipmapButton from "$lib/MipmapButton.svelte";
    import TierSelect from "$lib/projects/TierSelect.svelte";
    import { hours, update_project } from "$lib/projects/api";
    import type {
        HackatimeProject,
        Project,
        ProjectFields,
    } from "$lib/projects/types";

    let {
        project,
        hackatime,
        onclose,
        onsave,
    }: {
        project: Project;
        hackatime: HackatimeProject[];
        onclose: () => void;
        onsave: (project: Project) => void;
    } = $props();

    const uid = $props.id();
    const { id, screenshot, ...initial } = untrack(() =>
        $state.snapshot(project),
    );
    let fields = $state<ProjectFields>(initial);
    let search = $state("");
    let pending = $state(false);
    let error = $state("");
    let closed = false;

    const results = $derived(
        hackatime.filter((entry) =>
            entry.name.toLowerCase().includes(search.trim().toLowerCase()),
        ),
    );

    function toggle(name: string, checked: boolean) {
        fields.hackatime = checked
            ? [...fields.hackatime, name]
            : fields.hackatime.filter((entry) => entry !== name);
    }

    function close() {
        closed = true;
        onclose();
    }

    async function submit(e: SubmitEvent) {
        e.preventDefault();
        if (pending) return;
        pending = true;
        error = "";
        try {
            onsave(await update_project(id, $state.snapshot(fields)));
            if (!closed) close();
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            pending = false;
        }
    }
</script>

<Dialog labelledby="{uid}-title" corner="bottom-right" onclose={close}>
    <form onsubmit={submit}>
        <h2 id="{uid}-title">Edit project</h2>
        <Field label="Name" bind:value={fields.name} />
        <Field label="Description" bind:value={fields.description} multiline />
        <div class="row">
            <Field
                label="Repo"
                type="url"
                bind:value={fields.repo}
                placeholder="https://"
            />
            <Field
                label="Demo"
                type="url"
                bind:value={fields.demo}
                placeholder="https://"
            />
        </div>
        <TierSelect bind:value={fields.tier} />

        <section class="hackatime" aria-labelledby="{uid}-hackatime">
            <h3 id="{uid}-hackatime">Hackatime projects</h3>
            <Field
                type="search"
                aria-label="Search Hackatime projects"
                placeholder="Search"
                bind:value={search}
                onkeydown={(e) => e.key === "Enter" && e.preventDefault()}
            />
            <ul>
                {#each results as entry (entry.name)}
                    <li>
                        <label>
                            <span class="name">{entry.name}</span>
                            <span class="hours">{hours(entry.seconds)}</span>
                            <input
                                type="checkbox"
                                checked={fields.hackatime.includes(entry.name)}
                                onchange={(e) =>
                                    toggle(entry.name, e.currentTarget.checked)}
                            />
                        </label>
                    </li>
                {:else}
                    <li class="empty">No matching projects.</li>
                {/each}
            </ul>
        </section>

        {#if error}
            <div class="error">{error}</div>
        {/if}
        <div class="actions">
            <MipmapButton type="button" onclick={close}>Cancel</MipmapButton>
            <MipmapButton type="submit" disabled={pending}>Save</MipmapButton>
        </div>
    </form>
</Dialog>

<style>
    form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .row {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 1rem;
    }

    .hackatime {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    h3 {
        margin: 0;
        font: inherit;
        font-size: 0.85em;
        font-weight: 700;
    }

    ul {
        max-height: 16rem;
        margin: 0;
        padding: 0;
        overflow-y: auto;
        list-style: none;
        border: 2px solid #4f2f1f;
    }

    li + li {
        border-top: 2px solid #4f2f1f;
    }

    label,
    .empty {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 0.5rem 0.75rem;
        font-size: 0.85em;
    }

    label {
        cursor: pointer;
    }

    .name {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    input[type="checkbox"] {
        flex: none;
        width: 1.25em;
        height: 1.25em;
        margin: 0;
        appearance: none;
        background: #4f2f1f;
        cursor: pointer;
    }

    input[type="checkbox"]:checked {
        background: #d6ab70;
        box-shadow: inset 0 0 0 0.25em #4f2f1f;
    }

    input[type="checkbox"]:focus-visible {
        outline: 2px solid #ead7c2;
    }

    .actions {
        display: flex;
        justify-content: flex-end;
        gap: 1rem;
    }

    .error {
        font-size: 0.85em;
    }

    @media (max-width: 560px) {
        .row {
            grid-template-columns: minmax(0, 1fr);
        }
    }
</style>
