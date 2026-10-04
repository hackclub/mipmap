<script lang="ts">
    import Dialog from "$lib/ui/Dialog.svelte";

    let {
        title,
        ontour,
        onclose,
    }: { title: string; ontour: () => void; onclose: () => void } = $props();

    const uid = $props.id();
</script>

<Dialog labelledby="{uid}-title" width="44rem" {onclose}>
    <h2 id="{uid}-title">{title}</h2>
    <div class="guides">
        <button type="button" class="guide" data-autofocus onclick={ontour}>
            <span class="icon">
                <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path
                        d="M20 23.003L19.497 23C12.59 22.958 1 22.514 1 20c0-1.09 1.756-1.416 4.187-1.866 1.193-.22 3.677-.682 3.814-1.138-.116-.368-2.117-.889-4.523-.997L4 15.979V15l1 .026c2.06.128 5 .56 5 1.974 0 1.259-2.146 1.656-4.632 2.117-1.18.219-3.153.584-3.382.94.309.97 8.324 1.887 17.515 1.943H20zM9 5.133C9 7.412 6.814 10.5 5 14c-1.814-3.5-4-6.587-4-8.868A4.04 4.04 0 0 1 5 1a4.04 4.04 0 0 1 4 4.132zm-3.435 5.752C6.817 8.66 8 6.562 8 5.132A3.035 3.035 0 0 0 5 2a3.035 3.035 0 0 0-3 3.132c0 1.43 1.183 3.53 2.435 5.753.186.332.376.668.565 1.01.19-.342.379-.678.565-1.01zM7 5a2 2 0 1 1-2-2 2 2 0 0 1 2 2zM6 5a1 1 0 1 0-1 1 1 1 0 0 0 1-1zm17 7.132c0 2.281-2.186 5.368-4 8.868-1.814-3.5-4-6.587-4-8.868a4.002 4.002 0 1 1 8 0zm-3.435 5.753C20.817 15.66 22 13.562 22 12.132a3.003 3.003 0 1 0-6 0c0 1.43 1.183 3.53 2.435 5.753.186.332.376.668.565 1.01.19-.342.379-.678.565-1.01zM21 12a2 2 0 1 1-2-2 2 2 0 0 1 2 2zm-1 0a1 1 0 1 0-1 1 1 1 0 0 0 1-1z"
                    />
                </svg>
            </span>
            <span class="caption">Take a tour around!</span>
        </button>
        <a class="guide" href="/docs" target="_blank" rel="noopener">
            <span class="icon">
                <svg viewBox="0 0 16 16" aria-hidden="true">
                    <path
                        fill-rule="evenodd"
                        d="M8.316 2.07a.75.75 0 00-.632 0l-7 3.25a.75.75 0 000 1.36l1.434.666A.746.746 0 002 7.75V11a.75.75 0 00.158.46L2.75 11l-.592.46.001.002.001.001.003.004.008.01a1.882 1.882 0 00.103.12c.068.076.165.178.292.299.254.24.63.555 1.132.866C4.706 13.388 6.217 14 8.25 14c2.037 0 3.44-.615 4.345-1.266a5.32 5.32 0 00.977-.902 3.916 3.916 0 00.322-.448l.007-.012.003-.004v-.002h.001c0-.001 0-.002-.655-.366l.655.365A.754.754 0 0014 11V7.75a.747.747 0 00-.118-.404l1.434-.666a.75.75 0 000-1.36l-7-3.25zM12.5 7.988L8.316 9.93a.75.75 0 01-.632 0L3.5 7.988v2.723a5.585 5.585 0 00.99.776c.804.5 2.043 1.013 3.76 1.013 1.713 0 2.81-.51 3.468-.984a3.812 3.812 0 00.782-.745V7.988zM8 8.423L2.781 6 8 3.577 13.219 6 8 8.423z"
                        clip-rule="evenodd"
                    />
                </svg>
            </span>
            <span class="caption">Learn about all this stuff!</span>
        </a>
    </div>
</Dialog>

<style>
    .guides {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
        gap: 1.5rem;
        margin-top: 1.5rem;
    }

    .guide {
        display: flex;
        flex-direction: column;
        overflow: hidden;
        padding: 0;
        background: none;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font: inherit;
        text-align: center;
        text-decoration: none;
        cursor: pointer;
    }

    .icon {
        display: grid;
        place-items: center;
        aspect-ratio: 1;
        background: #5f4838;
    }

    .icon svg {
        width: 60%;
        height: 60%;
        fill: currentColor;
    }

    .caption {
        flex: 1;
        display: grid;
        place-items: center;
        padding: 0.75rem 1rem;
        background: #4f2f1f;
        font-weight: 700;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .guide:hover .caption,
    .guide:focus-visible .caption {
        background: #ead7c2;
        color: #4f2f1f;
    }
</style>
