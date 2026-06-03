<script>
  import { onMount } from 'svelte';
  import '../app.css';
  let { children } = $props();

  onMount(() => {
    const loader = document.getElementById('app-mounting-loader');
    if (loader) {
      loader.remove();
    }
  });
</script>

{@render children()}
