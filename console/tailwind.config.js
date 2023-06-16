/** @type {import('tailwindcss').Config} */
export default {
	content: ['./src/**/*.{html,js,svelte,ts}'],
	theme: {
		colors: {
			transparent: 'var(--color-transparent)',
			overlay: 'var(--color-overlay)',
			'base-0': 'var(--color-base-0)',
			'base-1': 'var(--color-base-1)',
			'base-2': 'var(--color-base-2)',
			'base-3': 'var(--color-base-3)',
			placeholder: 'var(--color-placeholder)',
			content: 'var(--color-content)',
			toolbar: 'var(--color-toolbar)',
			'toolbar-focus': 'var(--color-toolbar-focus)',
			'toolbar-content': 'var(--color-toolbar-content)',
			primary: 'var(--color-primary)',
			'primary-focus': 'var(--color-primary-focus)',
			'primary-content': 'var(--color-primary-content)',
			error: 'var(--color-error)',
			'error-outline': 'var(--color-error-outline)',
			'error-content': 'var(--color-error-content)',
			success: 'var(--color-success)',
			'success-outline': 'var(--color-success-outline)',
			'success-content': 'var(--color-success-content)',
			'fuchsia-void': '#140014',
			black: '#000000'
		},
		extend: {
			dropShadow: {
				card: '0 0px 2px rgb(0 0 0 / 0.1)'
			},
			opacity: {
				4: '.04'
			}
		}
	},
	plugins: []
}
