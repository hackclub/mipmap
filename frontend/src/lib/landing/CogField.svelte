<script lang="ts">
	import { cog_size, large_cog_size, hub_radius } from '$lib/cog_layout';
	import { belt_shadow, cog_image, cog_shadow, shadow_pad, shadow_x, shadow_y } from '$lib/landing/cog_images';
	import { mulberry32, type Blocked, type FieldCog } from '$lib/landing/cog_field';
	import item_metadata from '$lib/assets/item_metadata.json';

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
	const prizes = Object.entries(item_metadata as Record<string, { title: string; image: string }>).map(
		([id, meta]) => ({ id, ...meta })
	);
	const prize_seed = Math.floor(Math.random() * 2 ** 31);

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
		const extent = belt_radius + belt_thickness / 2;
		const left = Math.min(from.x, cog.x) - extent - shadow_pad;
		const top = Math.min(from.y, cog.y) - extent - shadow_pad;
		const width = Math.abs(dx) + 2 * extent + 2 * shadow_pad + shadow_x;
		const height = Math.abs(dy) + 2 * extent + 2 * shadow_pad + shadow_y;
		const angle = Math.atan2(dy, dx) * (180 / Math.PI);
		return {
			path,
			shadow: {
				src: belt_shadow(path, belt_thickness, [left, top, width, height]),
				left,
				top,
				width,
				height
			},
			runs: [
				{ x: from.x + nx, y: from.y + ny, angle },
				{ x: cog.x - nx, y: cog.y - ny, angle: angle + 180 }
			],
			perimeter: 2 * distance + 2 * Math.PI * belt_radius,
			mid_x: (from.x + cog.x) / 2,
			mid_y: (from.y + cog.y) / 2,
			distance,
			top: Math.min(from.y, cog.y) - belt_radius - belt_thickness / 2,
			bottom: Math.max(from.y, cog.y) + belt_radius + belt_thickness / 2,
			clockwise: cog.parity === 1
		};
	}

	const cog_prizes = $derived.by(() => {
		const rng = mulberry32(prize_seed);
		const assigned = new Map<FieldCog, { prize: (typeof prizes)[number]; phase: number }>();
		let deck: typeof prizes = [];
		for (const cog of cogs) {
			if (cog.size !== large_cog_size) continue;
			if (deck.length === 0) {
				deck = [...prizes];
				for (let i = deck.length - 1; i > 0; i--) {
					const j = Math.floor(rng() * (i + 1));
					[deck[i], deck[j]] = [deck[j], deck[i]];
				}
			}
			assigned.set(cog, { prize: deck.pop()!, phase: rng() });
		}
		return assigned;
	});

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

