<script lang="ts">
    import { onMount } from "svelte";
    import MipmapLogo from "$lib/MipmapLogo.svelte";
    import MipmapButton from "$lib/MipmapButton.svelte";
    import CogField from "$lib/CogField.svelte";
    import FaqSection from "$lib/FaqSection.svelte";
    import TextBlock from "$lib/TextBlock.svelte";
    import { grid } from "$lib/cog_layout";
    import { grow_field } from "$lib/cog_field";

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
    let now = $state(0);
    let ready = $state(false);

    const start_time = new Date("2026-10-07T16:00:00-04:00").getTime();
    const remaining_seconds = $derived(
        now === 0 ? 0 : Math.max(0, Math.ceil((start_time - now) / 1000)),
    );
    const countdown_active = $derived(remaining_seconds > 0);
    const countdown = $derived(
        [
            Math.floor(remaining_seconds / 3600),
            Math.floor((remaining_seconds % 3600) / 60),
            remaining_seconds % 60,
        ]
            .map(
                (part, index) =>
                    `${String(part).padStart(2, "0")}${["h", "m", "s"][index]}`,
            )
            .join(" "),
    );

    const logo_aspect = 84.666664 / 66.145827;
    const phone = $derived(viewport_width <= 600);
    const logo_top = $derived(phone && countdown_active ? 128 : 24);

    const logo = $derived.by(() => {
        const w = viewport_width;
        const v = viewport_height;
        const width = Math.min((phone ? 0.9 : 0.75) * w, 0.7 * v * logo_aspect);
        return {
            left: (w - width) / 2,
            width,
            height: width / logo_aspect,
        };
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
        Math.max(
            3 * viewport_height,
            faq_y + faq_height + 160,
        ),
    );
    const cog_field_height = $derived(page_height + footer_height + 48);
    const footer_blocked = $derived({
        left: (viewport_width - footer_width) / 2 - 20,
        right: (viewport_width + footer_width) / 2 + 20,
        top: page_height - 20,
        bottom: page_height + footer_height + 24,
    });

    const cogs = $derived(
        footer_width > 0 && footer_height > 0
            ? grow_field(viewport_width, cog_field_height, [logo_blocked, footer_blocked])
            : [],
    );

    function handle_login_click() {
        if (countdown_active) {
            window.alert("Mipmap hasn't started yet!");
            return;
        }
        window.location.href = "/login";
    }

    function measure() {
        viewport_width = document.documentElement.clientWidth;
        viewport_height = window.innerHeight;
    }

    onMount(() => {
        measure();
        now = Date.now();
        const reveal = window.requestAnimationFrame(() =>
            window.requestAnimationFrame(() => (ready = true)),
        );
        const interval = window.setInterval(() => (now = Date.now()), 1000);
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
            window.clearInterval(interval);
            window.removeEventListener("scroll", update_scroll);
            if (scroll_frame) window.cancelAnimationFrame(scroll_frame);
        };
    });
</script>

<svelte:window onresize={measure} />

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
    <h2>Don't know where to start?</h2>
    <div class="actions">
        <MipmapButton
            type="button"
            onclick={() =>
                window.alert(
                    "Whoops, this should bring you somewhere. Bug me in #mipmap!",
                )}
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
    <h2>Wanna get started?</h2>
    <div class="actions">
        <MipmapButton type="button" onclick={handle_login_click}>
            Log in!
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
        Ever wondered how video games work under the hood? Here's your chance to
        find out! Hack Club Mipmap is a program for teenagers (13–18 inclusive)
        by the nonprofit <a href="https://hackclub.com">Hack Club</a> where you can
        ship a game made without an engine to progress your tech tree and unlock free
        prizes!
    </p>
    <p>&ThickSpace;</p>
    <p>
        Got no idea how to even get started? That's totally fine! Mipmap's got a
        series of videos designed to get you up to speed with the basics in one
        bingeable session. After that, you know exactly what to look for when you're
        going through guides for the specific graphics library you're using.
        Just click the "Learn here!" button above!
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

<footer class="site-footer" bind:clientWidth={footer_width} bind:clientHeight={footer_height}>
    <div class="footer-links">
        <a href="https://hackclub.com/">Hack Club</a>
        <a href="https://hackclub.com/privacy-and-terms">Terms &amp; Privacy Policy</a>
        <a href="https://forms.hackclub.com/bounty">Fulfillment Bounty</a>
        <a href="https://security.hackclub.com/">Security</a>
    </div>
    <p class="made-with">
        Made with <span class="heart" aria-label="love">&hearts;</span> by
        <a href="https://hackclub.enterprise.slack.com/team/U08LEHUPSCR">Brendan</a>
        @ <a href="https://hackclub.com/">Hack Club</a>
    </p>
</footer>

{#if countdown_active}
    <div class="countdown-hanger" role="timer">
        <span class="countdown-cord left" aria-hidden="true"></span>
        <span class="countdown-cord right" aria-hidden="true"></span>
        <span class="countdown-pin left" aria-hidden="true"></span>
        <span class="countdown-pin right" aria-hidden="true"></span>
        <div class="countdown">Starting in {countdown}</div>
        <span class="countdown-pin join-left" aria-hidden="true"></span>
        <span class="countdown-pin join-right" aria-hidden="true"></span>
    </div>
{/if}

<div
    class="logo"
    style:top="{logo_top}px"
    style:left="{logo.left}px"
    style:width="{logo.width}px"
>
    <MipmapLogo />
</div>

<style>
    .site-footer {
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
        filter: drop-shadow(4px 6px 6px rgba(0, 0, 0, 0.45));
    }

    .logo {
        position: absolute;
        filter: drop-shadow(4px 6px 6px rgba(0, 0, 0, 0.45));
    }

    .countdown-hanger {
        position: absolute;
        top: 64px;
        left: 24px;
        width: 208px;
        z-index: 100;
    }

    .countdown-cord {
        position: absolute;
        top: -52px;
        width: 3px;
        height: 57.27px;
        background: #555555;
        transform-origin: 50% 0;
    }

    .countdown-cord.left {
        left: 24px;
        transform: rotate(25deg);
    }

    .countdown-cord.right {
        right: 24px;
        transform: rotate(-25deg);
    }

    .countdown-pin {
        position: absolute;
        top: -59px;
        width: 14px;
        height: 14px;
        border-radius: 50%;
        background: #4f2f1f;
    }

    .countdown-pin.left {
        left: 18px;
    }

    .countdown-pin.right {
        right: 18px;
    }

    .countdown-pin.join-left,
    .countdown-pin.join-right {
        top: -7px;
    }

    .countdown-pin.join-left {
        left: -7px;
    }

    .countdown-pin.join-right {
        right: -7px;
    }

    .countdown {
        box-sizing: border-box;
        width: 208px;
        padding: 0.65rem 0.75rem;
        background: #74472a;
        border: 2px solid #4f2f1f;
        border-radius: 7px;
        color: #ead7c2;
        font-size: 0.875rem;
        font-weight: 700;
        font-variant-numeric: tabular-nums;
        white-space: nowrap;
    }

    .actions {
        margin-top: 1.25rem;
    }
</style>
