import type { Bounds } from "$lib/dashboard/PanZoom.svelte";
import { grid } from "$lib/cog_layout";

const margin = 240;
const projects_width = 2400;
const side_width = 2400;
const fulfillment_height = 1752;
const orders_height = 780;
const feedback_height = 2100;
const help_height = 2100;
const referral_height = 780;
const challenge_height = 780;
const profile_height = 1200;
const panel_height = 3360;
const snapshots_width = 6300;
const snapshots_height = 4200;
const gap = 48;
const steps_gap = 192;

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
  const steps = snap({
    left: margin,
    top: projects.top + projects.height + gap,
    width: projects_width,
    height: tree.height - steps_gap - (projects.top + projects.height + gap),
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
  const challenge = snap({
    ...referral,
    top: referral.top + referral.height + gap,
    height: challenge_height,
  });
  const profile = snap({
    ...challenge,
    top: challenge.top + challenge.height + gap,
    height: profile_height,
  });
  const snapshots = snap({
    left: margin,
    top: tree.height,
    width: snapshots_width,
    height: snapshots_height,
  });
  const feedback = snap({
    left: side_left,
    top: tree.height,
    width: side_width,
    height: feedback_height,
  });
  const help = snap({
    left: side_left,
    top: Math.max(tree.height, profile.top + profile.height + gap),
    width: side_width,
    height: help_height,
  });
  const all: Bounds = {
    left: margin,
    top: margin,
    width:
      Math.max(side_left + side_width, snapshots.left + snapshots.width) -
      margin,
    height:
      Math.max(
        snapshots.top + snapshots.height,
        help.top + help.height,
      ) - margin,
  };
  return {
    projects,
    steps,
    tree: tree_bounds,
    fulfillment,
    orders,
    feedback,
    help,
    referral,
    challenge,
    profile,
    snapshots,
    all,
  };
}
