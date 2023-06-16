<script context="module" lang="ts">
	export enum ButtonType {
		HERO,
		PRIMARY,
		SECONDARY
	}

	function getButtonClass(type: ButtonType) {
		switch (type) {
			case ButtonType.HERO:
				return 'bg-primary hover:bg-primary-focus text-primary-content text-2xl py-4 px-10'
			case ButtonType.PRIMARY:
				return 'bg-primary hover:bg-primary-focus text-primary-content py-2 px-5'
			case ButtonType.SECONDARY:
				return 'bg-base-3 hover:bg-base-2 text-content py-2 px-5'
		}
	}
</script>

<script lang="ts">
	export let label: string
	export let onClick: () => void | undefined = undefined
	export let fullWidth: boolean = false
	export let expand: boolean = false
	export let type: ButtonType = ButtonType.PRIMARY

	const buttonClass = getButtonClass(type)
</script>

<button
	on:click={onClick}
	class="drop-shadow-card font-bold rounded-xl {buttonClass}"
	class:w-full={fullWidth}
	class:flex-grow={expand}
>
	<span>{label}</span>
</button>
