<script lang="ts">
    import ThemeSwitcher from "$lib/components/ThemeSwitcher.svelte";
</script>

<section class="grid gap-6">
    <h1 class="text-3xl font-bold tracking-tight">Welcome</h1>
    <p class="opacity-80">
        This app uses Tauri + SvelteKit + TailwindCSS + DaisyUI.
    </p>
    <ThemeSwitcher value="night"/>
    <div class="grid md:grid-cols-3 gap-4">
        <article class="card border p-4">
            <div class="card-body p-0">
                <h2 class="card-title">Tauri</h2>
                <p class="opacity-80">
                    Tauri is a framework for building tiny, blazing fast binaries for all major desktop platforms.
                </p>
                <div class="card-actions justify-end mt-2">
                    <a
                        href="https://tauri.app"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="btn btn-primary btn-sm"
                        >Learn More</a
                    >
                </div>
            </div>
        </article>

        <article class="card border">
            <div class="card-body p-4">
                <h2 class="card-title">SvelteKit</h2>
                <p class="opacity-80">
                    SvelteKit is a framework for building web applications of all sizes, with a beautiful development
                    experience and flexible filesystem-based routing.
                </p>
                <div class="card-actions justify-end mt-2">
                    <a
                        href="https://kit.svelte.dev"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="btn btn-secondary btn-sm"
                        >Learn More</a
                    >
                </div>
            </div>
		</article>
    </div>
</section>


