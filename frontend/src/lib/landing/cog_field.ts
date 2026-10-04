import { cog_size, large_cog_size, grid, tooth_pitch } from '$lib/cog_layout';

export type FieldCog = {
	x: number;
	y: number;
	parity: 1 | -1;
	offset: number;
	size: number;
	parent?: FieldCog;
};
export type Blocked = { left: number; right: number; top: number; bottom: number };

const gap = cog_size;
const max_cogs = 1500;
const rad = Math.PI / 180;
const outer_radius = (190 / 400) * cog_size;

function make_steps(axial: number | null, diagonals: [number, number][]) {
	const vectors: [number, number][] = [];
	if (axial !== null) vectors.push([axial, 0], [-axial, 0], [0, axial], [0, -axial]);
	for (const sx of [1, -1]) {
		for (const sy of [1, -1]) {
			for (const [x, y] of diagonals) vectors.push([x * sx, y * sy]);
		}
	}
	return vectors
		.map(([a, b]) => ({ x: a * grid, y: b * grid, angle: Math.atan2(a, -b) / rad }))
		.sort((p, q) => p.angle - q.angle);
}

const small_steps = make_steps(6, [[5, 3], [3, 5], [4, 4]]);
const mixed_steps = make_steps(null, [[9, 2], [8, 5], [5, 8], [2, 9]]);
const large_steps = make_steps(12, [[11, 5], [5, 11], [9, 8]]);

function steps_for(parent_size: number, child_size: number) {
	if (parent_size === child_size) return parent_size === cog_size ? small_steps : large_steps;
	return mixed_steps;
}

function nearest_step(angle: number, steps: typeof small_steps) {
	let best = 0;
	let best_error = Infinity;
	steps.forEach((s, i) => {
		const error = Math.abs(((s.angle - angle + 540) % 360) - 180);
		if (error < best_error) {
			best_error = error;
			best = i;
		}
	});
	return best;
}

