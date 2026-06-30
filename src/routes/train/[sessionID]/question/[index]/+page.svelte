<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { session, loadQuestion, answerCurrent } from '$lib/stores/session';
	import { getRecordingUrl } from '$lib/api/audio';
	import type { QuestionDto } from '$lib/api/quiz';

	const id = page.params.sessionID;

	let question = $state<QuestionDto | null>(null); // local snapshot (store clears `current` on answer)
	let audioUrl = $state<string | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let feedback = $state<{ correct: boolean; correct_name: string; guess: string } | null>(null);

	onMount(async () => {
		if (!$session || $session.id !== id) {
			goto('/train');
			return;
		}
		try {
			const q = await loadQuestion();
			if (!q) {
				goto(`/train/${id}/result`);
				return;
			}
			question = q;
			audioUrl = await getRecordingUrl(q.recording_id);
		} catch (e) {
			error = (e as Error)?.message ?? 'Failed to load question.';
		} finally {
			loading = false;
		}
	});

	async function choose(choice: string) {
		if (feedback) return; // one answer per question
		try {
			const rec = await answerCurrent(choice);
			if (rec) feedback = { correct: rec.correct, correct_name: rec.correct_name, guess: choice };
		} catch (e) {
			error = (e as Error)?.message ?? 'Failed to submit answer.';
		}
	}

	function next() {
		if (!$session || $session.finished) goto(`/train/${id}/result`);
		else goto(`/train/${id}/question/${$session.index}`);
	}
</script>

{#if loading}
	<p class="opacity-70">Loading…</p>
{:else if error}
	<div class="alert alert-error">{error}</div>
{:else if question}
	<h1 class="text-xl font-semibold mb-4">Which bird is this?</h1>

	{#if audioUrl}
		<audio class="w-full mb-6" src={audioUrl} controls preload="auto"></audio>
	{/if}

	<div class="grid sm:grid-cols-2 gap-3">
		{#each question.choices as c}
			<button
				class="btn"
				class:btn-outline={!feedback}
				class:btn-success={feedback && c === feedback.correct_name}
				class:btn-error={feedback && c === feedback.guess && !feedback.correct}
				disabled={!!feedback}
				onclick={() => choose(c)}
			>
				{c}
			</button>
		{/each}
	</div>

	{#if feedback}
		<div class="mt-6 flex items-center gap-4">
			<span class={feedback.correct ? 'text-success' : 'text-error'}>
				{feedback.correct ? 'Correct!' : `Nope — it was ${feedback.correct_name}.`}
			</span>
			<button class="btn btn-primary" onclick={next}>
				{$session?.finished ? 'See results' : 'Next'}
			</button>
		</div>
	{/if}
{/if}
