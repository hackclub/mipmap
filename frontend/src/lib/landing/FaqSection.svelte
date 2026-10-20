<script lang="ts">
    import type { Snippet } from "svelte";
    import TextBlock from "$lib/landing/TextBlock.svelte";
    import type { Blocked, FieldCog } from "$lib/landing/cog_field";

    let {
        top,
        viewport_width,
        pair_width,
        cogs,
        obstacles,
        height = $bindable(632),
    }: {
        top: number;
        viewport_width: number;
        pair_width: number;
        cogs: FieldCog[];
        obstacles: Blocked[];
        height?: number;
    } = $props();

    const faq_count = 8;
    let box_heights = $state<number[]>(Array(faq_count).fill(0));
    const columns = $derived(viewport_width < 760 ? 1 : 2);
    const box_width = $derived(
        columns === 1
            ? 0.9 * viewport_width
            : Math.max(0.35 * viewport_width, pair_width),
    );
    const gap = $derived(columns === 1 ? 96 : 0.04 * viewport_width);
    const left_x = $derived(
        (viewport_width - columns * box_width - (columns - 1) * gap) / 2,
    );
    const first_row_y = $derived(top + 200);
    const layout = $derived.by(() => {
        const bottoms = [first_row_y, first_row_y];
        const placed = box_heights.map((box_height) => {
            const column = columns === 2 && bottoms[1] < bottoms[0] ? 1 : 0;
            const box_top = bottoms[column];
            bottoms[column] = box_top + box_height + gap;
            return { column, box_top, box_height };
        });
        return { placed, bottom: Math.max(...bottoms) - gap };
    });
    const boxes = $derived(
        layout.placed.map(({ column, box_top, box_height }) => {
            const left = left_x + column * (box_width + gap);
            return {
                left,
                right: left + box_width,
                top: box_top,
                bottom: box_top + box_height,
            };
        }),
    );

    $effect(() => {
        height = layout.bottom - top;
    });
</script>

{#snippet faq_card(index: number, content: Snippet)}
    <TextBlock
        x={boxes[index].left}
        y={boxes[index].top}
        width={box_width}
        snap_width={false}
        {cogs}
        variant="faq"
        min_height={120}
        obstacles={[
            ...obstacles,
            ...boxes.filter((_, other) => other !== index),
        ]}
        bind:box_height={box_heights[index]}
    >
        {@render content()}
    </TextBlock>
{/snippet}

{#snippet faq_1()}
    <strong>Wait, I get prizes for building projects?</strong>
    <div>
        Yep! Hack Club Mipmap is a free online
        program where you track time towards engine-less games to accumulate
        Mips (one of Hack Club Mipmap's currencies - you'll learn about this in
        the onboarding!), which you can then put towards unlocking
        prizes in order along your Tech Tree. You can get
        things like video games, books, monitors, tools, and a bunch of other
        goodies thrown in there. You can also boost how many
        Mips you get through a bunch of means (Snapshots, referrals, limited time
        challenges, feedback opportunities, and more).
    </div>
{/snippet}
{@render faq_card(0, faq_1)}

{#snippet faq_2()}
    <strong>What sorts of projects can I build?</strong>
    <div>
        <p>
            Your challenge is to make a video game <i>without</i> making use of
            a game engine. There are three tiers of projects, sorted by
            complexity. Higher tiers accumulate
            Mips at a higher rate. Here they are:
        </p>
        <ul>
            <li>
                <h3>Tier Quebec</h3>
                <p>
                    This tier is for custom 2D games without a real game
                    engine. Maybe you're doing it in something like the
                    JavaScript canvas; something that lets you draw a few basic
                    shapes but beyond that, you're on your own. This tier is
                    really open in what it accepts.
                </p>
            </li>
            <li>
                <h3>Tier Romeo</h3>
                <p>
                    This tier covers 2D or 3D games made with a low-level graphics
                    library. This includes OpenGL, DirectX, Metal, BGFX, and
                    more. Libraries in this tier must be only for talking to the
                    GPU, they mustn't provide things like lighting. Ask in the <a
                        href="https://hackclub.enterprise.slack.com/archives/C0C2UMX4S9E"
                        >#mipmap</a
                    > Slack channel if you're curious!
                </p>
            </li>
            <li>
                <h3>Tier Sierra</h3>
                <p>
                    Projects in this tier use trickier graphics technologies and
                    get a bit of a Mips boost compared
                    to projects in
                    Tier Romeo. The only projects that
                    qualify are those that meet one of the following criteria:
                </p>
                <ul>
                    <li>Written using Vulkan</li>
                    <li>Written on bare metal (no operating system!)</li>
                    <li>
                        3D game written in a software renderer, that is using no
                        GPU library at all
                    </li>
                </ul>
            </li>
        </ul>
    </div>
{/snippet}
{@render faq_card(1, faq_2)}

{#snippet faq_3()}
    <strong>Who can get this stuff?</strong>
    <div>Any teenager! More specifically, anyone 13 through 18.</div>
{/snippet}
{@render faq_card(2, faq_3)}

{#snippet faq_4()}
    <strong>But I don't know how to do any of this!</strong>
    <div>
        That's totally alright. It's exactly why Hack Club Mipmap has the Mipmap Docs, with a video to teach the basics of graphics programming, along with
        a few other notes. Scroll up a bit and click the big "Learn here!" button
        to find them.
    </div>
{/snippet}
{@render faq_card(3, faq_4)}

{#snippet faq_5()}
    <strong>How do I track my time?</strong>
    <div>
        After you sign in and work through the onboarding, you'll link your <a
            href="https://hackatime.hackclub.com/">Hackatime</a
        >
        account. This is how your time is tracked so we know how many
        Mips
        to give you!
    </div>
{/snippet}
{@render faq_card(4, faq_5)}

{#snippet faq_6()}
    <strong>Can I use AI?</strong>
    <div>
        <p>
            Graphics programming is a tough concept, so you may wish to get some
            help from AI in order to better understand hard concepts. However,
            this trickiness means that it's a phenomenal learning opportunity,
            and vibecoding a project would rob you of this.
        </p>
        <p>
            So here's the rule. You may use as much or as little AI as you feel
            is necessary, but time spent AI coding will count for about a third less than
            time spent coding by hand. Note that the hour tracking in Hackatime
            does not take into account this deflation.
        </p>
        <p>
            You are welcome and encouraged to use these powerful new tools in
            order to help you understand tricky concepts. Graphics programming
            is indeed hard, and AI can be great for explaining concepts in the
            way that clicks best with your brain. However, simply retyping the
            code of an entire project that an LLM put out isn't allowed.
        </p>
    </div>
{/snippet}
{@render faq_card(5, faq_6)}

{#snippet faq_7()}
    <strong>And this is all free?</strong>
    <div>
        Yep! This is a program by <a href="https://hackclub.com">Hack Club</a>,
        a nonprofit organization whose mission is getting teenagers building
        awesome projects. All prizes are funded by generous donations.
    </div>
{/snippet}
{@render faq_card(6, faq_7)}

{#snippet faq_8()}
    <strong>I have more questions!</strong>
    <div>
        Ask in the <a
            href="https://hackclub.enterprise.slack.com/archives/C0C2UMX4S9E"
            >#mipmap</a
        >
        Slack channel or shoot me an <a href="mailto:brendan@hackclub.com">email</a>.
    </div>
{/snippet}
{@render faq_card(7, faq_8)}

<style>
    div {
        font-size: 0.85em;
    }

    h3 {
        margin-block: 1.67em;
        font-size: 0.83em;
    }

    div p:not(:last-child) {
        margin-bottom: 1em;
    }
</style>
