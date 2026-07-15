<script lang="ts">
	// A small emoji picker for pack icons. `value` is the chosen emoji, or null
	// for "no icon" (the library falls back to a rotating default).
	let {
		value = $bindable(null),
		disabled = false,
		onSelect
	}: { value?: string | null; disabled?: boolean; onSelect?: (icon: string | null) => void } =
		$props();

	function pick(icon: string | null) {
		value = icon;
		onSelect?.(icon);
	}

	// Curated bird/nature/theme set — enough variety without a full emoji keyboard.
	const ICONS = [
		'🐦', '🦅', '🦉', '🦜', '🦆', '🦢', '🕊️', '🐧',
		'🦃', '🦩', '🐤', '🪶', '🥚', '🌲', '🌿', '🍃',
		'🌊', '🏔️', '🏝️', '🏡', '🌅', '🌙', '⭐', '🔥',
		'🎵', '🎶', '🎧', '📚', '🌍', '🧭', '🔭', '🎯'
	];
</script>

<div class="flex flex-wrap gap-1.5" role="group" aria-label="Pack icon">
	<button
		type="button"
		{disabled}
		onclick={() => pick(null)}
		aria-pressed={value == null}
		title="No icon (use default)"
		class="flex h-9 w-9 items-center justify-center rounded-lg border text-xs transition-colors disabled:opacity-50 {value ==
		null
			? 'border-be-primary bg-be-primary/10 text-be-fg'
			: 'border-be-border text-be-muted-fg hover:bg-be-secondary'}"
	>
		∅
	</button>
	{#each ICONS as icon (icon)}
		<button
			type="button"
			{disabled}
			onclick={() => pick(icon)}
			aria-pressed={value === icon}
			class="flex h-9 w-9 items-center justify-center rounded-lg border text-xl leading-none transition-colors disabled:opacity-50 {value ===
			icon
				? 'border-be-primary bg-be-primary/10'
				: 'border-be-border hover:bg-be-secondary'}"
		>
			{icon}
		</button>
	{/each}
</div>
