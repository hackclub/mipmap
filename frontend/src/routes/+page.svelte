<script lang="ts">
    import { onMount } from "svelte";
    import MipmapLogo from "$lib/landing/MipmapLogo.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import CogField from "$lib/landing/CogField.svelte";
    import FaqSection from "$lib/landing/FaqSection.svelte";
    import TextBlock from "$lib/landing/TextBlock.svelte";
    import SiteHead, { site_url } from "$lib/SiteHead.svelte";
    import { grid } from "$lib/cog_layout";
    import { grow_field } from "$lib/landing/cog_field";

    let viewport_width = $state(1024);
    let viewport_height = $state(768);
    let scroll_y = $state(0);
    let footer_width = $state(0);
    let footer_height = $state(0);
    let first_container_height = $state(0);
    let left_content_height = $state(0);
    let left_heading_height = $state(0);
    let left_actions_height = $state(0);
    let right_content_height = $state(0);
    let left_box_height = $state(0);
    let right_box_height = $state(0);
    let description_box_height = $state(0);
    let faq_height = $state(632);
    let ready = $state(false);
    let field = $state(false);
    let logo_width = $state(768);

    const logo_aspect = 84.666664 / 66.145827;
    const logo_top = 24;

    const logo = $derived({
        left: (viewport_width - logo_width) / 2,
        width: logo_width,
        height: logo_width / logo_aspect,
    });

    const logo_blocked = $derived({
        left: logo.left,
        right: logo.left + logo.width,
        top: -Infinity,
        bottom: logo_top + logo.height,
    });
    const container_width = $derived(
        grid *
            Math.max(
                1,
                Math.round(
                    Math.min(960, Math.max(logo.width, 0.8 * viewport_width)) /
                        grid,
                ),
            ),
    );
    const container_x = $derived(
        logo.left + (logo.width - container_width) / 2,
    );
    const container_y = $derived(logo_top + logo.height + 48);
    const pair_gap = $derived(
        ((container_width / grid) % 2 === 0 ? 2 : 1) * grid,
    );
    const pair_width = $derived((container_width - pair_gap) / 2);
    const pair_y = $derived(container_y + first_container_height + 144);
    const pair_height = $derived(
        Math.max(left_content_height, right_content_height),
    );
    const learn_shift = $derived.by(() => {
        const below = pair_height - left_heading_height;
        const natural =
            left_content_height - left_heading_height - left_actions_height;
        return Math.max(0, (below - left_actions_height) / 2 - natural);
    });

    let email = $state("");

    function sign_up(e: SubmitEvent) {
        e.preventDefault();
        window.location.href = `/login?email=${encodeURIComponent(email.trim())}`;
    }

    const pair_box_height = $derived(
        Math.max(left_box_height, right_box_height),
    );
    const description_y = $derived(pair_y + pair_box_height + 144);
    const first_box = $derived({
        left: container_x,
        right: container_x + container_width,
        top: container_y,
        bottom: container_y + first_container_height,
    });
    const left_box = $derived({
        left: container_x,
        right: container_x + pair_width,
        top: pair_y,
        bottom: pair_y + pair_box_height,
    });
    const right_box = $derived({
        left: container_x + pair_width + pair_gap,
        right: container_x + container_width,
        top: pair_y,
        bottom: pair_y + pair_box_height,
    });
    const description_box = $derived({
        left: container_x,
        right: container_x + container_width,
        top: description_y,
        bottom: description_y + description_box_height,
    });
    const faq_y = $derived(description_y + description_box_height + 96);
    const page_height = $derived(
        Math.max(3 * viewport_height, faq_y + faq_height + 160),
    );
    const cog_field_height = $derived(page_height + footer_height + 48);
    const footer_blocked = $derived({
        left: (viewport_width - footer_width) / 2 - 20,
        right: (viewport_width + footer_width) / 2 + 20,
        top: page_height - 20,
        bottom: page_height + footer_height + 24,
    });

    const cogs = $derived(
        field && footer_width > 0 && footer_height > 0
            ? grow_field(viewport_width, cog_field_height, [
                  logo_blocked,
                  footer_blocked,
              ])
            : [],
    );

    function measure() {
        viewport_width = document.documentElement.clientWidth;
        viewport_height = window.innerHeight;
    }

    onMount(() => {
        try {
            if (!localStorage.getItem("visited")) {
                fetch("/api/visit", { method: "POST" })
                    .then((r) => r.ok && localStorage.setItem("visited", "1"))
                    .catch(() => {});
            }
        } catch {}
        const code = new URLSearchParams(location.search).get("ref");
        if (code && /^[A-Za-z0-9]{1,32}$/.test(code)) {
            document.cookie = `ref=${code}; Path=/; Max-Age=2592000; SameSite=Lax; Secure`;
        }
        measure();
        let reveal = window.requestAnimationFrame(() => {
            reveal = window.requestAnimationFrame(() => {
                ready = true;
                reveal = window.requestAnimationFrame(() => (field = true));
            });
        });
        let scroll_frame = 0;
        const update_scroll = () => {
            if (scroll_frame) return;
            scroll_frame = window.requestAnimationFrame(() => {
                scroll_y = window.scrollY;
                scroll_frame = 0;
            });
        };
        scroll_y = window.scrollY;
        window.addEventListener("scroll", update_scroll, { passive: true });
        return () => {
            window.cancelAnimationFrame(reveal);
            window.removeEventListener("scroll", update_scroll);
            if (scroll_frame) window.cancelAnimationFrame(scroll_frame);
        };
    });

    const description =
        "Hack Club Mipmap is a program for teenagers by the nonprofit Hack Club where you can ship a game made without an engine to progress your tech tree and unlock free prizes!";
    const structured = JSON.stringify({
        "@context": "https://schema.org",
        "@type": "WebSite",
        name: "Hack Club Mipmap",
        url: `${site_url}/`,
        description,
        publisher: {
            "@type": "Organization",
            name: "Hack Club",
            url: "https://hackclub.com/",
        },
    }).replace(/</g, "\\u003c");
