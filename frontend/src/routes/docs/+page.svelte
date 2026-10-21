<script lang="ts">
    import SiteHead from "$lib/SiteHead.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import LibraryPicker, { tier_labels, type Library } from "$lib/docs/LibraryPicker.svelte";

    let { data } = $props();

    function seconds(value: string | null) {
        if (!value) return 0;
        if (/^\d+$/.test(value)) return Number(value);
        const match = /^(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?$/.exec(value);
        return match
            ? Number(match[1] ?? 0) * 3600 + Number(match[2] ?? 0) * 60 + Number(match[3] ?? 0)
            : 0;
    }

    function embedUrl(value: string | undefined) {
        try {
            const url = new URL((value ?? "").trim().replace(/^["']+|["']+$/g, "").trim());
            const host = url.hostname.replace(/^(www|m)\./, "");
            const parts = url.pathname.split("/").filter(Boolean);
            const id =
                host === "youtu.be"
                    ? parts[0]
                    : (host === "youtube.com" || host === "youtube-nocookie.com") &&
                        ["embed", "shorts", "live"].includes(parts[0])
                      ? parts[1]
                      : url.searchParams.get("v");
            if (!id || !/^[\w-]{1,32}$/.test(id)) return null;
            const start = seconds(url.searchParams.get("t") ?? url.searchParams.get("start"));
            return `https://www.youtube-nocookie.com/embed/${id}${start ? `?start=${start}` : ""}`;
        } catch {
            return null;
        }
    }

    const video = embedUrl(import.meta.env.VITE_DOCS_VIDEO_URL);

    let picking = $state(false);

    const libraries: Library[] = [
        {
            name: "SDL",
            description: "Cross-platform windows, input and audio",
            tier: "window",
            note: "for opening the window and managing events such as input",
            links: [
                { text: "SDL website", href: "https://www.libsdl.org/" },
                { text: "SDL3 documentation", href: "https://wiki.libsdl.org/SDL3/FrontPage" },
            ],
        },
        {
            name: "GLFW",
            description: "Lightweight windows and input for OpenGL and Vulkan",
            tier: "window",
            note: "for opening the window and managing events such as input",
            links: [
                { text: "GLFW website", href: "https://www.glfw.org/" },
                { text: "GLFW quick guide", href: "https://www.glfw.org/docs/latest/quick_guide.html" },
            ],
        },
        {
            name: "OpenGL",
            description: "Cross-platform graphics library",
            tier: "romeo",
            links: [
                { text: "LearnOpenGL", href: "https://learnopengl.com/" },
                { text: "docs.gl API reference", href: "https://docs.gl/" },
                { text: "Khronos OpenGL Wiki", href: "https://www.khronos.org/opengl/wiki/" },
            ],
        },
        {
            name: "Vulkan",
            description: "Low level cross-platform graphics library",
            tier: "sierra",
            links: [
                { text: "How to Vulkan", href: "https://howtovulkan.com/" },
                { text: "Vulkan Guide", href: "https://vkguide.dev/" },
                { text: "Official Vulkan documentation", href: "https://docs.vulkan.org/" },
            ],
        },
        {
            name: "Direct3D",
            description: "Microsoft's graphics library",
            tier: "romeo",
            links: [
                { text: "Direct3D 12 programming guide", href: "https://learn.microsoft.com/en-us/windows/win32/direct3d12/directx-12-programming-guide" },
                { text: "Learning DirectX 12 (3D Game Engine Programming)", href: "https://www.3dgep.com/learning-directx-12-1/" },
            ],
        },
        {
            name: "wgpu",
            description: "Rust graphics library based on WebGPU",
            tier: "romeo",
            links: [
                { text: "wgpu website", href: "https://wgpu.rs/" },
                { text: "Learn Wgpu", href: "https://sotrh.github.io/learn-wgpu/" },
                { text: "WebGPU Fundamentals", href: "https://webgpufundamentals.org/" },
            ],
        },
        {
            name: "bgfx",
            description: "C/C++ graphics library that runs on top of every major API",
            tier: "romeo",
            links: [
                { text: "bgfx documentation", href: "https://bkaradzic.github.io/bgfx/" },
                { text: "bgfx examples", href: "https://bkaradzic.github.io/bgfx/examples.html" },
                { text: "bgfx on GitHub", href: "https://github.com/bkaradzic/bgfx" },
            ],
        },
        {
            name: "sokol",
            description: "Single-header C libraries for windows, graphics and audio",
            tier: "romeo",
            note: "for opening the window and managing events such as input",
            links: [
                { text: "sokol on GitHub", href: "https://github.com/floooh/sokol" },
                { text: "sokol samples", href: "https://floooh.github.io/sokol-html5/" },
                { text: "sokol samples source", href: "https://github.com/floooh/sokol-samples" },
            ],
        },
        {
            name: "miniquad",
            description: "Minimal cross-platform Rust rendering library",
            tier: "romeo",
            note: "for opening the window and managing events such as input",
            links: [
                { text: "miniquad on GitHub", href: "https://github.com/not-fl3/miniquad" },
                { text: "miniquad documentation", href: "https://docs.rs/miniquad" },
            ],
        },
        {
            name: "Metal",
            description: "Apple's graphics library",
            tier: "romeo",
            links: [
                { text: "Metal overview", href: "https://developer.apple.com/metal/" },
                { text: "Metal documentation", href: "https://developer.apple.com/documentation/metal" },
            ],
        },
        {
            name: "Software renderer",
            description: "Drawing every pixel yourself on the CPU",
            tier: "sierra",
            links: [
                { text: "tinyrenderer: build a software renderer in a few hundred lines", href: "https://github.com/ssloy/tinyrenderer/wiki" },
                { text: "Scratchapixel", href: "https://www.scratchapixel.com/" },
            ],
        },
        {
            name: "Bare metal",
            description: "Drawing straight to the hardware, no OS",
            tier: "sierra",
            links: [
                { text: "Hack Club Genesis", href: "https://genesis.hackclub.com" },
                { text: "Writing an OS in Rust", href: "https://os.phil-opp.com/" },
                { text: "OSDev Wiki: Drawing in a linear framebuffer", href: "https://wiki.osdev.org/Drawing_In_a_Linear_Framebuffer" },
                { text: "OSDev Wiki: VGA hardware", href: "https://wiki.osdev.org/VGA_Hardware" },
            ],
        },
        {
            name: "WebGL",
            description: "OpenGL in the browser",
            tier: "romeo",
            links: [
                { text: "WebGL Fundamentals", href: "https://webglfundamentals.org/" },
                { text: "MDN WebGL API", href: "https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API" },
            ],
        },
        {
            name: "LWJGL",
            description: "Java bindings for OpenGL, Vulkan, GLFW and more",
            tier: "romeo",
            note: "for opening the window and managing events such as input",
            links: [
                { text: "LWJGL website", href: "https://www.lwjgl.org/" },
                { text: "LWJGL getting started guide", href: "https://www.lwjgl.org/guide" },
                { text: "LWJGL Javadoc", href: "https://javadoc.lwjgl.org/" },
            ],
        },
        {
            name: "Silk.NET",
            description: "C# bindings for OpenGL, Vulkan, GLFW and more",
            tier: "romeo",
            note: "for opening the window and managing events such as input",
            links: [
                { text: "Silk.NET website", href: "https://dotnet.github.io/Silk.NET/" },
                { text: "Silk.NET OpenGL tutorial", href: "https://dotnet.github.io/Silk.NET/docs/opengl/c1/1-hello-window.html" },
                { text: "Silk.NET documentation", href: "https://dotnet.github.io/Silk.NET/docs/" },
            ],
        },
        {
            name: "OpenTK",
            description: "C# bindings for OpenGL, OpenAL and windowing",
            tier: "romeo",
            note: "for opening the window and managing events such as input",
            links: [
                { text: "OpenTK website", href: "https://opentk.net/" },
                { text: "OpenTK tutorials", href: "https://opentk.net/learn/index.html" },
                { text: "OpenTK API reference", href: "https://opentk.net/api/index.html" },
            ],
        },
        {
            name: "go-gl",
            description: "Go bindings for OpenGL and GLFW",
            tier: "romeo",
            note: "for opening the window and managing events such as input",
            links: [
                { text: "go-gl OpenGL bindings", href: "https://github.com/go-gl/gl" },
                { text: "go-gl GLFW bindings", href: "https://github.com/go-gl/glfw" },
                { text: "go-gl examples", href: "https://github.com/go-gl/example" },
            ],
        },
        {
            name: "moderngl",
            description: "Python wrapper for modern OpenGL",
            tier: "romeo",
            links: [
                { text: "moderngl documentation", href: "https://moderngl.readthedocs.io/" },
                { text: "moderngl on GitHub", href: "https://github.com/moderngl/moderngl" },
            ],
        },
        {
            name: "winit",
            description: "Rust library for windows and input",
            tier: "window",
            note: "for opening the window and managing events such as input",
            links: [
                { text: "winit documentation", href: "https://docs.rs/winit" },
                { text: "winit on GitHub", href: "https://github.com/rust-windowing/winit" },
            ],
        },
        {
            name: "glow",
            description: "Rust bindings for OpenGL, OpenGL ES and WebGL",
            tier: "romeo",
            links: [
                { text: "glow documentation", href: "https://docs.rs/glow" },
                { text: "glow on GitHub", href: "https://github.com/grovesNL/glow" },
            ],
        },
    ];

    const roadmaps = [
        {
            name: "JavaScript canvas game",
            steps: [
                "Put a canvas on a page",
                "Draw some rectangles",
                "Set up a game loop",
                "Keyboard and mouse input",
                "Move a player around",
                "Collisions",
                "Sprites and animation",
                "Sound and menus",
                "Polish and ship it!",
            ],
        },
        {
            name: "OpenGL game",
            steps: [
                "Open a window",
                "Load OpenGL and clear the screen",
                "Your first triangle",
                "Write some shaders",
                "Textures",
                "Transforms and a camera",
                "Load models",
                "Lighting",
                "Input and gameplay",
                "Polish and ship it!",
            ],
        },
        {
            name: "Vulkan game",
            steps: [
                "Open a window",
                "Instance and device",
                "Swapchain",
                "Pipeline and shaders",
                "Your first triangle",
                "Vertex and uniform buffers",
                "Descriptors and textures",
                "Depth buffer and camera",
                "Models and lighting",
                "Input and gameplay",
                "Polish and ship it!",
            ],
        },
        {
            name: "Software rendered game",
            steps: [
                "Get a pixel buffer on screen",
                "Draw pixels and lines",
                "Fill in triangles",
                "Vectors and matrices",
                "Project 3D onto the screen",
                "Depth buffer",
                "Shading and textures",
                "Camera and input",
                "Gameplay",
                "Polish and ship it!",
            ],
        },
    ];

    let roadmap = $state(0);
    let list = $state<HTMLOListElement>();
    let arrows = $state<string[]>([]);

    function draw() {
        if (!list) return;
        const boxes = [...list.children] as HTMLElement[];
        arrows = boxes.slice(1).map((next, index) => {
            const box = boxes[index];
            const x = box.offsetLeft;
            const y = box.offsetTop;
            const w = box.offsetWidth;
            const h = box.offsetHeight;
            if (next.offsetTop === y)
                return `M${x + w + 4} ${y + h / 2}H${next.offsetLeft - 4}m-6 -6l6 6l-6 6`;
            const middle = (y + h + next.offsetTop) / 2;
            const end_x = next.offsetLeft + next.offsetWidth / 2;
            return `M${x + w / 2} ${y + h + 4}V${middle}H${end_x}V${next.offsetTop - 4}m-6 -6l6 6l6 -6`;
        });
    }

    $effect(() => {
        void roadmap;
        if (!list) return;
        const observer = new ResizeObserver(draw);
        observer.observe(list);
        draw();
        return () => observer.disconnect();
    });

    const misc = [
        {
            text: "Calculate normal of a single triangle in 3D space",
            href: "https://stackoverflow.com/questions/19350792/calculate-normal-of-a-single-triangle-in-3d-space",
        },
    ];
</script>

<SiteHead
    title="Hack Club Mipmap Docs"
    description="Overview of all the important graphics concepts, with links to a bunch of resources for common libraries like SDL, GLFW, OpenGL, Vulkan, Direct3D 12 and wgpu."
    path="/docs"
/>

<nav aria-label="Site">
    <a class="home" href="/">Home</a>
</nav>

<main>
    <header class="card title">
        <p class="page-title">Mipmap Docs</p>
    </header>
    <div class="docs">
        <article class="card markdown">
            {@html data.html}
        </article>
        <section class="card wide">
            <h2>Rough Roadmaps</h2>
            <div class="picker">
                <button
                    type="button"
                    aria-label="Previous roadmap"
                    onclick={() => (roadmap = (roadmap + roadmaps.length - 1) % roadmaps.length)}
                >
                    <svg viewBox="0 0 24 12" aria-hidden="true">
                        <path d="M23 6H3M8 1L3 6l5 5" />
                    </svg>
                </button>
                <div class="dots" role="radiogroup" aria-label="Roadmap">
                    {#each roadmaps as { name }, index}
                        <input
                            type="radio"
                            name="roadmap"
                            value={index}
                            aria-label={name}
                            title={name}
                            bind:group={roadmap}
                        />
                    {/each}
                </div>
                <button
                    type="button"
                    aria-label="Next roadmap"
                    onclick={() => (roadmap = (roadmap + 1) % roadmaps.length)}
                >
                    <svg viewBox="0 0 24 12" aria-hidden="true">
                        <path d="M1 6h20M16 1l5 5-5 5" />
                    </svg>
                </button>
            </div>
            <h3 aria-live="polite">{roadmaps[roadmap].name}</h3>
            <div class="steps">
                <svg aria-hidden="true">
                    {#each arrows as arrow}
                        <path d={arrow} />
                    {/each}
                </svg>
                <ol bind:this={list}>
                    {#each roadmaps[roadmap].steps as text}
                        <li>{text}</li>
                    {/each}
                </ol>
            </div>
        </section>
    </div>
    <div class="resources">
        {#if video}
            <section class="card wide">
                <h2>Video</h2>
                <p>Overview of all the important graphics concepts</p>
                <iframe
                    src={video}
                    title="Overview of all the important graphics concepts"
                    allow="accelerometer; encrypted-media; gyroscope; picture-in-picture"
                    referrerpolicy="strict-origin-when-cross-origin"
                    loading="lazy"
                    allowfullscreen
                ></iframe>
            </section>
        {/if}
        <div class="section">
            <MipmapButton onclick={() => (picking = true)}>Which library to choose?</MipmapButton>
        </div>
        <header class="card">
            <h2>More Specific Resources</h2>
        </header>
        <div class="libraries">
            {#each libraries as { name, note, description, tier, links }}
                <section class="card">
                    <h3>{name}{#if note} ({note}){/if}</h3>
                    <p class="tier">{tier_labels[tier]}</p>
                    <p class="description">{description}</p>
                    {#if links.length}
                        <ul>
                            {#each links as { text, href }}
                                <li>
                                    <a {href} target="_blank" rel="noopener">{text}</a>
                                </li>
                            {/each}
                        </ul>
                    {/if}
                </section>
            {/each}
        </div>
        <header class="card section">
            <h2>Other things that might help you out</h2>
        </header>
        <section class="card wide">
            <ul>
                {#each misc as { text, href }}
                    <li>
                        <a {href} target="_blank" rel="noopener">{text}</a>
                    </li>
                {/each}
            </ul>
        </section>
    </div>
</main>

{#if picking}
    <LibraryPicker {libraries} onclose={() => (picking = false)} />
{/if}

<style>
    main {
        max-width: 100rem;
        margin: 0 auto;
        display: grid;
        grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
        align-items: start;
        gap: 1rem 2rem;
        padding: 4.5rem 1rem 3rem;
        color: #ead7c2;
    }

    .title {
        grid-column: 1 / -1;
    }

    .docs,
    .resources {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 1rem;
        min-width: 0;
    }

    .markdown {
        width: 100%;
        min-width: 0;
    }

    .markdown > :global(:first-child) {
        margin-top: 0;
    }

    .markdown > :global(:last-child) {
        margin-bottom: 0;
    }

    .markdown :global(h1),
    .markdown :global(h2),
    .markdown :global(h3) {
        margin: 1.5rem 0 0.5rem;
        font-weight: 700;
    }

    .markdown :global(p),
    .markdown :global(ul),
    .markdown :global(ol),
    .markdown :global(pre) {
        margin: 0 0 1rem;
    }

    .markdown :global(ul),
    .markdown :global(ol) {
        padding-left: 1.25rem;
    }

    .markdown :global(code) {
        padding: 0.1em 0.3em;
        background: #4f2f1f;
        border-radius: 4px;
    }

    .markdown :global(pre) {
        overflow-x: auto;
        padding: 0.75rem 1rem;
        background: #4f2f1f;
        border-radius: 7px;
    }

    .markdown :global(pre code) {
        padding: 0;
    }

    .markdown :global(img) {
        max-width: 100%;
    }

    .markdown :global(a) {
        color: #c8d5f3;
    }

    .markdown :global(a:focus-visible) {
        color: #ead7c2;
    }

    @media (hover: hover) {
        .markdown :global(a:hover) {
            color: #ead7c2;
        }
    }

    @media (max-width: 900px) {
        main {
            grid-template-columns: minmax(0, 1fr);
        }
    }

    .home {
        position: absolute;
        top: 1rem;
        left: 1rem;
        padding: 0.5rem 1rem;
        background-color: #4f2f1f;
        color: #ead7c2;
        font-weight: 700;
        text-decoration: none;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .home:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .home:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    .card {
        box-sizing: border-box;
        width: fit-content;
        padding: 1.25rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
    }

    .section {
        margin-top: 2rem;
    }

    .libraries {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        width: 100%;
    }

    .libraries .card {
        width: 100%;
    }

    .description {
        margin: 0.25rem 0 0;
        opacity: 0.85;
    }

    .description + ul {
        margin-top: 0.5rem;
    }

    .tier {
        display: inline-block;
        margin: 0.35rem 0 0;
        padding: 0.1rem 0.6rem;
        background-color: #4f2f1f;
        border-radius: 999px;
        font-size: 0.85rem;
        font-weight: 700;
    }

    h3 {
        margin: 0;
        font-size: 1.25rem;
        font-weight: 700;
    }

    ul {
        margin: 0;
        padding-left: 1.25rem;
    }

    a {
        color: #c8d5f3;
    }

    a:not(.home):focus-visible {
        color: #ead7c2;
    }

    @media (hover: hover) {
        a:not(.home):hover {
            color: #ead7c2;
        }
    }

    .wide {
        width: 100%;
    }

    .picker {
        display: flex;
        justify-content: center;
        align-items: center;
        gap: 1rem;
        margin-bottom: 1rem;
    }

    .picker button {
        display: grid;
        place-items: center;
        min-width: 2.5rem;
        min-height: 2.5rem;
        padding: 0.25rem;
        background: none;
        border: none;
        cursor: pointer;
    }

    .picker svg {
        width: 1.75rem;
        fill: none;
        stroke: #d6ab70;
        stroke-width: 2;
        stroke-linecap: round;
        stroke-linejoin: round;
        transition: stroke 0.3s;
    }

    .picker button:focus-visible {
        outline: 2px solid #ead7c2;
    }

    .picker button:focus-visible svg {
        stroke: #ead7c2;
    }

    @media (hover: hover) {
        .picker button:hover svg {
            stroke: #ead7c2;
        }
    }

    @media (max-width: 900px) {
        .picker {
            justify-content: flex-start;
        }
    }

    .dots {
        display: flex;
        gap: 0.75rem;
    }

    .dots input {
        width: 1rem;
        height: 1rem;
        margin: 0;
        box-sizing: border-box;
        appearance: none;
        background-color: #4f2f1f;
        border: 2px solid #d6ab70;
        border-radius: 50%;
        cursor: pointer;
    }

    .dots input:checked {
        background-color: #ead7c2;
        border-color: #ead7c2;
    }

    .dots input:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: 2px;
    }

    @media (hover: hover) {
        .dots input:not(:checked):hover {
            background-color: #d6ab70;
        }
    }

    .steps {
        position: relative;
        margin-top: 1rem;
        container: roadmap / inline-size;
    }

    .steps svg {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        overflow: visible;
        fill: none;
        stroke: #d6ab70;
        stroke-width: 2;
        stroke-linecap: round;
        stroke-linejoin: round;
    }

    .steps ol {
        display: grid;
        grid-template-columns: repeat(auto-fill, 9rem);
        gap: 2.5rem;
        margin: 0;
        padding: 0;
        list-style: none;
    }

    @container roadmap (min-width: 18.5rem) and (max-width: 20.5rem) {
        .steps ol {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }
    }

    @container roadmap (max-width: 9rem) {
        .steps ol {
            grid-template-columns: minmax(0, 1fr);
        }
    }

    .steps li {
        box-sizing: border-box;
        aspect-ratio: 1;
        display: grid;
        place-items: center;
        padding: 0.5rem;
        background: #5f4838;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        font-size: 1.25rem;
        line-height: 1.2;
        text-align: center;
        overflow-wrap: break-word;
    }

    .page-title {
        margin: 0;
        font-size: 2rem;
        font-weight: 700;
    }

    header h2 {
        margin: 0;
        font-size: 2rem;
    }

    h2 {
        margin: 0 0 0.5rem;
        font-size: 1.5rem;
        font-weight: 700;
    }

    p {
        margin: 0 0 1rem;
    }

    iframe {
        display: block;
        width: 100%;
        aspect-ratio: 16 / 9;
        border: 0;
        border-radius: 7px;
    }
</style>
