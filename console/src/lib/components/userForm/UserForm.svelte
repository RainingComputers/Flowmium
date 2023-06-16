<script lang="ts">
	import Button from '$lib/components/Button.svelte'
	import Card from '$lib/components/Card.svelte'
	import Entry from '$lib/components/Entry.svelte'
	import Error from '$lib/components/toast/Error.svelte'
	import LogoAndName from '$lib/components/app/LogoAndName.svelte'

	export let error: string
	export let buttonLabel: string
	export let onClick: () => void
	export let confirmPassword: boolean = false

	let usernameEntry: Entry
	let passwordEntry: Entry
	let confirmPasswordEntry: Entry

	export function getUsername() {
		return usernameEntry.getValue()
	}

	export function getPassword() {
		return passwordEntry.getValue()
	}

	export function getConfirmPassword() {
		return confirmPasswordEntry.getValue()
	}
</script>

<Card>
	<div class="flex flex-col gap-10 items-center p-5">
		<LogoAndName />
		<div class="flex flex-col w-full gap-4">
			{#if error}
				<Error fullWidth={true}>
					{error}
				</Error>
			{/if}
			<Entry label="Username" bind:this={usernameEntry} onEnter={onClick} />
			<Entry label="Password" bind:this={passwordEntry} password={true} onEnter={onClick} />
			{#if confirmPassword}
				<Entry
					label="Confirm password"
					bind:this={confirmPasswordEntry}
					password={true}
					onEnter={onClick}
				/>
			{/if}
		</div>

		<Button {onClick} fullWidth={true} label={buttonLabel} />
	</div>
</Card>
