<script lang="ts">
	import AppHeader from '$lib/components/AppHeader.svelte';
</script>

<AppHeader />

<main class="mx-auto max-w-3xl px-8 py-10">
	<div class="mb-8">
		<h2 class="font-be-serif mb-2 text-3xl font-bold leading-tight">How Birdet works</h2>
		<p class="text-sm text-be-muted-fg">
            A guide to Birdet: training, the review schedule, and using the app in general.
		</p>
	</div>

	<!-- TOC -->
	<nav class="mb-10 rounded-xl border border-be-border bg-be-card p-5">
		<p class="font-be-mono mb-3 text-xs uppercase tracking-widest text-be-muted-fg">Contents</p>
		<ul class="grid grid-cols-1 gap-1.5 text-sm sm:grid-cols-2">
			<li><a href="#overview" class="text-be-primary hover:underline">1 · The concept</a></li>
			<li><a href="#setup" class="text-be-primary hover:underline">2 · First-time setup</a></li>
			<li><a href="#session" class="text-be-primary hover:underline">3 · A training session</a></li>
			<li><a href="#question" class="text-be-primary hover:underline">4 · Answering a question</a></li>
			<li><a href="#schedule" class="text-be-primary hover:underline">5 · How the schedule works</a></li>
			<li><a href="#modes" class="text-be-primary hover:underline">6 · Session modes</a></li>
			<li><a href="#navigation" class="text-be-primary hover:underline">7 · Getting around</a></li>
			<li><a href="#collection" class="text-be-primary hover:underline">8 · Managing your collection</a></li>
			<li><a href="#shortcuts" class="text-be-primary hover:underline">9 · Keyboard shortcuts</a></li>
		</ul>
	</nav>

	<div class="space-y-12">
		<!-- 1 · Overview -->
		<section id="overview">
			<h3 class="font-be-serif mb-3 text-xl font-bold">1 · The idea</h3>
			<p class="mb-3 text-sm leading-relaxed text-be-fg">
				Birdet trains you to recognise birds <em>by ear</em>. You hear a recording, see its
				spectrogram, and pick the right species from four choices. Based on your answers, the app
                schedules each bird so you review it right before you'd forget it (ideally). This way,
                birds you find difficult are shown to you more often, while birds that distinctive don't 
                clutter your learning experience as often (*cough*Hadedas*cough*).
			</p>
			<p class="text-sm leading-relaxed text-be-muted-fg">
				This type of scheduling is called <strong class="text-be-fg">spaced repetition</strong> 
                and is conceptually the same as what apps like Anki do.
			</p>
		</section>

		<!-- 2 · Setup -->
		<section id="setup">
			<h3 class="font-be-serif mb-3 text-xl font-bold">2 · First-time setup</h3>
			<p class="mb-4 text-sm leading-relaxed text-be-fg">
				Birdet starts with a few demo birds, but you're expected to curate your own
				collection of local species. The steps to do this are given below.
			</p>
			<ol class="space-y-3 text-sm leading-relaxed">
				<li class="flex gap-3">
					<span class="font-be-mono shrink-0 text-be-muted-fg">1.</span>
					<span>
						<strong>Add API keys.</strong> In <a href="/settings" class="text-be-primary hover:underline">Settings</a>,
						paste a free eBird token and a free Xeno-Canto key. eBird tells Birdet which species
						live in a region; Xeno-Canto provides the recordings. To get an eBird API key, visit
                        the <a href="https://ebird.org/api/keygen" target="_blank" rel="noopener noreferrer">ebird.org/api/keygen</a> (and create an account if you don't have one). To obtain the Xeno-Canto key, you will need to <a href="https://xeno-canto.org/" target="_blank" rel="noopener noreferrer">create a Xeno-Canto account</a> and verify your email address (the key will then be available under your Xeno-Canto profile details).
					</span>
				</li>
				<li class="flex gap-3">
					<span class="font-be-mono shrink-0 text-be-muted-fg">2.</span>
					<span>
						<strong>Import birds.</strong> From <a href="/library" class="text-be-primary hover:underline">Library → Import</a>,
						pick a country and (optionally) a subregion. Birdet ranks that region's species by how commonly they're actually reported, then downloads recordings for the most common ones. You can also search for and add individual species by name. Each bird is imported with one or more (you can specify this) audio recordings from Xeno-Canto. <em>Note: this import process can take a while due to rate-limiting by the eBird/Xeno-Canto APIs, plus the time it takes to download the audio files.</em>
					</span>
				</li>
				<li class="flex gap-3">
					<span class="font-be-mono shrink-0 text-be-muted-fg">3.</span>
					<span>
						<strong>Start training.</strong> Head to the home screen and hit
						<em>Start session</em>.
					</span>
				</li>
			</ol>
		</section>

		<!-- 3 · Session -->
		<section id="session">
			<h3 class="font-be-serif mb-3 text-xl font-bold">3 · A training session</h3>
			<p class="mb-3 text-sm leading-relaxed text-be-fg">
				The home screen's <strong>Learn &amp; Review</strong> panel is your friend. The two
				numbers on it tell you what's waiting:
			</p>
			<div class="mb-4 grid grid-cols-1 gap-3 sm:grid-cols-2">
				<div class="rounded-lg border border-be-border bg-be-card p-4">
					<p class="font-be-serif text-lg font-bold text-be-accent">due</p>
					<p class="text-sm text-be-muted-fg">Birds you've learned before that are scheduled for review right now.</p>
				</div>
				<div class="rounded-lg border border-be-border bg-be-card p-4">
					<p class="font-be-serif text-lg font-bold">new</p>
					<p class="text-sm text-be-muted-fg">Birds in your collection you haven't studied yet.</p>
				</div>
			</div>
			<p class="text-sm leading-relaxed text-be-muted-fg">
				A normal session clears your <span class="text-be-accent">due</span> reviews first, then
				introduces a handful of <strong>new</strong> birds (the cap is set in Settings — default 10).
				Once you've cleared everything, the panel shows <em>All caught up</em> and tells you when the
				next review is due.
			</p>
		</section>

		<!-- 4 · Question -->
		<section id="question">
			<h3 class="font-be-serif mb-3 text-xl font-bold">4 · Answering a question</h3>
			<p class="mb-4 text-sm leading-relaxed text-be-fg">
				Each question plays one recording (picked randomly from the recordings locally available for that bird) and shows its spectrogram. Once you're ready, choose:
			</p>
			<ul class="mb-5 space-y-2.5 text-sm leading-relaxed">
				<li class="flex gap-2.5"><span class="text-be-primary">▸</span> <span><strong>Play / replay</strong> the call as many times as you like before answering.</span></li>
				<li class="flex gap-2.5"><span class="text-be-primary">▸</span> <span>Pick one of the <strong>four species</strong> options. You'll see immediately whether you were right, with the correct name.</span></li>
				<li class="flex gap-2.5"><span class="text-be-primary">▸</span> <span>Not sure? <strong>Skip</strong> (“I don't know”) reveals the answer without guessing — it counts as a miss so the bird comes back sooner. It's better to pick this instead of guessing when you're completely unsure.</span></li>
				<li class="flex gap-2.5"><span class="text-be-primary">▸</span> <span>Turn on <strong>auto-play</strong> in Settings to hear each call the moment it loads.</span></li>
			</ul>

			<p class="mb-3 text-sm font-medium">The tags in the top-right corner tell you about the current card:</p>
			<div class="space-y-2.5">
				<div class="flex items-center gap-3">
					<span class="font-be-mono shrink-0 rounded-full bg-be-primary/15 px-2.5 py-1 text-xs text-be-primary">new bird</span>
					<span class="text-sm text-be-muted-fg">A species you're seeing for the first time.</span>
				</div>
				<div class="flex items-center gap-3">
					<span class="font-be-mono shrink-0 rounded-full bg-be-accent/15 px-2.5 py-1 text-xs text-be-accent">review</span>
					<span class="text-sm text-be-muted-fg">A bird that came due — you've seen it before.</span>
				</div>
				<div class="flex items-center gap-3">
					<span class="font-be-mono shrink-0 rounded-full bg-be-secondary px-2.5 py-1 text-xs text-be-secondary-fg">3 / 10 new</span>
					<span class="text-sm text-be-muted-fg">New birds introduced this session, out of your cap (only in modes that add new birds).</span>
				</div>
				<div class="flex items-center gap-3">
					<span class="font-be-mono shrink-0 rounded-full bg-be-secondary px-2.5 py-1 text-xs text-be-secondary-fg">practice 4 / 10</span>
					<span class="text-sm text-be-muted-fg">You're in a practice run (see modes below) — question count, no effect on your schedule.</span>
				</div>
			</div>
		</section>

		<!-- 5 · Schedule -->
		<section id="schedule">
			<h3 class="font-be-serif mb-3 text-xl font-bold">5 · How the schedule works</h3>
			<p class="mb-4 text-sm leading-relaxed text-be-fg">
				Every bird moves through a few stages. Getting it right pushes the next review further out;
				getting it wrong pulls it back.
			</p>
			<div class="space-y-3">
				<div class="rounded-lg border border-be-border bg-be-card p-4">
					<p class="mb-1 text-sm font-semibold">Learning</p>
					<p class="text-sm text-be-muted-fg">
						A new bird starts here. It comes back after ~1 minute, then ~10 minutes — quick repeats
						within the same session to lock it in. Miss it and it restarts these steps.
					</p>
				</div>
				<div class="rounded-lg border border-be-border bg-be-card p-4">
					<p class="mb-1 text-sm font-semibold">Reviewing</p>
					<p class="text-sm text-be-muted-fg">
						After the learning steps the bird graduates to day-scale intervals — 1 day, then a few
						days, then longer. Each correct answer multiplies the gap; the interval grows as the bird
						gets easier for you.
					</p>
				</div>
				<div class="rounded-lg border border-be-border bg-be-card p-4">
					<p class="mb-1 text-sm font-semibold">Lapses</p>
					<p class="text-sm text-be-muted-fg">
						Miss a bird you'd learned and it drops back into short steps and becomes a little
						“harder”, so it'll show up more often until you're solid again.
					</p>
				</div>
			</div>
			<p class="mt-4 text-sm leading-relaxed text-be-muted-fg">
				You don't manage any of this by hand — just answer honestly and the schedule takes care of
				itself. Check <a href="/stats" class="text-be-primary hover:underline">Stats</a> to see each
				bird's state and accuracy.
			</p>
		</section>

		<!-- 6 · Modes -->
		<section id="modes">
			<h3 class="font-be-serif mb-3 text-xl font-bold">6 · Session modes</h3>
			<p class="mb-4 text-sm leading-relaxed text-be-fg">
				From the home screen you can pick how a session is scoped:
			</p>
			<div class="divide-y divide-be-border overflow-hidden rounded-xl border border-be-border bg-be-card">
				<div class="px-5 py-4">
					<p class="text-sm font-semibold">Start session <span class="font-normal text-be-muted-fg">(Learn &amp; Review)</span></p>
					<p class="text-sm text-be-muted-fg">Due reviews first, then new birds up to your cap. The everyday choice.</p>
				</div>
				<div class="px-5 py-4">
					<p class="text-sm font-semibold text-be-accent">Review due</p>
					<p class="text-sm text-be-muted-fg">Only clears the birds that are due. Introduces nothing new.</p>
				</div>
				<div class="px-5 py-4">
					<p class="text-sm font-semibold">New only</p>
					<p class="text-sm text-be-muted-fg">Introduces new birds and skips the review queue.</p>
				</div>
				<div class="px-5 py-4">
					<p class="text-sm font-semibold">Practice anyway</p>
					<p class="text-sm text-be-muted-fg">
						A fixed-length drill over your collection that <strong>ignores the schedule</strong> —
						great when you're caught up but want to keep sharp. Answers still count toward accuracy
						but don't change due dates.
					</p>
				</div>
			</div>
			<p class="mt-4 text-sm leading-relaxed text-be-muted-fg">
				Every pack on the home screen (and in the Library) runs the same schedule, just narrowed to
				that set of species.
			</p>
		</section>

		<!-- 7 · Navigation -->
		<section id="navigation">
			<h3 class="font-be-serif mb-3 text-xl font-bold">7 · Getting around</h3>
			<div class="space-y-3 text-sm leading-relaxed">
				<p><a href="/" class="font-semibold text-be-primary hover:underline">Home</a> — start a session, pick a pack, or see what's due.</p>
				<p><a href="/stats" class="font-semibold text-be-primary hover:underline">Stats</a> — your progress: how many birds you've learned, accuracy, and each bird's schedule state.</p>
				<p>
					<a href="/library" class="font-semibold text-be-primary hover:underline">Library</a> — two tabs.
					<strong>Packs</strong> are curated sets you can edit or train. <strong>Birds</strong> is every
					species you've collected; each shows which packs it's in. Click a bird to open its detail
					page, where you can <strong>play, add, or delete individual recordings</strong> — handy if a
					downloaded clip is poor quality. This is also where you <em>Import</em> or make a
					<em>New pack</em>.
				</p>
				<p><a href="/settings" class="font-semibold text-be-primary hover:underline">Settings</a> — new-birds-per-session cap, auto-play, answer mode, and your API keys.</p>
			</div>
		</section>

		<!-- 8 · Managing your collection -->
		<section id="collection">
			<h3 class="font-be-serif mb-3 text-xl font-bold">8 · Managing your collection</h3>
			<p class="mb-5 text-sm leading-relaxed text-be-fg">
				Everything you've imported lives in the <a href="/library" class="text-be-primary hover:underline">Library</a>,
				split into <strong>Packs</strong> and <strong>Birds</strong>. Here's how to manage it.
			</p>

			<!-- Birds -->
			<h4 class="font-be-serif mb-2 text-base font-semibold">Your birds</h4>
			<p class="mb-3 text-sm leading-relaxed text-be-muted-fg">
				The <strong>Birds</strong> tab lists every species you've collected. Search by common or
				scientific name, and each row shows a <span class="font-be-mono rounded-full bg-be-secondary/50 px-2 py-0.5 text-[11px] text-be-muted-fg">pack</span>
				tag for every pack it belongs to (click a tag to jump to that pack). The ♪ count is how many
				recordings that bird has. Click a bird to open its detail page.
			</p>

			<!-- Recordings -->
			<h4 class="font-be-serif mb-2 mt-6 text-base font-semibold">A bird's recordings</h4>
			<p class="mb-3 text-sm leading-relaxed text-be-muted-fg">
				On a bird's detail page you manage its individual audio clips. It's a good idea to make sure your birds have more than one clip, otherwise it is often easy to associate a bird with some irrelevent characteristic of a specific recording. Having multiple recordings forces you to actually recognise the bird song/call. 
			</p>
			<ul class="mb-3 space-y-2.5 text-sm leading-relaxed">
				<li class="flex gap-2.5"><span class="text-be-primary">▸</span> <span><strong>Play</strong> any recording to listen to it. Each shows its Xeno-Canto quality rating and type — <span class="font-be-mono rounded-full bg-be-secondary px-2 py-0.5 text-[11px] text-be-muted-fg">song</span> / <span class="font-be-mono rounded-full bg-be-secondary px-2 py-0.5 text-[11px] text-be-muted-fg">call</span> and <span class="font-be-mono rounded-full border border-be-border px-2 py-0.5 text-[11px] text-be-muted-fg">q:A</span> (A is best) — plus a link back to the source on Xeno-Canto.</span></li>
				<li class="flex gap-2.5"><span class="text-be-primary">▸</span> <span><strong>Add recordings</strong> pulls more clips for that species from Xeno-Canto. Filter by minimum quality and by song vs call, then tick the ones you want and download them.</span></li>
				<li class="flex gap-2.5"><span class="text-be-primary">▸</span> <span><strong>Delete</strong> removes a bad clip (and its file). A bird must keep at least one recording, so the last one can't be deleted — remove the bird instead.</span></li>
			</ul>

			<!-- Packs -->
			<h4 class="font-be-serif mb-2 mt-6 text-base font-semibold">Packs</h4>
			<p class="mb-3 text-sm leading-relaxed text-be-muted-fg">
				A <strong>pack</strong> is a named subset of your birds — “garden birds”, “warblers”, “trip to
				the coast”. Packs don't copy anything; they're just filters.
			</p>
			<ul class="mb-3 space-y-2.5 text-sm leading-relaxed">
				<li class="flex gap-2.5"><span class="text-be-primary">▸</span> <span><strong>Create</strong> one with <em>New pack</em> in the Library, or send birds straight into a new or existing pack while importing.</span></li>
				<li class="flex gap-2.5"><span class="text-be-primary">▸</span> <span><strong>Edit</strong> a pack (the <em>Edit</em> button on the Packs tab) to rename it, add birds from your library, or remove birds. Removing a bird from a pack leaves the bird itself untouched.</span></li>
				<li class="flex gap-2.5"><span class="text-be-primary">▸</span> <span><strong>Train</strong> straight from the pack, or <strong>Delete</strong> the pack — deleting only removes the grouping, never the birds or recordings.</span></li>
			</ul>

			<!-- Importing more -->
			<h4 class="font-be-serif mb-2 mt-6 text-base font-semibold">Adding more birds</h4>
			<p class="text-sm leading-relaxed text-be-muted-fg">
				Use <em>Import</em> any time to grow your collection — pull the most common species from
				another region, or search for specific birds by name. See
				<a href="#setup" class="text-be-primary hover:underline">First-time setup</a> for the details.
				Re-importing a bird you already have won't create duplicates.
			</p>
		</section>

		<!-- 9 · Shortcuts -->
		<section id="shortcuts">
			<h3 class="font-be-serif mb-3 text-xl font-bold">9 · Keyboard shortcuts</h3>
			<p class="mb-4 text-sm text-be-muted-fg">During a question:</p>
			<div class="overflow-hidden rounded-xl border border-be-border bg-be-card">
				<table class="w-full text-sm">
					<tbody class="divide-y divide-be-border">
						<tr><td class="px-5 py-2.5"><kbd class="font-be-mono rounded bg-be-secondary px-1.5 py-0.5 text-xs">Space</kbd> / <kbd class="font-be-mono rounded bg-be-secondary px-1.5 py-0.5 text-xs">K</kbd></td><td class="px-5 py-2.5 text-be-muted-fg">Play / pause the call</td></tr>
						<tr><td class="px-5 py-2.5"><kbd class="font-be-mono rounded bg-be-secondary px-1.5 py-0.5 text-xs">R</kbd></td><td class="px-5 py-2.5 text-be-muted-fg">Replay from the start</td></tr>
						<tr><td class="px-5 py-2.5"><kbd class="font-be-mono rounded bg-be-secondary px-1.5 py-0.5 text-xs">1</kbd>–<kbd class="font-be-mono rounded bg-be-secondary px-1.5 py-0.5 text-xs">9</kbd></td><td class="px-5 py-2.5 text-be-muted-fg">Pick that answer choice</td></tr>
						<tr><td class="px-5 py-2.5"><kbd class="font-be-mono rounded bg-be-secondary px-1.5 py-0.5 text-xs">S</kbd></td><td class="px-5 py-2.5 text-be-muted-fg">Skip (I don't know)</td></tr>
						<tr><td class="px-5 py-2.5"><kbd class="font-be-mono rounded bg-be-secondary px-1.5 py-0.5 text-xs">Enter</kbd> / <kbd class="font-be-mono rounded bg-be-secondary px-1.5 py-0.5 text-xs">→</kbd></td><td class="px-5 py-2.5 text-be-muted-fg">Next question</td></tr>
					</tbody>
				</table>
			</div>
		</section>
	</div>

	<div class="mt-12 flex justify-center border-t border-be-border pt-8">
		<a href="/" class="flex items-center gap-2 rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90">
			Start training
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
		</a>
	</div>
</main>
