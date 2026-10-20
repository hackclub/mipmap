export type NodeKind = "item" | "token" | "origin";

export interface TreeNode {
  id: number;
  kind: NodeKind;
  name: string;
  category: string | null;
  price?: number;
  requires: number[];
}

export interface Tree {
  currency: string;
  nodes: TreeNode[];
}

export interface ItemMeta {
  title: string;
  description: string;
  facts?: string;
  image: string;
}

export type ItemMetadata = Record<string, ItemMeta>;

export interface Shape {
  cog: number;
  label_width: number;
  label_height: number;
  label_offset: number;
}

export interface Layout {
  width: number;
  height: number;
  shapes: Record<NodeKind, Shape>;
  nodes: { id: number; x: number; y: number }[];
  edges: { from: number; to: number; path: string }[];
}
