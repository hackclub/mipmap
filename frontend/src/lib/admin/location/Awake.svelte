<script lang="ts">
    import { zone_hour } from "./geo";

    let {
        groups,
    }: { groups: { zone?: string; offset?: number; users: number }[] } = $props();

    let now = $state(new Date());
    let active = $state<number | null>(null);

    $effect(() => {
        const timer = setInterval(() => {
            const next = new Date();
            if (next.getHours() !== now.getHours()) now = next;
        }, 60_000);
        return () => clearInterval(timer);
    });

    const slots = $derived(
        Array.from({ length: 24 }, (_, hour) => {
            const at = new Date(now.getFullYear(), now.getMonth(), now.getDate(), hour);
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
    const label = (hour: number) =>
        new Date(2000, 0, 1, hour).toLocaleTimeString(undefined, { hour: "numeric" });
    const shown = $derived(slots[active ?? now.getHours()]);
    const describe = (slot: { hour: number; awake: number }) =>
        `${label(slot.hour)}: ${slot.awake.toLocaleString()} of ${total.toLocaleString()} awake`;

    function keydown(e: KeyboardEvent, hour: number) {
        const target =
            e.key === "ArrowRight"
                ? Math.min(23, hour + 1)
                : e.key === "ArrowLeft"
                  ? Math.max(0, hour - 1)
                  : e.key === "Home"
                    ? 0
                    : e.key === "End"
                      ? 23
                      : -1;
        if (target < 0) return;
        e.preventDefault();
        active = target;
        (e.currentTarget as HTMLElement).parentElement
            ?.querySelectorAll<HTMLElement>(".slot")
            [target]?.focus();
    }
</script>

<div class="awake">
    <div class="strip" role="group" aria-label="People awake at each hour">
        {#each slots as slot (slot.hour)}
            <button
                type="button"
                class="slot"
                class:now={slot.hour === now.getHours()}
                class:active={slot.hour === shown.hour}
                tabindex={slot.hour === shown.hour ? 0 : -1}
                aria-label={describe(slot)}
                onclick={() => (active = slot.hour)}
                onfocus={() => (active = slot.hour)}
                onpointerenter={() => (active = slot.hour)}
                onkeydown={(e) => keydown(e, slot.hour)}
            >
                <span class="bar" style:height="{(slot.awake / best) * 100}%"></span>
            </button>
        {/each}
    </div>
    <div class="hours" aria-hidden="true">
        {#each slots.filter((slot) => slot.hour % 3 === 0) as slot (slot.hour)}
            <span>{label(slot.hour)}</span>
        {/each}
    </div>
    <p class="readout" aria-hidden="true">{describe(shown)}</p>
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
        padding: 0;
        background: #4f2f1f;
        border: none;
        border-radius: 2px;
        cursor: pointer;
    }

    .slot.now {
        outline: 2px solid #ead7c2;
        outline-offset: 1px;
    }

    .slot.active .bar {
        background: #ead7c2;
    }

    .slot:focus-visible {
        outline: 2px dashed #ead7c2;
        outline-offset: 2px;
    }

    .bar {
        display: block;
        width: 100%;
        background: #d6ab70;
        border-radius: 2px;
    }

    .hours {
        display: grid;
        grid-template-columns: repeat(8, minmax(0, 1fr));
        color: #ead7c2;
        font-size: 0.7rem;
    }

    .readout {
        margin: 0;
        font-size: 0.85rem;
    }
</style>
