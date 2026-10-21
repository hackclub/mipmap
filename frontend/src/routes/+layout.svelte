<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { colour_words } from '$lib/words';

	let { children } = $props();

	onMount(colour_words);

	$effect(() => {
		document.documentElement.classList.toggle('canvas', page.route.id === '/dashboard');
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<link
		rel="preload"
		href="/fonts/sansation-normal-400-latin.woff2"
		as="font"
		type="font/woff2"
		crossorigin="anonymous"
	/>
	<link rel="stylesheet" href="/fonts/fonts.css" />
	<script defer src="/plausible-init.js"></script>
	<script defer src="https://plausible.io/js/pa-aEdCG4qC19phBoTKKs0Wx.js"></script>
</svelte:head>

{@render children()}

<style>
	:global(html) {
		min-height: 100%;
		background-color: #3d3029;
		background-image: radial-gradient(#5f4838 1.5px, transparent 1.5px);
		background-size: 24px 24px;
	}

	:global(html.canvas) {
		overflow-x: clip;
		overscroll-behavior: none;
	}

	:global(body) {
		margin: 0;
		font-family: 'Sansation', sans-serif;
		min-height: 100vh;
		min-height: 100dvh;
	}

	:global(*) {
		scrollbar-width: thin;
		scrollbar-color: rgb(214 171 112 / 0.45) transparent;
	}

	:global(::-webkit-scrollbar) {
		width: 8px;
		height: 8px;
	}

	:global(::-webkit-scrollbar-thumb) {
		background: rgb(214 171 112 / 0.45);
		border-radius: 4px;
	}

	:global(::-webkit-scrollbar-track) {
		background: transparent;
	}

	:global(a),
	:global(a:visited) {
		color: #c8d5f3;
		transition: color 0.3s;
	}

	:global(a:focus-visible) {
		color: #ead7c2;
	}

	@media (hover: hover) {
		:global(a:hover) {
			color: #ead7c2;
		}
	}
</style>
