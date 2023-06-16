export function randomPick<T>(items: T[]): T {
	return items[Math.floor(Math.random() * items.length)]
}

export function randomInt(min: number, max: number): number {
	return Math.floor(Math.random() * (max - min) + min)
}
