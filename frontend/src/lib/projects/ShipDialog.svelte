<script lang="ts">
    import Dialog from "$lib/Dialog.svelte";
    import type { Project } from "$lib/projects/types";

    let { project, onclose }: { project: Project; onclose: () => void } =
        $props();

    const uid = $props.id();
</script>

<Dialog labelledby="{uid}-title" corner="bottom-right" {onclose}>
    <h2 id="{uid}-title">Ship {project.name}</h2>
    <!-- TODO: ship dialog contents -->
</Dialog>
