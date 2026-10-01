<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	let { children, ...rest }: { children?: Snippet } & HTMLButtonAttributes = $props();
</script>

<button {...rest}>
	{@render children?.()}
</button>

<style>
	button {
		background-color: #4f2f1f;
		color: #ead7c2;
		border: none;
		border-radius: 0;
		padding: 0.75rem 1.5rem;
		font: inherit;
		font-size: clamp(1.25rem, 2vw, 1.75rem);
		font-weight: 700;
		cursor: pointer;
		transition: background-color 0.3s, color 0.3s;
	}

	button:hover,
	button:focus-visible {
		background-color: #ead7c2;
		color: #4f2f1f;
	}
</style>