export function mulberry32(seed: number) {
	return () => {
		seed |= 0;
		seed = (seed + 0x6d2b79f5) | 0;
		let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

export function grow_field(width: number, height: number, blocked: Blocked[]): FieldCog[] {
	const gears: FieldCog[] = [];
	if (width < gap || height < gap) return gears;
	const rng = mulberry32(7);
	const n = small_steps.length;
	const low = grid / 2;
	const top_edge = low - grid * Math.ceil((2 * cog_size) / grid);
	const right_edge = low + grid * Math.floor((width - grid) / grid);
	const bottom_edge = low + grid * Math.floor((height - grid) / grid);

	const inside = (x: number, y: number, size: number) =>
		x >= low && x <= right_edge && y >= top_edge &&
		y + outer_radius * (size / cog_size) <= bottom_edge;

	const free = (x: number, y: number, size: number, except?: FieldCog) => {
		const radius = outer_radius * (size / cog_size);
		return inside(x, y, size) &&
			!blocked.some((box) =>
				x + radius > box.left && x - radius < box.right &&
				y + radius > box.top && y - radius < box.bottom
			) &&
			gears.every((o) => o === except ||
				Math.hypot(o.x - x, o.y - y) >= radius + outer_radius * (o.size / cog_size) + 4);
	};

	function add(parent: FieldCog, index: number, size: number) {
		const s = steps_for(parent.size, size)[index];
		const parent_pitch = tooth_pitch * cog_size / parent.size;
		const child_pitch = tooth_pitch * cog_size / size;
		const parent_phase = ((s.angle - parent.offset + parent_pitch / 2) % parent_pitch +
			parent_pitch) % parent_pitch - parent_pitch / 2;
		const child: FieldCog = {
			x: parent.x + s.x,
			y: parent.y + s.y,
			parity: parent.parity === 1 ? -1 : 1,
			offset: s.angle + 180 - child_pitch / 2 + parent_phase * parent.size / size,
			size,
			parent
		};
		gears.push(child);
		return child;
	}

	function grow(start: FieldCog, first: number) {
		const before = gears.length;
		const p_turn = 0.25 + 0.5 * rng();
		let turn = rng() < 0.5 ? 1 : -1;
		let current = start;
		let heading = first;
		let started = false;
		while (gears.length < max_cogs) {
			if (started) {
				if (rng() < p_turn) heading = (heading + turn + n) % n;
				if (rng() < 0.12) turn = -turn;
			}
			started = true;
			const tries = [0, turn, -turn, 2 * turn, -2 * turn];
			let moved = false;
			const wanted_size = current.size === large_cog_size ? cog_size :
				rng() < 0.12 ? large_cog_size : cog_size;
			for (const size of wanted_size === large_cog_size ? [large_cog_size, cog_size] : [cog_size]) {
				const steps = steps_for(current.size, size);
				for (const t of tries) {
					const index = (heading + t + n) % n;
					const s = steps[index];
					if (!free(current.x + s.x, current.y + s.y, size, current)) continue;
					current = add(current, index, size);
					heading = index;
					moved = true;
					break;
				}
				if (moved) break;
			}
			if (!moved) break;
		}
		return gears.length - before;
	}

	function seed_logo_side(right: boolean) {
		const logo = blocked[0];
		if (!logo || !Number.isFinite(logo.bottom)) return;
		const room = right ? width - logo.right : logo.left;
		if (room < 2 * outer_radius + 2 * grid) return;
		const x = right ? right_edge - 3 * grid : low + 3 * grid;
		const middle_y = low + grid * Math.round((logo.bottom / 2 - low) / grid);
		for (const offset of [0, -6 * grid, 6 * grid]) {
			const y = middle_y + offset;
			if (!free(x, y, cog_size)) continue;
			const before = gears.length;
			const root: FieldCog = { x, y, parity: 1, offset: 0, size: cog_size };
			gears.push(root);
			if (grow(root, nearest_step(180, small_steps)) >= 2) return;
			gears.length = before;
		}
	}

	seed_logo_side(true);
	seed_logo_side(false);

	const seeds = 26;
	for (let i = 0; i < seeds; i++) {
		const side = Math.floor(rng() * 4);
		const wanted_size = rng() < 0.25 ? large_cog_size : cog_size;
		let x = 0;
		let y = 0;
		let inward = 0;
		if (side === 0) {
			x = low;
			y = top_edge + grid * Math.floor(rng() * ((bottom_edge - top_edge) / grid));
			inward = 90;
		} else if (side === 1) {
			x = right_edge;
			y = top_edge + grid * Math.floor(rng() * ((bottom_edge - top_edge) / grid));
			inward = 270;
		} else if (side === 2) {
			y = top_edge;
			x = low + grid * Math.floor(rng() * ((right_edge - low) / grid));
			inward = 180;
		} else {
			y = bottom_edge - grid * Math.ceil(
				outer_radius * (wanted_size / cog_size) / grid
			);
			x = low + grid * Math.floor(rng() * ((right_edge - low) / grid));
			inward = 0;
		}
		const size = free(x, y, wanted_size) ? wanted_size : cog_size;
		if (!free(x, y, size)) continue;
		const before = gears.length;
		const root: FieldCog = { x, y, parity: 1, offset: 0, size };
		gears.push(root);
		const jitter = Math.floor(rng() * 5) - 2;
		if (grow(root, (nearest_step(inward, steps_for(size, cog_size)) + jitter + n) % n) < 2)
			gears.length = before;
	}

	for (let pass = 0; pass < 8; pass++) {
		let added = false;
		const order = gears.map((_, i) => i).sort(() => rng() - 0.5);
		for (const i of order) {
			if (rng() > 0.7) continue;
			const first = Math.floor(rng() * n);
			const before = gears.length;
			if (grow(gears[i], first) < 2) gears.length = before;
			else added = true;
		}
		if (!added) break;
	}
	return gears;
}
