import type { Bounds } from "$lib/PanZoom.svelte";
import { grid } from "$lib/cog_layout";

const margin = 240;
const projects_width = 2400;
const side_width = 2400;
const fulfillment_height = 1440;
const orders_height = 780;
const referral_height = 780;
const profile_height = 760;
const panel_height = 3360;
const snapshots_width = 6300;
const gap = 48;

const dot = (value: number) =>
  Math.round((value - grid / 2) / grid) * grid + grid / 2;

function snap({ left, top, width, height }: Bounds): Bounds {
  const [l, t] = [dot(left), dot(top)];
  return {
    left: l,
    top: t,
    width: dot(left + width) - l,
    height: dot(top + height) - t,
  };
}

export function arrange(tree: { width: number; height: number }) {
  const projects = snap({
    left: margin,
    top: margin,
    width: projects_width,
    height: panel_height,
  });
  const tree_bounds: Bounds = {
    left: margin + projects_width,
    top: 0,
    width: tree.width,
    height: tree.height,
  };
  const side_left = tree_bounds.left + tree.width;
  const fulfillment = snap({
    left: side_left,
    top: margin,
    width: side_width,
    height: fulfillment_height,
  });
  const orders = snap({
    ...fulfillment,
    top: fulfillment.top + fulfillment.height + gap,
    height: orders_height,
  });
  const referral = snap({
    ...orders,
    top: orders.top + orders.height + gap,
    height: referral_height,
  });
  const profile = snap({
    ...referral,
    top: referral.top + referral.height + gap,
    height: profile_height,
  });
  const snapshots = snap({
    left: margin,
    top: tree.height,
    width: snapshots_width,
    height: panel_height,
  });
  const all: Bounds = {
    left: margin,
    top: margin,
    width: side_left + side_width - margin,
    height: snapshots.top + snapshots.height - margin,
  };
  return {
    projects,
    tree: tree_bounds,
    fulfillment,
    orders,
    referral,
    profile,
    snapshots,
    all,
  };
}
