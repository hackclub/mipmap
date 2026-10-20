<script lang="ts">
    import { zone_hour } from "./geo";

    let {
        groups,
    }: { groups: { zone?: string; offset?: number; users: number }[] } = $props();

    const now = new Date();
    const start = new Date(now);
    start.setHours(0, 0, 0, 0);

    const slots = $derived(
        Array.from({ length: 24 }, (_, hour) => {
            const at = new Date(start.getTime() + hour * 3_600_000);
            const awake = groups.reduce((sum, group) => {
                const local = group.zone
                    ? zone_hour(group.zone, at)
                    : (at.getUTCHours() + (group.offset ?? 0) + 24) % 24;
                return local >= 9 && local < 21 ? sum + group.users : sum;
            }, 0);
            return { hour, at, awake };
        }),
    );
    const total = $derived(groups.reduce((sum, group) => sum + group.users, 0));
    const best = $derived(Math.max(1, ...slots.map((slot) => slot.awake)));
    const label = (at: Date) => at.toLocaleTimeString(undefined, { hour: "numeric" });
</script>

<div class="awake">
    <div class="strip">
        {#each slots as slot (slot.hour)}
            <div
                class="slot"
                class:now={slot.hour === now.getHours()}
                title="{label(slot.at)}: {slot.awake.toLocaleString()} of {total.toLocaleString()} awake"
            >
                <div class="bar" style:height="{(slot.awake / best) * 100}%"></div>
            </div>
        {/each}
    </div>
    <div class="hours" aria-hidden="true">
        {#each slots.filter((slot) => slot.hour % 3 === 0) as slot (slot.hour)}
            <span>{label(slot.at)}</span>
        {/each}
    </div>
    <p class="sr">
        {#each slots as slot (slot.hour)}
            {label(slot.at)}: {slot.awake}.
        {/each}
    </p>
</div>

<style>
    .awake {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    .strip {
        display: grid;
        grid-template-columns: repeat(24, minmax(0, 1fr));
        gap: 2px;
        height: 5rem;
    }

    .slot {
        display: flex;
        align-items: flex-end;
        background: #4f2f1f;
        border-radius: 2px;
    }

    .slot.now {
        outline: 2px solid #ead7c2;
        outline-offset: 1px;
    }

    .bar {
        width: 100%;
        background: #d6ab70;
        border-radius: 2px;
    }

    .hours {
        display: grid;
        grid-template-columns: repeat(8, minmax(0, 1fr));
        color: #d6ab70;
        font-size: 0.7rem;
    }

    .sr {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }
</style>
