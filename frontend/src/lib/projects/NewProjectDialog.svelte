<script lang="ts">
    import FormDialog from "$lib/ui/FormDialog.svelte";
    import Field from "$lib/ui/Field.svelte";
    import { create_project } from "$lib/projects/api";
    let {
        touring = false,
        onsaved,
        onclose,
    }: { touring?: boolean; onsaved?: () => void; onclose: () => void } =
        $props();

    let name = $state("");
</script>

<FormDialog
    title="New project"
    submit="Create"
    width="32rem"
    valid={!!name.trim()}
    onsubmit={async () => {
        await create_project(name.trim());
        onsaved?.();
    }}
    locked={touring}
    {onclose}
>
    <Field
        label="Name"
        bind:value={name}
        maxlength={100}
        focus
        data-tour="name"
    />
</FormDialog>
