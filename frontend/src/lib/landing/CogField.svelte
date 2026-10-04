<script lang="ts">
	import Cog from '$lib/ui/Cog.svelte';
	import large_cog from '$lib/assets/large-cog.svg';
	import { cog_size, large_cog_size, hub_radius } from '$lib/cog_layout';
	import type { Blocked, FieldCog } from '$lib/landing/cog_field';

	let {
		cogs,
		scroll_y,
		viewport_height,
		footer_blocked
	}: { cogs: FieldCog[]; scroll_y: number; viewport_height: number; footer_blocked: Blocked } = $props();
	const belt_thickness = hub_radius / 2;
	const belt_radius = hub_radius + belt_thickness / 2;
	const belt_speed = 80;
	const stripe_length = 24;
	const render_margin = cog_size;
	const max_belt_height = 900 + 2 * (belt_radius + belt_thickness / 2);
	const belt_cell_size = 420;

	function brightness_for(cog: FieldCog) {
		let hash = Math.imul(cog.x, 374761393) ^ Math.imul(cog.y, 668265263);
		hash = Math.imul(hash ^ (hash >>> 13), 1274126177);
		return 0.95 + (hash >>> 0) / 4294967295 * 0.1;
	}

	function lower_bound<T>(items: T[], y: number, position: (item: T) => number) {
		let low = 0;
		let high = items.length;
		while (low < high) {
			const middle = (low + high) >>> 1;
			if (position(items[middle]) < y) low = middle + 1;
			else high = middle;
		}
		return low;
	}

	function crosses_footer(ax: number, ay: number, bx: number, by: number) {
		const padding = belt_radius + belt_thickness / 2;
		const left = footer_blocked.left - padding;
		const right = footer_blocked.right + padding;
		const top = footer_blocked.top - padding;
		const bottom = footer_blocked.bottom + padding;
		const dx = bx - ax;
		const dy = by - ay;
		let near = 0;
		let far = 1;
		for (const [p, q] of [
			[-dx, ax - left],
			[dx, right - ax],
			[-dy, ay - top],
			[dy, bottom - ay]
		]) {
			if (p === 0) {
				if (q < 0) return false;
			} else {
				const distance = q / p;
				if (p < 0) near = Math.max(near, distance);
				else far = Math.min(far, distance);
			}
		}
		return near <= far;
	}

	function belt_for(cog: FieldCog) {
		let ancestor: FieldCog | undefined = cog;
		let best: { from: FieldCog; distance: number } | null = null;
		for (let depth = 1; depth <= 6; depth++) {
			ancestor = ancestor?.parent;
			if (!ancestor) break;
			if (depth < 4 || depth % 2 !== 0) continue;
			const dx = cog.x - ancestor.x;
			const dy = cog.y - ancestor.y;
			const distance = Math.hypot(dx, dy);
			if (distance < 280 || distance > 900) continue;
			if (crosses_footer(ancestor.x, ancestor.y, cog.x, cog.y)) continue;
			if (!best || Math.abs(distance - 560) < Math.abs(best.distance - 560))
				best = { from: ancestor, distance };
		}
		if (!best) return null;
		const from = best.from;
		const dx = cog.x - from.x;
		const dy = cog.y - from.y;
		const distance = best.distance;
		const nx = (-dy / distance) * belt_radius;
		const ny = (dx / distance) * belt_radius;
		const path = [
			`M ${from.x + nx} ${from.y + ny}`,
			`L ${cog.x + nx} ${cog.y + ny}`,
			`A ${belt_radius} ${belt_radius} 0 0 0 ${cog.x - nx} ${cog.y - ny}`,
			`L ${from.x - nx} ${from.y - ny}`,
			`A ${belt_radius} ${belt_radius} 0 0 0 ${from.x + nx} ${from.y + ny}`,
			'Z'
		].join(' ');
		return {
			path,
			perimeter: 2 * distance + 2 * Math.PI * belt_radius,
			mid_x: (from.x + cog.x) / 2,
			mid_y: (from.y + cog.y) / 2,
			distance,
			top: Math.min(from.y, cog.y) - belt_radius - belt_thickness / 2,
			bottom: Math.max(from.y, cog.y) + belt_radius + belt_thickness / 2,
			clockwise: cog.parity === 1
		};
	}

	const ordered_cogs = $derived([...cogs].sort((a, b) => a.y - b.y));
	const visible_cogs = $derived.by(() => {
		const top = scroll_y - large_cog_size / 2 - render_margin;
		const bottom = scroll_y + viewport_height + large_cog_size / 2 + render_margin;
		return ordered_cogs.slice(
			lower_bound(ordered_cogs, top, (cog) => cog.y),
			lower_bound(ordered_cogs, bottom, (cog) => cog.y)
		);
	});

	const belts = $derived.by(() => {
		const cells = new Map<string, NonNullable<ReturnType<typeof belt_for>>>();
		for (const cog of cogs) {
			const belt = belt_for(cog);
			if (!belt) continue;
			const column = Math.floor(belt.mid_x / belt_cell_size);
			const row = Math.floor(belt.mid_y / belt_cell_size);
			const key = `${column}:${row}`;
			const current = cells.get(key);
			const score = (candidate: typeof belt) =>
				Math.hypot(
					candidate.mid_x - (column + 0.5) * belt_cell_size,
					candidate.mid_y - (row + 0.5) * belt_cell_size
				) + Math.abs(candidate.distance - 560) / 2;
			if (!current || score(belt) < score(current)) cells.set(key, belt);
		}
		return [...cells.values()].sort((a, b) => a.top - b.top);
	});
	const visible_belts = $derived.by(() => {
		const top = scroll_y - render_margin;
		const bottom = scroll_y + viewport_height + render_margin;
		return belts.slice(
			lower_bound(belts, top - max_belt_height, (belt) => belt.top),
			lower_bound(belts, bottom, (belt) => belt.top)
		).filter((belt) => belt.bottom >= top);
	});
