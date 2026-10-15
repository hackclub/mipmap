<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<link rel="preconnect" href="https://fonts.googleapis.com" />
	<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
	<link
		rel="stylesheet"
		href="https://fonts.googleapis.com/css2?family=Phudu:wght@300..900&family=Sansation:ital,wght@0,300;0,400;0,700;1,300;1,400;1,700&display=swap"
	/>
	<script src="/plausible-init.js"></script>
	<script async src="https://plausible.io/js/pa-aEdCG4qC19phBoTKKs0Wx.js"></script>
</svelte:head>

{@render children()}

<style>
	:global(html) {
		min-height: 100%;
		background-color: #3d3029;
		background-image: radial-gradient(#5f4838 1.5px, transparent 1.5px);
		background-size: 24px 24px;
		overflow-x: clip;
		overscroll-behavior: none;
	}

	:global(body) {
		margin: 0;
		font-family: 'Sansation', sans-serif;
		min-height: 100vh;
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
		color: #e8c58c;
		transition: color 0.3s;
	}

	:global(a:focus-visible) {
		color: #d6ab70;
	}

	@media (hover: hover) {
		:global(a:hover) {
			color: #d6ab70;
		}
	}
</style>
