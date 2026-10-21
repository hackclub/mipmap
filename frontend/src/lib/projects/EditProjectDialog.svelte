<script lang="ts">
    import { untrack } from "svelte";
    import FormDialog from "$lib/ui/FormDialog.svelte";
    import Field from "$lib/ui/Field.svelte";
    import TierSelect from "$lib/projects/TierSelect.svelte";
    import HoldButton from "$lib/tree/HoldButton.svelte";
    import {
        delete_project,
        update_project,
        uploading,
    } from "$lib/projects/api";
    import { hours } from "$lib/format";
    import type {
        HackatimeProject,
        Project,
        ProjectFields,
    } from "$lib/projects/types";

    let {
        project,
        hackatime,
        linked,
        touring = false,
        onsaved,
        onclose,
    }: {
        project: Project;
        hackatime: HackatimeProject[];
        linked: boolean;
        touring?: boolean;
        onsaved?: () => void;
        onclose: () => void;
    } = $props();

    const uid = $props.id();
    const { id, screenshot, ...initial } = untrack(() =>
        $state.snapshot(project),
    );
    const reviewing = untrack(() => project.status === "shipped");
    const locked = untrack(() => project.locked);
    const saved_links = { repo: initial.repo, demo: initial.demo };
    let fields = $state<ProjectFields>(initial);
    let search = $state("");
    let confirming = $state(false);

    const results = $derived(
        [
            ...hackatime,
            ...fields.hackatime
                .filter((name) => !hackatime.some((entry) => entry.name === name))
                .map((name) => ({ name, seconds: 0 })),
        ].filter((entry) =>
            entry.name.toLowerCase().includes(search.trim().toLowerCase()),
        ),
    );

    function link(label: string, value: string) {
        const trimmed = value.trim();
        if (!trimmed) return "";
        const full = /^[a-z][a-z\d+.-]*:\/\//i.test(trimmed)
            ? trimmed
            : `https://${trimmed}`;
        try {
            if (/\s/.test(full)) throw new Error();
            new URL(full);
        } catch {
            throw new Error(
                `The ${label} link doesn't look right, try something like https://github.com/you/project`,
            );
        }
        return full;
    }

    function toggle(name: string, checked: boolean) {
        fields.hackatime = checked
            ? [...fields.hackatime, name]
            : fields.hackatime.filter((entry) => entry !== name);
    }
</script>

{#snippet remove()}
    {#if !locked}
        <HoldButton
            label="Delete"
            duration={2000}
            onconfirm={() => (confirming = true)}
        />
    {/if}
{/snippet}

<FormDialog
    title="Edit project"
    submit="Save"
    cancel
    extra={remove}
    locked={touring}
    disabled={uploading.has(id)}
    onsubmit={async () => {
        const repo =
            fields.repo === saved_links.repo
                ? fields.repo
                : link("repo", fields.repo);
        const demo =
            fields.demo === saved_links.demo
                ? fields.demo
                : link("demo", fields.demo);
        fields.repo = repo;
        fields.demo = demo;
        await update_project(id, { ...$state.snapshot(fields), repo, demo });
        onsaved?.();
    }}
    {onclose}
>
    {#if reviewing}
        <p class="note">This project is in review, so only its name can change.</p>
    {/if}
    {#if uploading.has(id)}
        <p class="note" role="status">
            You can save once the screenshot finishes uploading
        </p>
    {/if}
    <Field
        label="Name"
        hint="What your project is called"
        bind:value={fields.name}
        maxlength={100}
        required
    />
    <Field
        label="Description"
        hint="A short summary of what you're making and how it works"
        bind:value={fields.description}
        multiline
        maxlength={5000}
        disabled={reviewing}
    />
    <div class="row" data-tour="links">
        <Field
            label="Repo"
            hint="Your public code repository on GitHub, GitLab or Codeberg"
            type="text"
            inputmode="url"
            autocapitalize="off"
            autocomplete="url"
            spellcheck="false"
            bind:value={fields.repo}
            placeholder="https://"
            maxlength={500}
            disabled={reviewing || locked}
        />
        <Field
            label="Demo"
            hint="A public link where people can try your project, like a release page or a hosted build"
            type="text"
            inputmode="url"
            autocapitalize="off"
            autocomplete="url"
            spellcheck="false"
            bind:value={fields.demo}
            placeholder="https://"
            maxlength={500}
            disabled={reviewing}
        />
    </div>
    <div data-tour="tier">
        <TierSelect
            bind:value={fields.tier}
            disabled={reviewing || locked}
            hint="How deep your project goes. Higher tiers earn Mips faster"
        />
    </div>

    <section
        class="hackatime"
        aria-labelledby="{uid}-hackatime"
        aria-describedby="{uid}-hackatime-hint"
        data-tour="hackatime"
    >
        <h3 id="{uid}-hackatime">Hackatime projects</h3>
        <p class="hint" id="{uid}-hackatime-hint">
            The Hackatime projects that track your time on this. Each Hackatime
            project can only be linked to one Mipmap project
        </p>
        <Field
            type="search"
            aria-label="Search Hackatime projects"
            placeholder="Search"
            bind:value={search}
            disabled={reviewing}
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
                            disabled={reviewing}
                            onchange={(e) =>
                                toggle(entry.name, e.currentTarget.checked)}
                        />
                    </label>
                </li>
            {:else}
                <li class="empty">
                    {linked ? "No matching projects" : "Link Hackatime to select projects"}
                </li>
            {/each}
        </ul>
        {#if fields.hackatime.length}
            <p class="note">
                Each Hackatime entry can only be used for one Mipmap project!
            </p>
        {/if}
    </section>
</FormDialog>

{#if confirming}
    <FormDialog
        title="Delete {project.name}?"
        submit="Delete"
        cancel
        width="32rem"
        onsubmit={async () => {
            await delete_project(id);
            onclose();
        }}
        onclose={() => (confirming = false)}
    >
        <p class="note">This can't be undone</p>
    </FormDialog>
{/if}

<style>
    .note {
        margin: 0;
        font-size: 0.85em;
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

    .hint {
        margin: 0;
        font-size: 0.75em;
        opacity: 0.85;
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

    label:has(:disabled) {
        cursor: not-allowed;
        opacity: 0.6;
    }

    .name {
        flex: 1;
        min-width: 0;
        overflow-wrap: anywhere;
    }

    input[type="checkbox"] {
        flex: none;
        box-sizing: border-box;
        width: 1.25em;
        height: 1.25em;
        margin: 0;
        appearance: none;
        background: #4f2f1f;
        border: 1px solid #d6ab70;
        cursor: pointer;
    }

    input[type="checkbox"]:disabled {
        cursor: not-allowed;
    }

    input[type="checkbox"]:checked {
        background: #d6ab70;
        box-shadow: inset 0 0 0 0.25em #4f2f1f;
    }

    input[type="checkbox"]:focus-visible {
        outline: 2px solid #ead7c2;
    }

    @media (forced-colors: active) {
        input[type="checkbox"] {
            appearance: auto;
        }
    }

    @media (max-width: 560px) {
        .row {
            grid-template-columns: minmax(0, 1fr);
        }
    }
</style>