</script>

<svg class="belts" aria-hidden="true">
	{#each visible_belts as belt (belt)}
		<path class="belt" d={belt.path} stroke-width={belt_thickness} />
		<path
			class="stripe"
			class:clockwise={belt.clockwise}
			d={belt.path}
			stroke-width={belt_thickness}
			stroke-dasharray={`${stripe_length} ${belt.perimeter / 2 - stripe_length}`}
			style:--travel="{belt.perimeter}px"
			style:--duration="{belt.perimeter / belt_speed}s"
		/>
	{/each}
</svg>

{#each visible_cogs as cog (cog)}
	<div
		class="cog"
		style:left="{cog.x - cog.size / 2}px"
		style:top="{cog.y - cog.size / 2}px"
		style:--spin-duration="{12 * cog.size / cog_size}s"
		style:filter="brightness({brightness_for(cog)})"
	>
		<div style:transform="rotate({cog.offset}deg)">
			<div class={cog.parity === 1 ? 'cw' : 'ccw'}>
				{#if cog.size === large_cog_size}
					<img src={large_cog} width={cog.size} height={cog.size} alt="" />
				{:else}
					<Cog size={cog.size} />
				{/if}
			</div>
		</div>
	</div>
{/each}

<style>
	.belts {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
		pointer-events: none;
	}

	.belt,
	.stripe {
		fill: none;
		stroke-linecap: round;
	}

	.belt {
		stroke: #4f2f1f;
	}

	.stripe {
		stroke: #a87958;
		animation: belt-travel var(--duration) linear infinite;
	}

	.stripe.clockwise {
		animation-direction: reverse;
	}

	@keyframes belt-travel {
		from {
			stroke-dashoffset: 0;
		}
		to {
			stroke-dashoffset: calc(-1 * var(--travel));
		}
	}

	.cog {
		position: absolute;
		line-height: 0;
	}

	.cw,
	.ccw {
		animation: spin var(--spin-duration) linear infinite;
	}

	.ccw {
		animation-direction: reverse;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.cw,
		.ccw,
		.stripe {
			animation: none;
		}
	}
</style>
