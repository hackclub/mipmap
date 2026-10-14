<script lang="ts">
    let {
        size,
        state = "in",
        tilt = 0,
    }: { size: number; state?: "in" | "idle" | "out"; tilt?: number } =
        $props();

    const cog =
        "m 163.75,13.49 a 190,190 0 0 1 72.5,0 l 5.1,42.32 a 150,150 0 0 1 45.6,21.97 l 36.26,-22.41 a 190,190 0 0 1 45.21,56.69 l -29.91,30.37 a 150,150 0 0 1 11.26,49.35 l 40.13,14.38 a 190,190 0 0 1 -16.14,70.69 l -42.39,-4.46 a 150,150 0 0 1 -31.56,39.58 l 13.77,40.33 a 190,190 0 0 1 -65.32,31.46 l -22.95,-35.91 a 150,150 0 0 1 -50.62,0 l -22.95,35.91 A 190,190 0 0 1 86.42,352.3 l 13.77,-40.33 A 150,150 0 0 1 68.63,272.39 l -42.39,4.46 A 190,190 0 0 1 10.1,206.16 L 50.23,191.78 A 150,150 0 0 1 61.5,142.43 L 31.58,112.06 A 190,190 0 0 1 76.79,55.37 l 36.26,22.41 a 150,150 0 0 1 45.6,-21.97 z M 95,200 a 105,105 0 1 0 210,0 105,105 0 1 0 -210,0 z";
    const mouth =
        "M 7.3558598,49.459401 C 20.412913,61.001532 38.616474,62.681662 55.041749,65.627138 61.874063,66.804321 70.16519,61.048107 75.40725,62.435889 77.016854,69.3093 72.704893,69.202241 61.974827,75.624609 59.785277,78.074693 46.155626,80.982947 37.402663,77.510348 29.525866,74.205606 21.411526,71.017334 15.554539,64.513638 11.890293,61.015709 3.3759313,54.908547 7.3558598,49.459401 Z";
    const eyes = [
        { x: -1.2027391, y: -1.1908172, spin: "left" },
        { x: 40.259095, y: 8.4190743, spin: "right" },
    ];
    const scale = 0.11908308;
</script>

<div
    class="avatar {state}"
    style:width="{size}px"
    style:height="{size}px"
    style:--tilt="{tilt}deg"
    aria-hidden="true"
>
    <div class="part eyes">
        <div class="drift">
            {#each eyes as eye (eye.x)}
                <div
                    class="eye spin {eye.spin}"
                    style:left="{(eye.x / 87) * 100}%"
                    style:top="{(eye.y / 87) * 100}%"
                    style:width="{((400 * scale) / 87) * 100}%"
                >
                    <svg viewBox="0 0 400 400">
                        <path fill-rule="evenodd" d={cog} />
                        <circle cx="200" cy="200" r="28" />
                    </svg>
                </div>
            {/each}
        </div>
    </div>
    <div class="part mouth">
        <div class="drift">
            <svg viewBox="0 0 87 87"><path d={mouth} /></svg>
        </div>
    </div>
</div>

<style>
    .avatar {
        position: relative;
        transform: rotate(var(--tilt));
        transition: transform 0.4s ease-out;
    }

    .part,
    .drift,
    svg {
        position: absolute;
        inset: 0;
    }

    svg {
        width: 100%;
        height: 100%;
        overflow: visible;
        fill: #7b4b2a;
    }

    .eye {
        position: absolute;
        aspect-ratio: 1;
    }

    .spin,
    .drift {
        will-change: transform;
    }

    .spin {
        animation: spin 7s linear infinite;
    }

    .spin.left {
        animation-direction: reverse;
    }

    .eyes .drift {
        animation: drift-eyes 6.2s ease-in-out infinite alternate;
    }

    .mouth .drift {
        animation: drift-mouth 4.7s ease-in-out infinite alternate;
    }

    .in .eyes {
        animation: in-eyes 1.2s cubic-bezier(0.2, 0.75, 0.3, 1.08) both;
    }

    .in .mouth {
        animation: in-mouth 1.2s cubic-bezier(0.2, 0.75, 0.3, 1.08) 0.12s both;
    }

    .out .eyes {
        animation: out-eyes 0.8s cubic-bezier(0.55, 0, 0.8, 0.4) both;
    }

    .out .mouth {
        animation: out-mouth 0.8s cubic-bezier(0.55, 0, 0.8, 0.4) both;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    @keyframes drift-eyes {
        0% {
            transform: translate(0, 0);
        }
        35% {
            transform: translate(3%, -2.5%);
        }
        70% {
            transform: translate(-2%, -4%);
        }
        100% {
            transform: translate(-3%, 1.5%);
        }
    }

    @keyframes drift-mouth {
        0% {
            transform: translate(0, 0) rotate(0deg);
        }
        40% {
            transform: translate(-3%, 2.5%) rotate(-3deg);
        }
        100% {
            transform: translate(2.5%, 4%) rotate(2deg);
        }
    }

    @keyframes in-eyes {
        from {
            transform: translate(-60vw, -70vh) rotate(-140deg) scale(0.6);
        }
    }

    @keyframes in-mouth {
        from {
            transform: translate(55vw, 75vh) rotate(160deg) scale(0.6);
        }
    }

    @keyframes out-eyes {
        to {
            transform: translate(70vw, -80vh) rotate(160deg) scale(0.6);
        }
    }

    @keyframes out-mouth {
        to {
            transform: translate(-70vw, 80vh) rotate(-140deg) scale(0.6);
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .avatar,
        .spin,
        .drift,
        .in .eyes,
        .in .mouth,
        .out .eyes,
        .out .mouth {
            animation: none;
            transition: none;
        }
    }
</style>
