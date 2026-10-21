import large_cog_svg from '$lib/assets/large-cog.svg?raw';
import { small_cog_path } from '$lib/cog_paths';
import { cog_size } from '$lib/cog_layout';

export const shadow_x = 4;
export const shadow_y = 6;
export const shadow_blur = 3;
export const shadow_pad = 4 * shadow_blur;

const large_cog_path = large_cog_svg.match(/ d="([^"]+)"/)![1];
const cache = new Map<string, string>();
const decoded: HTMLImageElement[] = [];

export function svg_url(svg: string) {
	return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

function scale(hex: string, factor: number) {
	return '#' + [1, 3, 5]
		.map((i) => Math.min(255, Math.round(parseInt(hex.slice(i, i + 2), 16) * factor)).toString(16).padStart(2, '0'))
		.join('');
}

function shapes(size: number, ring: string, inner: string) {
	if (size === cog_size)
		return `<circle cx="200" cy="200" r="108" fill="${inner}"/>` +
			`<path fill="${ring}" fill-rule="evenodd" d="${small_cog_path}"/>` +
			`<circle cx="200" cy="200" r="28" fill="${ring}"/>`;
	return `<path fill="${ring}" fill-rule="evenodd" d="${large_cog_path}"/>` +
		`<circle cx="400" cy="400" r="28" fill="${ring}"/>`;
}

function view_size(size: number) {
	return size === cog_size ? 400 : 800;
}

function cached(key: string, build: () => string) {
	let url = cache.get(key);
	if (!url) {
		url = svg_url(build());
		cache.set(key, url);
		if (typeof Image !== 'undefined') {
			const image = new Image();
			image.src = url;
			image.decode().catch(() => {});
			decoded.push(image);
		}
	}
	return url;
}

export function cog_image(size: number, brightness: number) {
	const level = Math.round(brightness * 100) / 100;
	return cached(`cog:${size}:${level}`, () => {
		const view = view_size(size);
		return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${view} ${view}">` +
			shapes(size, scale('#7b4b2a', level), scale('#59361e', level)) + '</svg>';
	});
}

function blurred(view: [number, number, number, number], content: string) {
	const [x, y, width, height] = view;
	return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${x} ${y} ${width} ${height}">` +
		`<filter id="s" filterUnits="userSpaceOnUse" x="${x}" y="${y}" width="${width}" height="${height}">` +
		`<feGaussianBlur stdDeviation="${shadow_blur}"/></filter>` +
		`<g opacity="0.45"><g filter="url(#s)">${content}</g></g></svg>`;
}

export function cog_shadow(size: number) {
	return cached(`shadow:${size}`, () =>
		blurred(
			[-shadow_pad, -shadow_pad, size + 2 * shadow_pad, size + 2 * shadow_pad],
			`<g transform="scale(${size / view_size(size)})">${shapes(size, '#000', '#000')}</g>`
		)
	);
}

const belts = new Map<string, string>();

export function belt_shadow(path: string, thickness: number, view: [number, number, number, number]) {
	const key = `${path}:${thickness}:${view.join(' ')}`;
	let url = belts.get(key);
	if (!url) {
		url = svg_url(
			blurred(
				view,
				`<path transform="translate(${shadow_x} ${shadow_y})" fill="none" stroke="#000" stroke-width="${thickness}" d="${path}"/>`
			)
		);
		belts.set(key, url);
	}
	return url;
}
