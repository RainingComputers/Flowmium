<script lang="ts">
    import { scale } from "svelte/transition"

    export let width: number = 400
</script>

<span
    class=" bg-base-1 border-base-2 border drop-shadow-card "
    style="width: {width}px;"
    transition:scale={{ duration: 70, opacity: 1 }}
>
    <slot />
</span>
