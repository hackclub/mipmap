<script lang="ts" module>
    import type { Snippet } from "svelte";

    export type Part =
        | "projects"
        | "tree"
        | "fulfillment"
        | "orders"
        | "referral"
        | "profile"
        | "snapshots"
        | "feedback";

    export interface TourStep {
        title: string;
        body: Snippet;
        focus?: Part[];
    }

    export const steps: TourStep[] = [
        { title: "Your dashboard", body: dashboard },
        { title: "Projects", body: projects, focus: ["projects"] },
        { title: "Tech tree", body: tree, focus: ["tree"] },
        {
            title: "Orders and fulfillment rounds",
            body: orders,
            focus: ["orders", "fulfillment"],
        },
        {
            title: "Referrals and profile",
            body: account,
            focus: ["referral", "profile"],
        },
        { title: "Snapshots", body: snapshots, focus: ["snapshots"] },
        { title: "Feedback", body: feedback, focus: ["feedback"] },
        { title: "That's everything!", body: done },
    ];
</script>

{#snippet dashboard()}
    <p>
        Welcome to <span class="mipmap">Hack Club Mipmap</span>! This is the
        dashboard that you'll handle everything like shipping projects, claiming
        prizes, and a whole lot more.
    </p>
{/snippet}

{#snippet projects()}
    <p>
        This is the Projects panel, where you will manage creating new projects,
        editing them to make sure they have everything they need, and finally
        shipping them for review to get <span class="mip">Mips</span>!
    </p>
{/snippet}

{#snippet tree()}
    <p>
        Once you get your <span class="mip">Mips</span>, you can redeem them
        here at the <span class="tech-tree">Tech Tree</span>. You only need one
        prerequisite of an item in order to be able to unlock that item. For
        actually claiming items, you'll notice Tokens spread throughout the
        tree. After unlocked, they can be used to then claim <b>any one item</b> in
        the tree, regardless of where it's situated.
    </p>
{/snippet}

{#snippet orders()}
    <p>
        After you claim an item, it'll show up here. Notice the fulfillment
        rounds. These are when projects will be reviewed and prizes will be
        fulfilled. No guarantees on review timelines exist outside these
        fulfillment rounds, so you might want to make sure you get your project
        in by each one.
    </p>
{/snippet}

{#snippet account()}
    <p>
        Here's where you can see you account information and your referral link.
        Having a friend ship a project after having signed up with your link
        gets you 100 <span class="mip">Mips</span> and your friend 20
        <span class="mip">Mips</span> to start out with!
    </p>
{/snippet}

{#snippet feedback()}
    <p>
        Share your voice on how <span class="mipmap">Mipmap</span> is going! It would
        be awesome if you could do this somewhat frequently, since this is your chance
        to directly shape the future direction of the program to make it even more
        awesome.
    </p>
{/snippet}

{#snippet snapshots()}
    <p>
        Here's the snapshots view! A snapshot is a screenshot plus a caption of
        what you're working on (both successes and fails are welcome). The top
        left is where you submit a snapshot, under that you can see you past
        snapshots, and to the right you can see what everyone else in the
        <span class="mipmap">Mipmap</span> community is up to!
    </p>
    <p>
        Also, posting snapshots gets you more <span class="mip">Mips</span>!
        Each one you share gives you an additional 1% towards all
        <span class="mip">Mips</span> payouts, capping at 32%. You don't lose these
        percentage points, but you can only post once per day so make sure you've
        got your bonus up by the time you ship!
    </p>
{/snippet}

{#snippet done()}
    <p>
        And that's about it! Go make an awesome engineless video game. Looking
        forward to see what you're capable of!
    </p>
    <p>
        If anything pops up, stick it into the <a
            href="https://hackclub.enterprise.slack.com/archives/C0C2UMX4S9E"
            >#mipmap</a
        >
        or shoot <a href="mailto:brendan@hackclub.com">me</a> an email.
    </p>
{/snippet}

<style>
    .mip {
        color: #ff9a3d;
    }

    .mipmap {
        color: #8fcf6b;
    }

    .tech-tree {
        color: #7cc4f0;
    }
</style>
