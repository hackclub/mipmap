<script lang="ts">
    import SiteHead from "$lib/SiteHead.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import LibraryPicker, { type Library } from "$lib/docs/LibraryPicker.svelte";

    function embedUrl(value: string | undefined) {
        try {
            const url = new URL(value ?? "");
            const id =
                url.hostname === "youtu.be"
                    ? url.pathname.slice(1)
                    : url.searchParams.get("v");
            return id && /^[\w-]{1,32}$/.test(id)
                ? `https://www.youtube-nocookie.com/embed/${id}`
                : null;
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
            note: "for doing window stuff",
            links: [
                { text: "SDL website", href: "https://www.libsdl.org/" },
                { text: "SDL3 documentation", href: "https://wiki.libsdl.org/SDL3/FrontPage" },
            ],
        },
        {
            name: "GLFW",
            description: "Lightweight windows and input for OpenGL and Vulkan",
            note: "for doing window stuff",
            links: [
                { text: "GLFW website", href: "https://www.glfw.org/" },
                { text: "GLFW quick guide", href: "https://www.glfw.org/docs/latest/quick_guide.html" },
            ],
        },
        {
            name: "OpenGL",
            description: "Cross-platform graphics library",
            links: [
                { text: "LearnOpenGL", href: "https://learnopengl.com/" },
                { text: "docs.gl API reference", href: "https://docs.gl/" },
                { text: "Khronos OpenGL Wiki", href: "https://www.khronos.org/opengl/wiki/" },
            ],
        },
        {
            name: "Vulkan",
            description: "Low-level cross-platform graphics library",
            links: [
                { text: "How to Vulkan", href: "https://howtovulkan.com/" },
                { text: "Vulkan Guide", href: "https://vkguide.dev/" },
                { text: "Official Vulkan documentation", href: "https://docs.vulkan.org/" },
            ],
        },
        {
            name: "Direct3D",
            description: "Microsoft's graphics library",
            links: [
                { text: "Direct3D 12 programming guide", href: "https://learn.microsoft.com/en-us/windows/win32/direct3d12/directx-12-programming-guide" },
                { text: "Learning DirectX 12 (3D Game Engine Programming)", href: "https://www.3dgep.com/learning-directx-12-1/" },
            ],
        },
        {
            name: "wgpu",
            description: "Rust graphics library based on WebGPU",
            links: [
                { text: "wgpu website", href: "https://wgpu.rs/" },
                { text: "Learn Wgpu", href: "https://sotrh.github.io/learn-wgpu/" },
                { text: "WebGPU Fundamentals", href: "https://webgpufundamentals.org/" },
            ],
        },
        {
            name: "bgfx",
            description: "C/C++ graphics library that runs on top of every major API",
            links: [
                { text: "bgfx documentation", href: "https://bkaradzic.github.io/bgfx/" },
                { text: "bgfx examples", href: "https://bkaradzic.github.io/bgfx/examples.html" },
                { text: "bgfx on GitHub", href: "https://github.com/bkaradzic/bgfx" },
            ],
        },
        {
            name: "sokol",
            description: "Single-header C libraries for windows, graphics and audio",
            links: [
                { text: "sokol on GitHub", href: "https://github.com/floooh/sokol" },
                { text: "sokol samples", href: "https://floooh.github.io/sokol-html5/" },
                { text: "sokol samples source", href: "https://github.com/floooh/sokol-samples" },
            ],
        },
        {
            name: "miniquad",
            description: "Minimal cross-platform Rust rendering library",
            links: [
                { text: "miniquad on GitHub", href: "https://github.com/not-fl3/miniquad" },
                { text: "miniquad documentation", href: "https://docs.rs/miniquad" },
            ],
        },
        {
            name: "Metal",
            description: "Apple's graphics library",
            links: [
                { text: "Metal overview", href: "https://developer.apple.com/metal/" },
                { text: "Metal documentation", href: "https://developer.apple.com/documentation/metal" },
            ],
        },
        {
            name: "Software renderer",
            description: "Drawing every pixel yourself on the CPU",
            links: [
                { text: "tinyrenderer: build a software renderer in a few hundred lines", href: "https://github.com/ssloy/tinyrenderer/wiki" },
                { text: "Scratchapixel", href: "https://www.scratchapixel.com/" },
            ],
        },
        {
            name: "Bare metal",
            description: "Drawing straight to the hardware, no OS",
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
            links: [
                { text: "WebGL Fundamentals", href: "https://webglfundamentals.org/" },
                { text: "MDN WebGL API", href: "https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API" },
            ],
        },
        {
            name: "LWJGL",
            description: "Java bindings for OpenGL, Vulkan, GLFW and more",
            links: [
                { text: "LWJGL website", href: "https://www.lwjgl.org/" },
                { text: "LWJGL getting started guide", href: "https://www.lwjgl.org/guide" },
                { text: "LWJGL Javadoc", href: "https://javadoc.lwjgl.org/" },
            ],
        },
        {
            name: "Silk.NET",
            description: "C# bindings for OpenGL, Vulkan, GLFW and more",
            links: [
                { text: "Silk.NET website", href: "https://dotnet.github.io/Silk.NET/" },
                { text: "Silk.NET OpenGL tutorial", href: "https://dotnet.github.io/Silk.NET/docs/opengl/c1/1-hello-window.html" },
                { text: "Silk.NET documentation", href: "https://dotnet.github.io/Silk.NET/docs/" },
            ],
        },
        {
            name: "OpenTK",
            description: "C# bindings for OpenGL, OpenAL and windowing",
            links: [
                { text: "OpenTK website", href: "https://opentk.net/" },
                { text: "OpenTK tutorials", href: "https://opentk.net/learn/index.html" },
                { text: "OpenTK API reference", href: "https://opentk.net/api/index.html" },
            ],
        },
        {
            name: "go-gl",
            description: "Go bindings for OpenGL and GLFW",
            links: [
                { text: "go-gl OpenGL bindings", href: "https://github.com/go-gl/gl" },
                { text: "go-gl GLFW bindings", href: "https://github.com/go-gl/glfw" },
                { text: "go-gl examples", href: "https://github.com/go-gl/example" },
            ],
        },
        {
            name: "moderngl",
            description: "Python wrapper for modern OpenGL",
            links: [
                { text: "moderngl documentation", href: "https://moderngl.readthedocs.io/" },
                { text: "moderngl on GitHub", href: "https://github.com/moderngl/moderngl" },
            ],
        },
        {
            name: "winit",
            description: "Rust library for windows and input",
            links: [
                { text: "winit documentation", href: "https://docs.rs/winit" },
                { text: "winit on GitHub", href: "https://github.com/rust-windowing/winit" },
            ],
        },
        {
            name: "glow",
            description: "Rust bindings for OpenGL, OpenGL ES and WebGL",
            links: [
                { text: "glow documentation", href: "https://docs.rs/glow" },
                { text: "glow on GitHub", href: "https://github.com/grovesNL/glow" },
            ],
        },
    ];

    const misc = [
        {
            text: "Calculate normal of a single triangle in 3D space",
            href: "https://stackoverflow.com/questions/19350792/calculate-normal-of-a-single-triangle-in-3d-space",
        },
    ];
