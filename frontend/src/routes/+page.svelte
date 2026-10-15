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
    title="Hack Club Mipmap - Make a game without an engine"
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
            <h1>Make a game without an engine, progress your tech tree!</h1>
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
            <div class="pair">
                <h2>Don't know where to start?</h2>
            </div>
            <div class="actions">
                <MipmapButton
                    type="button"
                    onclick={() => (window.location.href = "/docs")}
                >
                    Learn here!
                </MipmapButton>
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
            <div class="actions">
                <MipmapButton
                    type="button"
                    onclick={() => (window.location.href = "/login")}
                >
                    Log in with Hack Club Auth!
                </MipmapButton>
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

    @media (max-width: 600px) {
        .actions :global(button) {
            padding: 0.5rem 0.75rem;
            font-size: 1rem;
        }
    }
</style>
