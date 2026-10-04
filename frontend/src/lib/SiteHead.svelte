<script lang="ts">
	let { title, description, path }: { title: string; description: string; path: string } =
		$props();
	const site_url = (import.meta.env.VITE_SITE_URL || '').trim().replace(/\/+$/, '');
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<meta property="og:type" content="website" />
	<meta property="og:site_name" content="Hack Club Mipmap" />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	{#if site_url}
		<link rel="canonical" href="{site_url}{path}" />
		<meta property="og:url" content="{site_url}{path}" />
		<meta property="og:image" content="{site_url}/og.png" />
		<meta name="twitter:card" content="summary_large_image" />
	{:else}
		<meta name="twitter:card" content="summary" />
	{/if}
</svelte:head>
