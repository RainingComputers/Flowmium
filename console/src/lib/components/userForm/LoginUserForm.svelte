<script lang="ts">
	import UserForm from './UserForm.svelte'

	let error: string = ''

	async function onClick() {}
</script>

<UserForm {error} {onClick} buttonLabel={'Login'} />
