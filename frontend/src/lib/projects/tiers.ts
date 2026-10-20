import type { Tier } from "$lib/projects/types";

export const tiers: { id: Tier; name: string; description: string }[] = [
  {
    id: "quebec",
    name: "Tier Quebec",
    description: "2D games without engines using simpler libraries",
  },
  {
    id: "romeo",
    name: "Tier Romeo",
    description:
      "With a standard low level graphics library (OpenGL, DirectX, BGFX, etc.). Most 3D projects will end up here",
  },
  {
    id: "sierra",
    name: "Tier Sierra",
    description:
      "Specifically written with Vulkan, on bare metal (no operating system), or as a software renderer (no graphics library)",
  },
];
