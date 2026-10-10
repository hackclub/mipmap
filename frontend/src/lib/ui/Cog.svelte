<script lang="ts">
    import { small_cog_path } from "$lib/cog_paths";

    let { size = 200, color = "#7B4B2A" }: { size?: number; color?: string } =
        $props();
    const darker = $derived(`color-mix(in srgb, ${color} 72%, black)`);
</script>

<svg
    xmlns="http://www.w3.org/2000/svg"
    width={size}
    height={size}
    viewBox="0 0 400 400"
>
    <circle cx="200" cy="200" r="108" style:fill={darker} />
    <path
        fill={color}
        fill-rule="evenodd"
        d={small_cog_path}
    />
    <circle cx="200" cy="200" r="28" fill={color} />
</svg>
