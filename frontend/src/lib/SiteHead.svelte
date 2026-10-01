<script lang="ts">
	const title = 'Hack Club Mipmap - Build a Game Without an Engine';
	const description =
		'Hack Club Mipmap challenges teens to build a game without an engine, learn how games work, progress a tech tree, and unlock prizes.';
	const site_url = (import.meta.env.VITE_SITE_URL || '').trim().replace(/\/+$/, '');
	const canonical = site_url ? `${site_url}/` : null;
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<meta property="og:type" content="website" />
	<meta property="og:site_name" content="Hack Club Mipmap" />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta name="twitter:card" content="summary" />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	{#if canonical}
		<link rel="canonical" href={canonical} />
		<meta property="og:url" content={canonical} />
	{/if}
</svelte:head>