</script>

<SiteHead
    title="Docs - Hack Club Mipmap"
    description="Guides for Hack Club Mipmap."
    path="/docs"
/>

<a class="home" href="/">Home</a>

<main>
    <header class="card">
        <h1>Mipmap Docs</h1>
    </header>
    <section class="card wide">
        <h2>Video</h2>
        <p>Overview of all the important graphics concepts</p>
        {#if video}
            <iframe
                src={video}
                title="Overview of all the important graphics concepts"
                allow="accelerometer; encrypted-media; gyroscope; picture-in-picture"
                referrerpolicy="strict-origin-when-cross-origin"
                allowfullscreen
            ></iframe>
        {/if}
    </section>
    <div class="section">
        <MipmapButton onclick={() => (picking = true)}>Which library to choose?</MipmapButton>
    </div>
    <header class="card">
        <h2>More Specific Resources</h2>
    </header>
    <div class="libraries">
        {#each libraries as { name, note, description, links }}
            <section class="card">
                <h3>{name}{#if note} ({note}){/if}</h3>
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
        <h2>Misc</h2>
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
</main>

{#if picking}
    <LibraryPicker {libraries} onclose={() => (picking = false)} />
{/if}

<style>
    main {
        max-width: 48rem;
        margin: 0 auto;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 1rem;
        padding: 4.5rem 1rem 3rem;
        color: #ead7c2;
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

    .home:hover,
    .home:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
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
        opacity: 0.7;
    }

    .description + ul {
        margin-top: 0.5rem;
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
        color: #e8c58c;
    }

    .wide {
        width: 100%;
    }

    h1 {
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
