<script lang="ts">
    import Section from "$lib/dashboard/Section.svelte";
    import MipmapButton from "$lib/ui/MipmapButton.svelte";
    import SlackDialog from "./SlackDialog.svelte";
    import { json, request } from "$lib/api";
    import { Task } from "$lib/load.svelte";
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
    let slack = $state(false);

    const log_out = () =>
        logout.run(async () => {
            const response = await request("/api/logout", { method: "POST" });
            if (!response.ok) await json(response);
            location.href = "/";
        });
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
            <MipmapButton type="button" onclick={() => (slack = true)}>
                Manage your Slack DMs from Mipmap
            </MipmapButton>
            <MipmapButton
                type="button"
                disabled={logout.pending}
                onclick={log_out}
            >
                Log out
            </MipmapButton>
            {#if logout.error}
                <p role="alert">{logout.error}</p>
            {/if}
        </div>
    </div>
</Section>

{#if slack}
    <SlackDialog onclose={() => (slack = false)} />
{/if}

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
