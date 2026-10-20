<script lang="ts" module>
    import type { Snippet } from "svelte";

    export type Frame =
        "projects" | "tile" | "tree" | "start" | "snapshots" | "side" | "home";

    export type Hole =
        | "projects"
        | "new"
        | "name"
        | "create"
        | "edit"
        | "tier"
        | "hackatime"
        | "save"
        | "dialog"
        | "ship"
        | "tree"
        | "token"
        | "sticker"
        | "snapshots"
        | "side";

    export type Wait =
        | "new-open"
        | "created"
        | "edit-open"
        | "saved"
        | "sticker-unlocked"
        | "token-unlocked"
        | "sticker-redeemed"
        | "slack-answer";

    export interface Context {
        editable: boolean;
        granted: boolean;
        returning: boolean;
        sticker_skipped: boolean;
        slack_linked: boolean;
        slack_join: "joined" | "failed" | null;
        unlocked: Set<number>;
        purchased: Set<number>;
    }

    export interface Step {
        key: string;
        say: Snippet;
        sub?: Snippet;
        sub_when?: (ctx: Context) => boolean;
        frame?: Frame;
        holes?: Hole[];
        wait?: Wait;
        dialog?: "new" | "edit";
        resume?: string;
        center?: boolean;
        grant?: boolean;
        project?: boolean;
        skip?: (ctx: Context) => boolean;
        replay?: boolean;
        required?: boolean;
        interactive?: boolean;
        demo?: boolean;
    }

    import { STICKER, STICKER_TOKEN } from "$lib/tree/tutorial";

    export const steps: Step[] = [
        {
            key: "Welcome",
            say: welcome,
            sub: improved,
            sub_when: (ctx) => ctx.returning,
            center: true,
            frame: "home",
        },
        {
            key: "Projects",
            say: projects,
            frame: "projects",
            holes: ["projects"],
        },
        {
            key: "New project",
            say: plus,
            frame: "projects",
            holes: ["new"],
            wait: "new-open",
            skip: (ctx) => ctx.editable,
            replay: false,
        },
        {
            key: "Name",
            say: name,
            holes: ["name", "create"],
            wait: "created",
            dialog: "new",
            resume: "New project",
            skip: (ctx) => ctx.editable,
            replay: false,
        },
        {
            key: "Edit",
            say: edit,
            frame: "tile",
            holes: ["edit"],
            wait: "edit-open",
            project: true,
        },
        {
            key: "Tiers",
            say: tiers,
            sub: tiers_note,
            holes: ["tier"],
            dialog: "edit",
            interactive: true,
            resume: "Edit",
            project: true,
        },
        {
            key: "Hackatime",
            say: hackatime,
            holes: ["hackatime"],
            dialog: "edit",
            interactive: true,
            resume: "Edit",
            project: true,
        },
        {
            key: "Save",
            say: save,
            holes: ["save", "dialog"],
            wait: "saved",
            dialog: "edit",
            resume: "Edit",
            project: true,
        },
        {
            key: "Ship",
            say: ship,
            frame: "tile",
            holes: ["ship"],
            project: true,
        },
        { key: "Tech Tree", say: tree, frame: "tree", holes: ["tree"] },
        { key: "Currencies", say: currencies, frame: "tree", holes: ["tree"] },
        { key: "Mips", say: mips, frame: "tree", holes: ["tree"], demo: true },
        { key: "Tokens", say: tokens, frame: "start", holes: ["token"] },
        { key: "Token use", say: token_use, frame: "start", holes: ["token"] },
        {
            key: "Free Mips",
            say: practice,
            frame: "start",
            holes: ["sticker", "token"],
            grant: true,
            skip: (ctx) => ctx.granted,
            replay: false,
            required: true,
        },
        {
            key: "Unlock sticker",
            say: unlock_sticker,
            frame: "start",
            holes: ["sticker"],
            wait: "sticker-unlocked",
            skip: (ctx) => ctx.unlocked.has(STICKER),
            required: true,
        },
        {
            key: "Unlock token",
            say: unlock_token,
            frame: "start",
            holes: ["token"],
            wait: "token-unlocked",
            skip: (ctx) => ctx.unlocked.has(STICKER_TOKEN),
            required: true,
        },
        {
            key: "Redeem sticker",
            say: redeem,
            sub: redeem_note,
            frame: "start",
            holes: ["sticker"],
            wait: "sticker-redeemed",
            skip: (ctx) => ctx.purchased.has(STICKER) || ctx.sticker_skipped,
            required: true,
        },
        {
            key: "Sticker sent",
            say: sent,
            frame: "start",
            holes: ["sticker"],
            skip: (ctx) => ctx.sticker_skipped,
        },
        {
            key: "Snapshots",
            say: snapshots,
            frame: "snapshots",
            holes: ["snapshots"],
        },
        {
            key: "Snapshot info",
            say: snapshot_info,
            frame: "snapshots",
            holes: ["snapshots"],
        },
        {
            key: "Snapshot rules",
            say: snapshot_rules,
            frame: "snapshots",
            holes: ["snapshots"],
        },
        { key: "Other panels", say: side, frame: "side", holes: ["side"] },
        { key: "Slack", say: slack, center: true, frame: "home" },
        {
            key: "Join Slack",
            say: join_slack,
            center: true,
            frame: "home",
            wait: "slack-answer",
            skip: (ctx) => !ctx.slack_linked,
        },
        {
            key: "Joined Slack",
            say: joined_slack,
            center: true,
            frame: "home",
            skip: (ctx) => ctx.slack_join !== "joined",
        },
        {
            key: "Join failed",
            say: join_failed,
            center: true,
            frame: "home",
            skip: (ctx) => ctx.slack_join !== "failed",
        },
        { key: "Goodbye", say: goodbye, center: true, frame: "home" },
    ];

    export const index = (key: string) =>
        steps.findIndex((step) => step.key === key);
