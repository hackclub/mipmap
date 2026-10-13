<script lang="ts">
    import { tiers } from "$lib/projects/tiers";
    import type { Tier } from "$lib/projects/types";

    let {
        value = $bindable(),
        disabled = false,
        hint,
    }: { value: Tier | null; disabled?: boolean; hint?: string } = $props();

    const uid = $props.id();
    const options: { id: Tier | null; name: string; description?: string }[] = [
        { id: null, name: "None" },
        ...tiers,
    ];
    let open = $state(false);
    let trigger = $state<HTMLButtonElement>();
    let list = $state<HTMLElement>();
    const current = $derived(tiers.find((tier) => tier.id === value));

    const items = (el: HTMLElement) => [...el.children] as HTMLElement[];
    const index_at = (e: Event & { currentTarget: HTMLElement }) =>
        items(e.currentTarget).indexOf(
            (e.target as Element).closest('[role="option"]') as HTMLElement,
        );

    function focus_selected(el: HTMLElement) {
        const all = items(el);
        (
            all.find((item) => item.getAttribute("aria-selected") === "true") ??
            all[0]
        )?.focus();
    }

    function close() {
        open = false;
        trigger?.focus();
    }

    function choose(id: Tier | null) {
        value = id;
        close();
    }

    function navigate(e: KeyboardEvent & { currentTarget: HTMLElement }) {
        const all = items(e.currentTarget);
        const at = index_at(e);
        if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            if (at >= 0) choose(options[at].id);
            return;
        }
        const next = {
            ArrowDown: at + 1,
            ArrowUp: at - 1,
            Home: 0,
            End: all.length - 1,
        }[e.key];
        if (next === undefined) return;
        e.preventDefault();
        all[Math.max(0, Math.min(all.length - 1, next))].focus();
    }
</script>

<div
    class="select"
    role="presentation"
    onfocusout={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null))
            open = false;
    }}
    onkeydown={(e) => {
        if (open && e.key === "Escape") {
            e.stopPropagation();
            close();
        }
    }}
>
    <span class="label" id="{uid}-label">Tier</span>
    {#if hint}
        <p class="hint" id="{uid}-hint">{hint}</p>
    {/if}
    <button
        type="button"
        class="trigger"
        bind:this={trigger}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? `${uid}-list` : undefined}
        aria-labelledby="{uid}-label {uid}-value"
        aria-describedby={hint ? `${uid}-hint` : undefined}
        {disabled}
        onclick={() => (open = !open)}
        onkeydown={(e) => {
            if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
            e.preventDefault();
            if (open && list) focus_selected(list);
            else open = true;
        }}
    >
        <span id="{uid}-value">{current?.name ?? "Choose a tier"}</span>
        <span class="arrow" class:open aria-hidden="true">▾</span>
    </button>
    {#if open}
        <ul
            class="options"
            id="{uid}-list"
            role="listbox"
            aria-labelledby="{uid}-label"
            bind:this={list}
            {@attach focus_selected}
            onclick={(e) => {
                const at = index_at(e);
                if (at >= 0) choose(options[at].id);
            }}
            onkeydown={navigate}
        >
            {#each options as option (option.id)}
                <li
                    role="option"
                    tabindex="-1"
                    aria-selected={option.id === value}
                >
                    <span class="name">{option.name}</span>
                    {#if option.description}
                        <span class="description">{option.description}</span>
                    {/if}
                </li>
            {/each}
        </ul>
    {/if}
</div>

<style>
    .select {
        position: relative;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    .label {
        font-size: 0.85em;
        font-weight: 700;
    }

    .hint {
        margin: 0;
        font-size: 0.75em;
        opacity: 0.75;
    }

    .trigger {
        display: flex;
        justify-content: space-between;
        align-items: center;
        width: 100%;
        padding: 0.5rem 0.75rem;
        background: #4f2f1f;
        color: #ead7c2;
        border: none;
        border-radius: 0;
        font: inherit;
        font-size: 0.85em;
        text-align: left;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .trigger:hover:not(:disabled),
    .trigger:focus-visible,
    .options li:hover,
    .options li:focus-visible,
    .options li[aria-selected="true"] {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    .trigger:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    .arrow {
        transition: transform 0.3s;
    }

    .arrow.open {
        transform: rotate(180deg);
    }

    .options {
        position: absolute;
        top: 100%;
        left: 0;
        right: 0;
        z-index: 1;
        margin: 0;
        padding: 0;
        overflow: hidden;
        list-style: none;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    .options li {
        display: flex;
        flex-direction: column;
        padding: 0.5rem 0.75rem;
        background: #4f2f1f;
        color: #ead7c2;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .options li + li {
        border-top: 2px solid #74472a;
    }

    .name {
        font-size: 0.85em;
        font-weight: 700;
    }

    .description {
        font-size: 0.85em;
    }
</style>
