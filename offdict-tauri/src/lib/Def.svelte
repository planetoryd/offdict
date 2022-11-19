<script>
  import DefUnit from "./DefUnit.svelte";

  export let def;
</script>

<div>
  {#if def.definitions}
    {#each def.definitions as d}
      <DefUnit def={d} />
    {/each}
  {:else}
    <DefUnit {def} />
  {/if}
</div>