</script>

{#snippet welcome()}
    Hi, I'm Steve the Sprite, and welcome to Mipmap!
    I'm here to show you how everything works internally. This is how you will
    track the time of your game made without an engine and earn prizes!
{/snippet}

{#snippet improved()}
    The onboarding flow is rebuilt to minimize confusion. I've heard there's a
    free sticker at the end...
{/snippet}

{#snippet projects()}
    If you want to claim your time for prizes, you've
    gotta first set up a project. You'll track this time with a tool called
    <a href="https://hackatime.hackclub.com/" target="_blank" rel="noopener"
        >Hackatime</a
    > (which we'll link afterwards) and it'll be submitted for Mips (which
    we'll get to in a moment)
{/snippet}

{#snippet plus()}
    Use this button to make that project
{/snippet}

{#snippet name()}
    Pick a snazzy name for your project. You can always change this later
{/snippet}

{#snippet edit()}
    Let's go add some more details to our project
{/snippet}

{#snippet tiers()}
    Pick the tier that matches how deep your project
    goes. Higher tiers earn more points towards prizes
    <ul>
        <li>
            Tier Quebec: 2D games without an engine,
            maybe in a JS canvas?
        </li>
        <li>
            Tier Romeo: 3D games in most low level
            graphics libraries
        </li>
        <li>
            Tier Sierra: 3D game in either Vulkan, a
            software renderer (no graphics libraries), or on bare metal
        </li>
    </ul>
{/snippet}

{#snippet tiers_note()}
    Don't worry if you don't know what these words mean, there's some helpful
    resources mentioned later in this onboarding
{/snippet}

{#snippet hackatime()}
    Hackatime is Hack Club's tool for tracking coding time. Once you've linked
    it and tracked time towards projects, make sure to come here and tick them
    off! I'll link it at the end of this onboarding then you'll be able to actually see the project list
{/snippet}

{#snippet save()}
    Make sure to save!
{/snippet}

{#snippet ship()}
    Whenever your project's good to go, click "Ship" to send it off for review!
    After we review your project, you get your Mips to use in your
    Tech Tree. Speaking of which...
{/snippet}

{#snippet tree()}
    Here's your Tech Tree of
    prizes! I know it's a bit scary, but don't worry,
    I'll show you through it!
{/snippet}

{#snippet currencies()}
    There are two currencies, Mips and
    Tokens
{/snippet}

{#snippet mips()}
    You get Mips from shipping projects, and these let you
    unlock nodes down the tree
{/snippet}

{#snippet tokens()}
    Now, unlocking a node doesn't immediately send that prize to your door.
    You've got to also unlock a Token, then use that Token on an unlocked node
    to actually get the prize sent to you
{/snippet}

{#snippet token_use()}
    One Token lets you redeem
    <b
        >any single prize from a node you've unlocked
        with Mips</b
    >
{/snippet}

{#snippet practice()}
    I'll give you 120 Mips for the sticker!
{/snippet}

{#snippet unlock_sticker()}
    First, unlock the Mipmap sticker with your
    Mips! (click the node to open, then press and hold
    the 'Unlock' button)
{/snippet}

{#snippet unlock_token()}
    Then, unlock this Token with the rest of your
    Mips!
{/snippet}

{#snippet redeem()}
    Alright, now go spend this Token so we'll send a free
    sticker to your door!
{/snippet}

{#snippet redeem_note()}
    Normally any Token can be used for any
    prize. You will have to tick a customs agreement box, and then the sticker
    will be sent to your door!
{/snippet}

{#snippet sent()}
    Awesome! We'll send you the sticker in the mail (you can track it at
    <a href="https://mail.hackclub.com" target="_blank" rel="noopener"
        >mail.hackclub.com</a
    >)
{/snippet}

{#snippet snapshots()}
    We're almost near the end, I promise! Making games is a super visual
    activity, so you can be sharing regular Snapshots with everyone else
{/snippet}

{#snippet snapshot_info()}
    A Snapshot is a screenshot + a caption. You
    don't have to do this, but it does give you a sweet
    Mips multiplier to your ships!
{/snippet}

{#snippet snapshot_rules()}
    You can only post a Snapshot once per day and after
    having logged 15 minutes of new time on a project
{/snippet}

{#snippet side()}
    And that's about it for now! You can find some details on the right side,
    like limited time challenges and referrals.
{/snippet}

{#snippet slack()}
    If you're wanting to meet a bunch of people also interested in Mipmap,
    we've got a channel in the Hack Club Slack! Join
    <a
        href="https://hackclub.enterprise.slack.com/archives/C0C2UMX4S9E"
        target="_blank"
        rel="noopener">#mipmap</a
    > to:
    <ul>
        <li>Meet some other teens interested in this just as much as you are</li>
        <li>Ask for some help when you're stuck</li>
        <li>Get inspiration and ideas for projects to build</li>
        <li>Much more!</li>
    </ul>
{/snippet}

{#snippet join_slack()}
    Do you want me to automatically join you to this channel?
{/snippet}

{#snippet joined_slack()}
    Done, see you in
    <a
        href="https://hackclub.enterprise.slack.com/archives/C0C2UMX4S9E"
        target="_blank"
        rel="noopener">#mipmap</a
    >!
{/snippet}

{#snippet join_failed()}
    Oh, looks like something went wrong and I wasn't able to add you. You can
    still join on your own!
{/snippet}

{#snippet goodbye()}
    I'm super looking forward to what you create. If you don't know where to
    start, check out the bottom right of this canvas (you might have to scroll
    to find it). Happy Hacking!
{/snippet}
