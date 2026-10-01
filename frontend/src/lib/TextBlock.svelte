<script lang="ts">
	import type { Snippet } from 'svelte';
	import { grid, cog_size } from '$lib/cog_layout';
	import { mulberry32 } from '$lib/cog_field';

	let {
		children,
		x = grid / 2,
		y = 192,
		width = 480,
		snap_width = true,
		cogs = [],
		obstacles = [],
		variant = 'title',
		min_height = 0,
		box_height = $bindable(0),
		content_height = $bindable(0)
	}: {
		children?: Snippet;
		x?: number;
		y?: number;
		width?: number;
		snap_width?: boolean;
		cogs?: { x: number; y: number; size?: number }[];
		obstacles?: { left: number; right: number; top: number; bottom: number }[];
		variant?: 'title' | 'body' | 'faq';
		min_height?: number;
		box_height?: number;
		content_height?: number;
	} = $props();

	const clearance = 8;
	const span = $derived(snap_width ? grid * Math.max(1, Math.round(width / grid)) : width);

	function segment_distance(px: number, py: number, ax: number, ay: number, bx: number, by: number) {
		const dx = bx - ax;
		const dy = by - ay;
		const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)));
		return Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
	}

	function crosses_box(ax: number, ay: number, bx: number, by: number, box: (typeof obstacles)[number]) {
		if (box.bottom <= box.top) return false;
		const margin = 8;
		const dx = bx - ax;
		const dy = by - ay;
		let near = 0;
		let far = 1;
		for (const [p, q] of [
			[-dx, ax - box.left + margin],
			[dx, box.right + margin - ax],
			[-dy, ay - box.top + margin],
			[dy, box.bottom + margin - ay]
		]) {
			if (p === 0) {
				if (q < 0) return false;
			} else {
				const t = q / p;
				if (p < 0) near = Math.max(near, t);
				else far = Math.min(far, t);
			}
		}
		return near <= far;
	}

	function anchor_for(corner_x: number, corner_y: number, side: number) {
		const rng = mulberry32(Math.round(x * 31 + y * 17 + side * 101));
		const max_steps = Math.max(1, Math.min(4, Math.floor((span - grid) / (2 * grid))));
		const candidates: { x: number; y: number }[] = [];
		for (let step = 1; step <= max_steps; step++) {
			for (let rise = 96; rise <= 288; rise += grid) {
				const ideal_x = corner_x + (side === 0 ? 1 : -1) * step * grid;
				candidates.push({
					x: grid / 2 + grid * Math.round((ideal_x - grid / 2) / grid),
					y: grid / 2 + grid * Math.round((corner_y - rise - grid / 2) / grid)
				});
			}
		}
		for (let i = candidates.length - 1; i > 0; i--) {
			const j = Math.floor(rng() * (i + 1));
			[candidates[i], candidates[j]] = [candidates[j], candidates[i]];
		}
		let fallback: { x: number; y: number } | null = null;
		let fallback_room = -Infinity;
		for (const c of candidates) {
			if (c.y < grid / 2) continue;
			if (obstacles.some((box) => crosses_box(c.x, c.y, corner_x, corner_y, box))) continue;
			const room = Math.min(
				...cogs.map((g) =>
					Math.min(
						Math.hypot(g.x - c.x, g.y - c.y),
						segment_distance(g.x, g.y, c.x, c.y, corner_x, corner_y) + 4
					) - (g.size ?? cog_size) / 2
				),
				Infinity
			);
			if (room >= clearance) return c;
			if (room > fallback_room) {
				fallback_room = room;
				fallback = c;
			}
		}
		return fallback;
	}

	const cords = $derived(
		[x, x + span].flatMap((corner_x, side) => {
			const anchor = anchor_for(corner_x, y, side);
			if (!anchor) return [];
			const dx = corner_x - anchor.x;
			const dy = y - anchor.y;
			return [{
				from_x: anchor.x - x,
				from_y: anchor.y - y,
				to_x: corner_x - x,
				length: Math.hypot(dx, dy),
				angle: Math.atan2(-dx, dy) * (180 / Math.PI)
			}];
		})
	);
</script>

<div class="block" style:left="{x}px" style:top="{y}px" style:width="{span}px">
	{#each cords as cord}
		<span
			class="cord"
			style:left="{cord.from_x}px"
			style:top="{cord.from_y}px"
			style:height="{cord.length}px"
			style:transform="rotate({cord.angle}deg)"
		></span>
		<span class="anchor" style:left="{cord.from_x}px" style:top="{cord.from_y}px"></span>
	{/each}
	<div class="box" class:body={variant === 'body'} class:faq={variant === 'faq'} bind:clientHeight={box_height}>
		<div style:min-height="{min_height}px">
			<div bind:clientHeight={content_height}>
				{@render children?.()}
			</div>
		</div>
	</div>
	{#each cords as cord}
		<span class="anchor" style:left="{cord.to_x}px" style:top="0px"></span>
	{/each}
</div>

<style>
	.block {
		position: absolute;
	}

	.cord {
		position: absolute;
		width: 3px;
		margin-left: -1.5px;
		transform-origin: 50% 0;
		background: #555555;
	}

	.anchor {
		position: absolute;
		width: 14px;
		height: 14px;
		margin: -7px 0 0 -7px;
		border-radius: 50%;
		background: #4f2f1f;
	}

	.box {
		box-sizing: border-box;
		padding: 1.5rem;
		background: #74472a;
		border: 2px solid #4f2f1f;
		border-radius: 7px;
		color: #ead7c2;
		font-size: clamp(2rem, 4vw, 3.5rem);
		font-weight: 700;
		line-height: 1.15;
		text-align: center;
	}

	.box.body {
		font-size: clamp(1.25rem, 2vw, 1.75rem);
		font-weight: 400;
		line-height: 1.5;
		text-align: left;
	}

	.box.faq {
		font-size: clamp(1.25rem, 2vw, 1.75rem);
		font-weight: 400;
		line-height: 1.35;
		text-align: left;
	}

	.box :global(p) {
		margin: 0;
	}

	.box :global(h1),
	.box :global(h2) {
		margin: 0;
		font: inherit;
	}
</style>
