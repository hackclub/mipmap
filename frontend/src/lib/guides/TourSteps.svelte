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

    export interface TourNote {
        anchor: string;
        side: "left" | "right";
        title?: Snippet;
        body: Snippet;
    }

    export interface TourStep {
        title?: Snippet;
        body?: Snippet;
        focus?: Part[];
        frame?: (Part | "token")[];
        center?: boolean;
        spot?: "new" | "edit";
        glow?: boolean;
        dialog?: "new" | "edit";
        notes?: TourNote[];
    }

    export const interactive = (step: TourStep) => !!(step.spot || step.dialog);

    export const steps: TourStep[] = [
        { title: welcome_title, body: welcome, center: true },
        { title: tree_title, body: tree, focus: ["tree"] },
        {
            title: token_title,
            body: token,
            focus: ["tree"],
            frame: ["token"],
            glow: true,
        },
        {
            title: project_title,
            body: project,
            focus: ["projects"],
            spot: "new",
        },
        {
            focus: ["projects"],
            dialog: "new",
            notes: [
                { anchor: "name", side: "left", title: name_title, body: name },
            ],
        },
        { title: edit_title, focus: ["projects"], spot: "edit" },
        {
            focus: ["projects"],
            dialog: "edit",
            notes: [
                { anchor: "links", side: "left", body: links },
                { anchor: "tier", side: "right", body: tiers },
                { anchor: "hackatime", side: "left", body: hackatime },
                { anchor: "actions", side: "left", body: save },
            ],
        },
        { title: snapshots_title, body: snapshots, focus: ["snapshots"] },
        {
            title: admin_title,
            body: admin,
            focus: ["fulfillment", "orders", "referral", "profile", "feedback"],
        },
    ];
</script>

{#snippet welcome_title()}
    Welcome to <span class="mipmap">Mipmap</span>!
{/snippet}

{#snippet welcome()}
    <p>Looking forward to seeing what you create!</p>
{/snippet}

{#snippet tree_title()}
    Here's the <span class="tech-tree">tech tree</span> with all your prizes!
{/snippet}

{#snippet tree()}
    <p>
        Ship projects to earn <span class="mip">Mips</span> to unlock nodes down
        the tree
    </p>
{/snippet}

{#snippet token_title()}
    This is a token
{/snippet}

{#snippet token()}
    <p>
        After unlocking a node with <span class="mip">Mips</span>, you can
        redeem that prize with one of these tokens also found in the tree. One
        token can unlock any one prize
    </p>
{/snippet}

{#snippet project_title()}
    Let's make a project to get <span class="mip">Mips</span>!
{/snippet}

{#snippet project()}
    <p>Click on the '+' button to make a new project</p>
{/snippet}

{#snippet name_title()}
    Give your project a name
{/snippet}

{#snippet name()}
    <p><b>Everything can be changed later</b></p>
{/snippet}

{#snippet edit_title()}
    Let's take a further look into this project
{/snippet}

{#snippet links()}
    <p>You don't have to fill these now, you can do it later</p>
{/snippet}

{#snippet tiers()}
    <p>
        <span class="tier">Tier Quebec</span>: 2D games without an engine, maybe
        in a JS canvas?
    </p>
    <p>
        <span class="tier">Tier Romeo</span>: 3D games in most low level
        graphics libraries
    </p>
    <p>
        <span class="tier">Tier Sierra</span>: 3D game in either Vulkan, a
        software renderer (no graphics libraries), or on bare metal
    </p>
{/snippet}

{#snippet hackatime()}
    <p>Once you start tracking your project with Hackatime, it'll show here!</p>
{/snippet}

{#snippet save()}
    <p>Save your changes to continue</p>
{/snippet}

{#snippet snapshots_title()}
    Share what you're up to!
{/snippet}

{#snippet snapshots()}
    <p>Though you also get extra <span class="mip">Mips</span> for doing so...</p>
{/snippet}

{#snippet admin_title()}
    And here's some administrative stuff
{/snippet}

{#snippet admin()}
    <p>
        That's it for now, excited to see what you create for
        <span class="mipmap">Mipmap</span>!
    </p>
{/snippet}

