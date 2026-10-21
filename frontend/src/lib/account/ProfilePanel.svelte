<script lang="ts">
    import Section from "$lib/dashboard/Section.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import { Task } from "$lib/load.svelte";
    import { log_out } from "$lib/account/logout";
    import type { User } from "$lib/dashboard/state.svelte";
    import type { Bounds } from "$lib/dashboard/PanZoom.svelte";

    let {
        bounds,
        currency,
        user,
        tokens,
    }: { bounds: Bounds; currency: string; user: User; tokens: number } =
        $props();

    const logout = new Task("Couldn't log out");
</script>

<Section title="Profile" {bounds} zoom={3} padded>
    <div class="profile">
        <dl>
            <dt>First name</dt>
            <dd>{user.first_name ?? ""}</dd>
            <dt>{currency}</dt>
            <dd>{user.mips} {currency}</dd>
            <dt>Tokens</dt>
            <dd>{tokens} left to spend</dd>
        </dl>
        <div class="logout">
            <a class="button" href="/notifications">
                Manage your Slack DMs from Mipmap
            </a>
            <MipmapButton
                type="button"
                disabled={logout.pending}
                onclick={() => logout.run(() => log_out())}
            >
                Log out
            </MipmapButton>
            <MipmapButton
                type="button"
                disabled={logout.pending}
                onclick={() => logout.run(() => log_out(true))}
            >
                Log out everywhere
            </MipmapButton>
            {#if logout.error}
                <p role="alert">{logout.error}</p>
            {/if}
        </div>
    </div>
</Section>

<style>
    .profile {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        gap: 0.75rem 1.5rem;
    }

    dl {
        display: grid;
        grid-column: 1 / -1;
        grid-template-columns: subgrid;
        row-gap: 0.75rem;
        margin: 0;
    }

    .logout {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
        grid-column: 2;
        margin-top: 0.75rem;
    }

    .button,
    .button:visited {
        background-color: #4f2f1f;
        color: #ead7c2;
        padding: 0.75rem 1.5rem;
        font-size: clamp(1.25rem, 2vw, 1.75rem);
        font-weight: 700;
        text-decoration: none;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    .button:focus-visible {
        background-color: #ead7c2;
        color: #4f2f1f;
    }

    @media (hover: hover) {
        .button:hover {
            background-color: #ead7c2;
            color: #4f2f1f;
        }
    }

    .logout p {
        flex-basis: 100%;
        margin: 0;
        font-size: 0.85em;
    }

    dt {
        font-size: 0.85em;
        font-weight: 700;
    }

    dd {
        margin: 0;
        font-size: 0.85em;
    }
</style>
