<script>
  export let def;
  let ele;
  import { onMount, onDestroy } from "svelte";
  import { inview } from "svelte-inview";
  onMount(() => {
    window.viewlist.set(ele, { inView: false, def });
  });
  onDestroy(() => {
    window.viewlist.delete(ele);
  });
</script>

<div
  class="explain"
  use:inview={{}}
  bind:this={ele}
  on:change={(event) => {
    const { inView, entry, scrollDirection, observer, node } = event.detail;
    window.viewlist.set(node, { inView, def });
    // else window.viewlist.delete(node)
  }}
>
  {#if def.CN}
    <div class="CN">{def.CN}</div>
  {/if}
  {#if def.EN}
    <div class="EN">{def.EN}</div>
  {/if}
  {#if def.examples}
    {#each def.examples as ex}
      {#if ex}
        <div class="example">
          {#if ex.CN}
            <div>{ex.CN}</div>
          {/if}
          {#if ex.EN}
            <div>{ex.EN}</div>
          {/if}
          {#if typeof ex === "string"}
            <div>{ex}</div>
          {/if}
        </div>
      {/if}
    {/each}
  {/if}
  {#if def.type}
    <div class="chip">{def.type}</div>
  {/if}
  {#if def.info}
    <div class="info">{def.info}</div>
  {/if}
  {#if def.related}
    {#each def.related as re}
      <div class="related">
        {#if re.CN}
          <div>{re.CN}</div>
        {/if}
        {#if re.EN}
          <div>{re.EN}</div>
        {/if}
        {#if typeof re === "string"}
          <div>{re}</div>
        {/if}
      </div>
    {/each}
  {/if}
  <div class="unknown">
    {#if def.definitions}
      {#if def.title}
        <div class="titl">{def.title}</div>
      {/if}

      {#each def.definitions as d}
        {#if d.CN}
          <div class="CN">{d.CN}</div>
        {/if}
        {#if d.EN}
          <div class="EN">{d.EN}</div>
        {/if}
      {/each}
    {/if}
  </div>
</div>

<style lang="scss">
  .example {
    margin-left: 20px;
    margin-top: 5px;
  }

  .unknown {
    .CN,
    .EN {
      background: white;
      color: #868e96;
    }
  }

  .CN,
  .EN,
  .titl {
    color: #f1f3f5;
    background: #868e96ce;
    padding: 4px;
    // border-radius: 2px;
    margin-top: 5px;
    margin-bottom: 5px;
    padding-left: 15px;
    padding-right: 10px;
    padding-top: 5px;
    padding-bottom: 5px;
    border-radius: 5px;
  }
  .info {
    color: #868e96;
    border-radius: 5px;
  }
  .explain {
    margin-bottom: 10px;
  }
  .related {
    margin-top: 15px;
  }
</style>
