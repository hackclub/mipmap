import type { TreeNode } from "$lib/tree/types";

export const category_colors: Record<string, string> = {
  Games: "#8fcf6b",
  Displays: "#7cc4f0",
  Equipment: "#ff9a3d",
  Knickknacks: "#f0788a",
  VFX: "#ffe066",
  Books: "#c3a3e6",
};

export const token_color = "#e8c58c";
export const origin_color = "#7B4B2A";
export const unlocked_border = "#d6ab70";

export function node_color(node: TreeNode) {
  if (node.kind === "token") return token_color;
  if (node.kind === "origin") return origin_color;
  return category_colors[node.category ?? ""] ?? origin_color;
}

export function price_label(
  node: TreeNode,
  currency: string,
  unlocked: boolean,
  purchased: boolean,
) {
  return purchased
    ? "Purchased!"
    : unlocked
      ? "Unlocked!"
      : `${node.price} ${currency}`;
}

export const cog_path =
  "M 363.579 21.749 A 380 380 0 0 1 436.421 21.749 L 447.319 63.309 A 340 340 0 0 1 503.452 76.121 L 531.302 43.405 A 380 380 0 0 1 596.931 75.011 L 588.718 117.183 A 340 340 0 0 1 633.733 153.081 L 673.020 135.689 A 380 380 0 0 1 718.437 192.640 L 692.739 227.072 A 340 340 0 0 1 717.720 278.947 L 760.663 280.323 A 380 380 0 0 1 776.872 351.340 L 738.779 371.212 A 340 340 0 0 1 738.779 428.788 L 776.872 448.660 A 380 380 0 0 1 760.663 519.677 L 717.720 521.053 A 340 340 0 0 1 692.739 572.928 L 718.437 607.360 A 380 380 0 0 1 673.020 664.311 L 633.733 646.919 A 340 340 0 0 1 588.718 682.817 L 596.931 724.989 A 380 380 0 0 1 531.302 756.595 L 503.452 723.879 A 340 340 0 0 1 447.319 736.691 L 436.421 778.251 A 380 380 0 0 1 363.579 778.251 L 352.681 736.691 A 340 340 0 0 1 296.548 723.879 L 268.698 756.595 A 380 380 0 0 1 203.069 724.989 L 211.282 682.817 A 340 340 0 0 1 166.267 646.919 L 126.980 664.311 A 380 380 0 0 1 81.563 607.360 L 107.261 572.928 A 340 340 0 0 1 82.280 521.053 L 39.337 519.677 A 380 380 0 0 1 23.128 448.660 L 61.221 428.788 A 340 340 0 0 1 61.221 371.212 L 23.128 351.340 A 380 380 0 0 1 39.337 280.323 L 82.280 278.947 A 340 340 0 0 1 107.261 227.072 L 81.563 192.640 A 380 380 0 0 1 126.980 135.689 L 166.267 153.081 A 340 340 0 0 1 211.282 117.183 L 203.069 75.011 A 380 380 0 0 1 268.698 43.405 L 296.548 76.121 A 340 340 0 0 1 352.681 63.309 L 363.579 21.749 Z";
export const cog_inner_radius = 210;
