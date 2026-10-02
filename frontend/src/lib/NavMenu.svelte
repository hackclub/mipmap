<script lang="ts">
    import { cubicOut } from "svelte/easing";

    let { items }: { items: { label: string; onselect: () => void }[] } =
        $props();

    const open_distance = 96;
    const close_distance = 192;

    let menu: HTMLElement;
    let toggle: HTMLButtonElement;
    let list = $state<HTMLUListElement>();
    let open = $state(false);
    let pointer = "";

    function grow(node: HTMLElement) {
        const { offsetWidth: width, offsetHeight: height } = node;
        const min = (node.previousElementSibling as HTMLElement).offsetWidth;
        const border = parseFloat(getComputedStyle(node).borderBottomWidth);
        return {
            duration: 200,
            easing: cubicOut,
            css: (t: number) =>
                `overflow: hidden; width: ${min + (width - min) * t}px; height: ${height * t}px; border-bottom-width: ${border * t}px`,
        };
    }

    function close() {
        if (list?.contains(document.activeElement)) toggle.focus();
        open = false;
    }

    function distance(e: PointerEvent) {
        const rect = menu.getBoundingClientRect();
        const dx = Math.max(rect.left - e.clientX, 0, e.clientX - rect.right);
        const dy = Math.max(rect.top - e.clientY, 0, e.clientY - rect.bottom);
        return Math.hypot(dx, dy);
    }
</script>

<svelte:window
    onpointermove={(e) => {
        if (e.pointerType !== "mouse") return;
        const d = distance(e);
        if (!open && d < open_distance) open = true;
        else if (open && d > close_distance) close();
    }}
    onpointerdown={(e) => {
        if (
            open &&
            e.pointerType !== "mouse" &&
            !menu.contains(e.target as Node)
        )
            close();
    }}
    onkeydown={(e) => e.key === "Escape" && close()}
/>

<nav class="menu" bind:this={menu} aria-label="Sections">
    <button
        bind:this={toggle}
        type="button"
        class="toggle"
        aria-label="Sections"
        aria-expanded={open}
        onpointerdown={(e) => (pointer = e.pointerType)}
        onclick={() => {
            if (pointer !== "mouse") open = !open;
            pointer = "";
        }}
    >
        <svg viewBox="0 0 24 24" width="1em" height="1em" aria-hidden="true">
            <path
                d="M3 5h18M3 12h18M3 19h18"
                stroke="currentColor"
                stroke-width="2.5"
            />
        </svg>
    </button>
    {#if open}
        <ul bind:this={list} transition:grow>
            {#each items as item (item.label)}
                <li>
                    <button type="button" onclick={item.onselect}
                        >{item.label}</button
                    >
                </li>
            {/each}
        </ul>
    {/if}
</nav>

<style>
    .menu {
        position: fixed;
        left: 1rem;
        bottom: 1rem;
        display: flex;
        flex-direction: column-reverse;
        overflow: hidden;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    ul {
        margin: 0;
        padding: 0;
        list-style: none;
        box-sizing: border-box;
        border-bottom: 2px solid #4f2f1f;
    }

    li + li {
        border-top: 2px solid #4f2f1f;
    }

    button {
        display: grid;
        place-items: center start;
        width: 100%;
        min-width: 3rem;
        height: 3rem;
        padding: 0 1rem;
        background: none;
        border: none;
        color: #ead7c2;
        font: inherit;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
        font-weight: 700;
        white-space: nowrap;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .toggle {
        place-items: center;
        width: 3rem;
        padding: 0;
    }

    button:hover,
    button:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }
</style>