{#each visible_belts as belt (belt)}
	<img decoding="sync"
		class="belt-shadow"
		src={belt.shadow.src}
		style:left="{belt.shadow.left}px"
		style:top="{belt.shadow.top}px"
		width={belt.shadow.width}
		height={belt.shadow.height}
		alt=""
	/>
{/each}

{#each visible_cogs as cog (cog)}
	<div
		class="cog"
		style:left="{cog.x - cog.size / 2 - shadow_pad + shadow_x}px"
		style:top="{cog.y - cog.size / 2 - shadow_pad + shadow_y}px"
		style:--spin-duration="{12 * cog.size / cog_size}s"
	>
		<div style:transform="rotate({cog.offset}deg)">
			<div class={cog.parity === 1 ? 'cw' : 'ccw'}>
				<img decoding="sync" src={cog_shadow(cog.size)} width={cog.size + 2 * shadow_pad} height={cog.size + 2 * shadow_pad} alt="" />
			</div>
		</div>
	</div>
{/each}

<svg class="belts" aria-hidden="true">
	{#each visible_belts as belt (belt)}
		<path class="belt" d={belt.path} stroke-width={belt_thickness} />
	{/each}
</svg>

{#each visible_belts as belt (belt)}
	{#each belt.runs as run}
		<div
			class="run"
			style:left="{run.x}px"
			style:top="{run.y - belt_thickness / 2}px"
			style:width="{belt.distance}px"
			style:height="{belt_thickness}px"
			style:transform="rotate({run.angle}deg)"
		>
			<div
				class="track"
				class:clockwise={belt.clockwise}
				style:width="{belt.perimeter}px"
				style:--duration="{belt.perimeter / 2 / belt_speed}s"
			>
				<span
					style:left="0"
					style:width="{stripe_length + belt_thickness}px"
					style:margin-left="{-belt_thickness / 2}px"
				></span>
				<span
					style:left="50%"
					style:width="{stripe_length + belt_thickness}px"
					style:margin-left="{-belt_thickness / 2}px"
				></span>
			</div>
		</div>
	{/each}
{/each}

{#each visible_cogs as cog (cog)}
	<div
		class="cog"
		style:left="{cog.x - cog.size / 2}px"
		style:top="{cog.y - cog.size / 2}px"
		style:--spin-duration="{12 * cog.size / cog_size}s"
	>
		<div style:transform="rotate({cog.offset}deg)">
			<div class={cog.parity === 1 ? 'cw' : 'ccw'}>
				<img decoding="sync" src={cog_image(cog.size, brightness_for(cog))} width={cog.size} height={cog.size} alt="" />
			</div>
		</div>
		{#if cog_prizes.get(cog)}
			{@const { prize, phase } = cog_prizes.get(cog)!}
			<div
				class="prize"
				class:reverse={cog.parity !== 1}
				style:--clutch-phase={phase}
			>
				<img
					src="/dashboard/item-images/{prize.id}"
					alt=""
					loading="lazy"
					decoding="async"
					referrerpolicy="no-referrer"
					onerror={(e) => {
						const img = e.currentTarget as HTMLImageElement;
						const fallback = new URL(prize.image, location.href).href;
						if (img.src !== fallback) img.src = fallback;
					}}
				/>
				<span>{prize.title}</span>
			</div>
		{/if}
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

	.belt {
		fill: none;
		stroke: #4f2f1f;
	}

	.belt-shadow {
		position: absolute;
		pointer-events: none;
	}

	.run {
		position: absolute;
		overflow: hidden;
		transform-origin: 0 50%;
		pointer-events: none;
	}

	.track {
		position: absolute;
		inset: 0 auto 0 0;
		animation: belt-travel var(--duration) linear infinite;
	}

	.track.clockwise {
		animation-direction: reverse;
	}

	.track span {
		position: absolute;
		top: 0;
		height: 100%;
		border-radius: 999px;
		background: #a87958;
	}

	@keyframes belt-travel {
		from {
			transform: translateX(-50%);
		}
		to {
			transform: translateX(0);
		}
	}

	.cog {
		position: absolute;
		line-height: 0;
	}

	.prize {
		position: absolute;
		left: 50%;
		top: 50%;
		box-sizing: border-box;
		width: 108px;
		height: 108px;
		margin: -54px 0 0 -54px;
		padding: 6px;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 4px;
		background: #74472a;
		border: 2px solid #4f2f1f;
		border-radius: 7px;
		color: #ead7c2;
		font-size: 0.8rem;
		line-height: 1.15;
		text-align: center;
		animation: clutch calc(var(--spin-duration) * 0.336) linear infinite;
		animation-delay: calc(var(--spin-duration) * -0.336 * var(--clutch-phase));
	}

	.prize.reverse {
		animation-name: clutch-reverse;
	}

	.prize img {
		width: 100%;
		height: 56px;
		object-fit: contain;
		border-radius: 4px;
	}

	.prize span {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		overflow: hidden;
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

	@keyframes clutch {
		0% {
			transform: rotate(-14deg);
			animation-timing-function: linear;
		}
		77.7% {
			transform: rotate(80deg);
			animation-timing-function: ease-in-out;
		}
		85.1% {
			transform: rotate(-42deg);
			animation-timing-function: ease-in-out;
		}
		92.6% {
			transform: rotate(24deg);
			animation-timing-function: ease-in-out;
		}
		100% {
			transform: rotate(-14deg);
		}
	}

	@keyframes clutch-reverse {
		0% {
			transform: rotate(14deg);
			animation-timing-function: linear;
		}
		77.7% {
			transform: rotate(-80deg);
			animation-timing-function: ease-in-out;
		}
		85.1% {
			transform: rotate(42deg);
			animation-timing-function: ease-in-out;
		}
		92.6% {
			transform: rotate(-24deg);
			animation-timing-function: ease-in-out;
		}
		100% {
			transform: rotate(14deg);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.cw,
		.ccw,
		.prize,
		.track {
			animation: none;
		}
	}
</style>
