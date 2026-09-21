<script lang="ts">
	type Hello = { message: string };

	let result = $state<string>('');

	async function ping() {
		const res = await fetch('/api/hello');
		const data: Hello = await res.json();
		result = data.message;
	}
</script>

<h1>mipmap</h1>
<button onclick={ping}>Call the API</button>
{#if result}
	<p>{result}</p>
{/if}