</script>

<svelte:window onresize={measure} />

<svelte:head>
    <link rel="preload" as="image" href="/logo.svg" fetchpriority="high" />
    {#if site_url}
        {@html `<script type="application/ld+json">${structured}</script>`}
    {/if}
</svelte:head>

<SiteHead
    title="Hack Club Mipmap - Make a game without an engine, earn prizes down your Tech Tree!"
    {description}
    path="/"
/>

<div class="page">
    <div class="field" style:height="{cog_field_height}px">
        <CogField {cogs} {scroll_y} {viewport_height} {footer_blocked} />
    </div>

    <div style:visibility={ready ? "visible" : "hidden"}>
        <TextBlock
            x={container_x}
            y={container_y}
            width={container_width}
            {cogs}
            obstacles={[left_box, right_box, description_box]}
            bind:box_height={first_container_height}
        >
            <h1>Make a game without an engine, earn prizes down your Tech Tree!</h1>
        </TextBlock>

        <TextBlock
            x={container_x}
            y={pair_y}
            width={pair_width}
            {cogs}
            min_height={pair_height}
            obstacles={[first_box, right_box, description_box]}
            bind:box_height={left_box_height}
            bind:content_height={left_content_height}
        >
            <div class="pair" bind:clientHeight={left_heading_height}>
                <h2>Don't know where to start?</h2>
            </div>
            <div
                class="actions learn"
                bind:clientHeight={left_actions_height}
                style:transform="translateY({learn_shift}px)"
            >
                <a class="guide" href="/docs">
                    <span class="icon">
                        <svg viewBox="0 0 16 16" aria-hidden="true">
                            <path
                                fill-rule="evenodd"
                                d="M8.316 2.07a.75.75 0 00-.632 0l-7 3.25a.75.75 0 000 1.36l1.434.666A.746.746 0 002 7.750V11a.75.75 0 00.158.46L2.75 11l-.592.46.001.002.001.001.003.004.008.01a1.882 1.882 0 00.103.12c.068.076.165.178.292.299.254.24.63.555 1.132.866C4.706 13.388 6.217 14 8.250 14c2.037 0 3.440-.615 4.345-1.266a5.320 5.320 0 00.977-.902 3.916 3.916 0 00.322-.448l.007-.012.003-.004v-.002h.001c0-.001 0-.002-.655-.366l.655.365A.754.754 0 0014 11V7.750a.747.747 0 00-.118-.404l1.434-.666a.75.75 0 000-1.360l-7-3.250zM12.5 7.988L8.316 9.930a.75.75 0 01-.632 0L3.5 7.988v2.723a5.585 5.585 0 00.990.776c.804.5 2.043 1.013 3.760 1.013 1.713 0 2.810-.51 3.468-.984a3.812 3.812 0 00.782-.745V7.988zM8 8.423L2.781 6 8 3.577 13.219 6 8 8.423z"
                                clip-rule="evenodd"
                            />
                        </svg>
                    </span>
                    <span class="caption">Learn here!</span>
                </a>
            </div>
        </TextBlock>

        <TextBlock
            x={container_x + pair_width + pair_gap}
            y={pair_y}
            width={pair_width}
            {cogs}
            min_height={pair_height}
            obstacles={[first_box, left_box, description_box]}
            bind:box_height={right_box_height}
            bind:content_height={right_content_height}
        >
            <div class="pair">
                <h2>Wanna get started?</h2>
            </div>
            <div class="start">
                <form class="signup" onsubmit={sign_up}>
                    <label for="signup-email">Sign up with your email!</label>
                    <div class="email">
                        <input
                            id="signup-email"
                            type="email"
                            autocomplete="email"
                            placeholder="heidi@hackclub.com"
                            maxlength={254}
                            size={1}
                            required
                            bind:value={email}
                        />
                        <button type="submit" aria-label="Continue">
                            <svg viewBox="0 0 16 16" aria-hidden="true">
                                <path
                                    d="M8.47 2.47a.75.75 0 0 1 1.06 0l5 5a.75.75 0 0 1 0 1.06l-5 5a.75.75 0 1 1-1.06-1.06l3.72-3.72H2a.75.75 0 0 1 0-1.5h10.19L8.47 3.53a.75.75 0 0 1 0-1.06Z"
                                />
                            </svg>
                        </button>
                    </div>
                </form>
                <div class="or" aria-hidden="true">OR</div>
                <div class="actions">
                    <MipmapButton
                        type="button"
                        onclick={() => (window.location.href = "/login")}
                    >
                        Log in with Hack Club Auth
                    </MipmapButton>
                </div>
            </div>
        </TextBlock>

        <TextBlock
            x={container_x}
            y={description_y}
            width={container_width}
            {cogs}
            variant="body"
            min_height={240}
            obstacles={[first_box, left_box, right_box]}
            bind:box_height={description_box_height}
        >
            <p>
                Ever wondered how video games work under the hood? Here's your
                chance to find out! Hack Club Mipmap is a program for teenagers
                (13-18 inclusive) by the nonprofit <a href="https://hackclub.com"
                    >Hack Club</a
                > where you can ship a game made without an engine to progress your tech
                tree and unlock free prizes!
            </p>
            <p>&ThickSpace;</p>
            <p>
                Got no idea how to even get started? That's totally fine! Mipmap's
                got a video designed to get you up to speed with the
                basics in one sitting. After that, you know exactly what
                to look for when you're going through guides for the specific
                graphics library you're using. Just click the "Learn here!" button
                above!
            </p>
        </TextBlock>

        <FaqSection
            top={faq_y}
            {viewport_width}
            {pair_width}
            {cogs}
            obstacles={[first_box, left_box, right_box, description_box]}
            bind:height={faq_height}
        />
    </div>

    <div class="content" style:height="{page_height}px"></div>

    <footer
        class="site-footer"
        bind:clientWidth={footer_width}
        bind:clientHeight={footer_height}
    >
        <div class="footer-links">
            <a href="https://hackclub.com/">Hack Club</a>
            <a href="https://hackclub.com/privacy-and-terms"
                >Terms &amp; Privacy Policy</a
            >
            <a href="https://forms.hackclub.com/bounty">Fulfillment Bounty</a>
            <a href="https://security.hackclub.com/">Security</a>
        </div>
        <p class="made-with">
            Made with <span class="heart" aria-label="love">&hearts;</span> by
            <a href="https://hackclub.enterprise.slack.com/team/U08LEHUPSCR"
                >Brendan</a
            >
            @ <a href="https://hackclub.com/">Hack Club</a>
        </p>
    </footer>

    <div
        class="logo"
        style:top="{logo_top}px"
        style:--aspect={logo_aspect}
        bind:clientWidth={logo_width}
    >
        <MipmapLogo />
    </div>
</div>

<style>
    .page {
        position: relative;
        overflow-x: clip;
    }

    .site-footer {
        position: relative;
        box-sizing: border-box;
        width: fit-content;
        max-width: calc(100% - 2rem);
        margin: 0 auto 3rem;
        padding: 1.25rem 1.5rem;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.75rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        text-align: center;
    }

    .footer-links {
        display: flex;
        flex-wrap: wrap;
        justify-content: center;
        gap: 0.75rem 1.5rem;
    }

    .site-footer a {
        text-decoration: none;
    }

    .made-with {
        margin: 0;
        color: #ead7c2;
    }

    .heart {
        color: #f0788a;
    }

    .field {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
    }

    .logo {
        position: absolute;
        width: min(75%, calc(70svh * var(--aspect)));
        left: calc((100% - min(75%, calc(70svh * var(--aspect)))) / 2);
    }

    @media (max-width: 600px) {
        .logo {
            width: min(90%, calc(70svh * var(--aspect)));
            left: calc((100% - min(90%, calc(70svh * var(--aspect)))) / 2);
        }
    }

    .pair {
        font-size: clamp(1rem, 4vw, 3.5rem);
        font-weight: 400;
        overflow-wrap: break-word;
    }

    .actions {
        margin-top: 1.25rem;
    }

    .actions :global(button) {
        max-width: 100%;
    }

    .learn {
        display: flex;
        justify-content: center;
    }

    .guide {
        display: flex;
        flex-direction: column;
        width: clamp(9rem, 24vw, 16rem);
        max-width: 100%;
        overflow: hidden;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        text-align: center;
        text-decoration: none;
    }

    .icon {
        display: grid;
        place-items: center;
        aspect-ratio: 2.25;
        min-height: 0;
        background: #5f4838;
    }

    .icon svg {
        width: 60%;
        height: 60%;
        fill: currentColor;
    }

    .caption {
        padding: 0.5rem 0.25rem;
        background: #4f2f1f;
        font-size: clamp(0.95rem, 2.6vw, 2.25rem);
        white-space: nowrap;
        font-weight: 700;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .guide:focus-visible {
        outline: none;
    }

    .guide:focus-visible .caption {
        background: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .guide:hover .caption {
            background: #ead7c2;
            color: #4f2f1f;
        }
    }

    .start {
        display: grid;
        width: fit-content;
        max-width: 100%;
        margin: 0 auto;
    }

    .start .actions :global(button) {
        width: 100%;
    }

    .or {
        margin-top: 1rem;
        font-size: clamp(0.8rem, 1.4vw, 1.1rem);
        font-weight: 700;
        text-align: center;
        opacity: 0.5;
    }

    .start .actions {
        margin-top: 1rem;
    }

    .signup {
        display: grid;
        gap: 0.5rem;
        margin-top: 1.25rem;
        font-size: clamp(1rem, 2vw, 1.5rem);
    }

    .email {
        position: relative;
    }

    .email input {
        box-sizing: border-box;
        width: 100%;
        padding: 0.6rem 3.25rem 0.6rem 0.9rem;
        background: #4f2f1f;
        color: #ead7c2;
        border: none;
        border-radius: 0;
        font: inherit;
    }

    .email input::placeholder {
        color: inherit;
        opacity: 0.6;
    }

    .email input:focus-visible {
        outline: 2px solid #ead7c2;
        outline-offset: -2px;
    }

    .email button {
        position: absolute;
        top: 0;
        right: 0;
        bottom: 0;
        display: grid;
        place-items: center;
        width: 3rem;
        padding: 0;
        background: none;
        border: none;
        color: #ead7c2;
        font-size: inherit;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .email button:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
        outline: none;
    }

    @media (hover: hover) {
        .email button:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    .email svg {
        width: 1.1em;
        height: 1.1em;
        fill: currentColor;
    }

    @media (max-width: 600px) {
        .actions :global(button) {
            padding: 0.5rem 0.75rem;
            font-size: 1rem;
        }

        .email input {
            padding: 0.5rem 2.25rem 0.5rem 0.5rem;
        }

        .email button {
            width: 2rem;
        }
    }
</style>
