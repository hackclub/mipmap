<script lang="ts">
    import { tick, type Snippet } from "svelte";
    import Dialog from "$lib/ui/Dialog.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { Task } from "$lib/load.svelte";

    let {
        title,
        submit,
        width,
        valid = true,
        cancel = false,
        locked = false,
        disabled = false,
        problem,
        onsubmit,
        onclose,
        extra,
        children,
    }: {
        title: string;
        submit: string;
        width?: string;
        valid?: boolean;
        cancel?: boolean;
        locked?: boolean;
        disabled?: boolean;
        problem?: string;
        onsubmit: () => Promise<unknown>;
        onclose: () => void;
        extra?: Snippet;
        children: Snippet;
    } = $props();

    const uid = $props.id();
    const task = new Task();
    let closed = false;
    let form: HTMLFormElement;
    let invalid = $state("");

    type Control = HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;
    const bad = (el: Control) =>
        !el.checkValidity() ||
        (el.required && "value" in el && !el.value.trim());

    function close() {
        if (task.pending) return;
        closed = true;
        onclose();
    }

    function recheck(e: Event) {
        const el = e.target as Control;
        if (el.getAttribute?.("aria-invalid") === "true" && !bad(el))
            el.removeAttribute("aria-invalid");
        if (!form.querySelector('[aria-invalid="true"]') && valid)
            invalid = "";
    }

    async function save(e: SubmitEvent) {
        e.preventDefault();
        if (task.pending) return;
        const controls = [
            ...form.querySelectorAll<Control>("input, textarea, select"),
        ].filter((el) => !el.disabled && bad(el));
        if (!valid || controls.length) {
            controls.forEach((el) => el.setAttribute("aria-invalid", "true"));
            invalid =
                problem ??
                (controls[0]?.validationMessage ||
                    "Fill in the required fields first");
            await tick();
            controls[0]?.focus();
            return;
        }
        invalid = "";
        await task.run(onsubmit, () => {
            if (!closed) {
                closed = true;
                onclose();
            }
        });
    }

    function reveal(node: HTMLElement) {
        node.scrollIntoView({ block: "nearest" });
    }
</script>

<Dialog
    labelledby="{uid}-title"
    corner="bottom-right"
    {width}
    busy={task.pending}
    onclose={locked ? undefined : close}
>
    <form bind:this={form} novalidate onsubmit={save} oninput={recheck}>
        <h2 id="{uid}-title">{title}</h2>
        {@render children()}
        {#if invalid || task.error}
            {#key invalid || task.error}
                <div class="error" role="alert" {@attach reveal}>
                    {invalid || task.error}
                </div>
            {/key}
        {/if}
        <div class="actions" data-tour="actions">
            {#if !locked}
                {@render extra?.()}
            {/if}
            {#if cancel && !locked}
                <MipmapButton
                    type="button"
                    disabled={task.pending}
                    onclick={close}>Cancel</MipmapButton
                >
            {/if}
            <MipmapButton
                type="submit"
                disabled={task.pending || disabled}
                data-tour="submit"
            >
                {submit}
            </MipmapButton>
        </div>
    </form>
</Dialog>

<style>
    form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .actions {
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        gap: 1rem;
    }

    .error {
        font-size: 0.85em;
    }
</style>
