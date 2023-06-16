<script lang="ts">
    import { scale } from "svelte/transition"

    export let fullWidth = false
</script>

<span
    class=" text-success-content bg-success p-4 rounded-lg border-2 border-success-outline drop-shadow-card"
    class:w-full={fullWidth}
    transition:scale={{ duration: 70, opacity: 1 }}
>
    <slot />
</span>
