<script lang="ts">
	import { onMount } from 'svelte'
	import { randomInt, randomPick } from './utils'

	export let numDotsFactor = 0.00007
	export let dotRadius = 5
	export let maxDistance = 320
	export let dotOpacity = 0.5
	export let lineOpacity = 0.8
	export let maxConnectCount = 3

	let canvas: HTMLCanvasElement
	let ctx: CanvasRenderingContext2D

	let width: number
	let height: number
	let cssScale = 1

	let numDots = 0
	let dotsX: number[] = []
	let dotsY: number[] = []
	let dotsDirX: number[] = []
	let dotsDirY: number[] = []

	function initDots() {
		let area = height * width
		numDots = numDotsFactor * area

		for (let i = 0; i < numDots; i += 1) {
			dotsX.push(randomInt(0, width))
			dotsY.push(randomInt(0, height))
			dotsDirX.push(randomPick([1, -1]) * Math.random())
			dotsDirY.push(randomPick([1, -1]) * Math.random())
		}
	}

	function drawDot(x: number, y: number) {
		ctx.beginPath()
		ctx.arc(x, y, dotRadius, 0, 2 * Math.PI)
		ctx.fill()
	}

	function reverse(x: number) {
		return -1 * Math.sign(x) * Math.random()
	}

	function updatePositions(deltaTime: number) {
		ctx.fillStyle = `rgba(255,255,255,${dotOpacity})`

		for (let i = 0; i < numDots; i += 1) {
			dotsX[i] += dotsDirX[i] * deltaTime
			dotsY[i] += dotsDirY[i] * deltaTime

			if (dotsX[i] >= width) {
				dotsX[i] = width - 1
				dotsDirX[i] = reverse(dotsDirX[i])
			}

			if (dotsX[i] < 0) {
				dotsX[i] = 0
				dotsDirX[i] = reverse(dotsDirX[i])
			}

			if (dotsY[i] >= height) {
				dotsY[i] = height - 1
				dotsDirY[i] = reverse(dotsDirY[i])
			}

			if (dotsY[i] < 0) {
				dotsY[i] = 0
				dotsDirY[i] = reverse(dotsDirY[i])
			}

			drawDot(dotsX[i], dotsY[i])
		}
	}

	function distance(x1: number, y1: number, x2: number, y2: number) {
		return Math.sqrt(Math.pow(x1 - x2, 2) + Math.pow(y1 - y2, 2))
	}

	function updateLines() {
		ctx.lineWidth = 2

		for (let i = 0; i < numDots; i += 1) {
			let count = 0

			for (let j = i + 1; j < numDots; j += 1) {
				if (count > maxConnectCount) {
					continue
				}

				let dist = distance(dotsX[i], dotsY[i], dotsX[j], dotsY[j])

				if (dist > maxDistance) {
					continue
				}

				ctx.strokeStyle = `rgba(255,255,255,${lineOpacity - dist / maxDistance})`
				ctx.beginPath()
				ctx.moveTo(dotsX[i], dotsY[i])
				ctx.lineTo(dotsX[j], dotsY[j])
				ctx.stroke()

				count += 1
			}
		}
	}

	function setWidthHeightAndCssScale() {
		cssScale = window.devicePixelRatio
		canvas.width = canvas.clientWidth * cssScale
		canvas.height = canvas.clientHeight * cssScale
		width = ctx.canvas.width
		height = ctx.canvas.height
	}

	onMount(() => {
		ctx = canvas.getContext('2d')!

		setWidthHeightAndCssScale()
		initDots()

		let interval = setInterval(() => {
			setWidthHeightAndCssScale()
			ctx.clearRect(0, 0, width, height)
			updateLines()
			updatePositions(1)
		}, 32)

		return () => clearInterval(interval)
	})
</script>

<div class="bg-black">
	<canvas bind:this={canvas} class="h-screen w-screen bg-[#140014] opacity-25" />
</div>
