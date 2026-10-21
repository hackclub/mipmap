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
      "2D or 3D games in most low level graphics libraries (OpenGL, DirectX, BGFX, etc.)",
  },
  {
    id: "sierra",
    name: "Tier Sierra",
    description:
      "A 3D game in Vulkan, a software renderer (no graphics libraries), or on bare metal (no operating system)",
  },
];
