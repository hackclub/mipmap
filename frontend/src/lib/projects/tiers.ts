import type { Tier } from "$lib/projects/types";

export const tiers: { id: Tier; name: string; description: string }[] = [
  {
    id: "quebec",
    name: "Tier Quebec",
    description: "Custom 2D game engines",
  },
  {
    id: "romeo",
    name: "Tier Romeo",
    description:
      "With a standard low level graphics library. Most 3D projects will end up here",
  },
  {
    id: "sierra",
    name: "Tier Sierra",
    description:
      "Specifically written with Vulkan, on bare metal, or as a software renderer with no GPU library to speak of",
  },
];
