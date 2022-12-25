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
  {#if def.type || def.CN || def.EN}
    <div class="def_basic">
      {#if def.type}
        <span class="chip">{def.type}</span>
      {/if}
      {#if def.CN}
        <span class="CN">{def.CN}</span>
      {/if}
      {#if def.EN}
        <div class="EN">{def.EN}</div>
      {/if}
    </div>
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
  {#if def.info}
    <div class="info">{def.info}</div>
  {/if}
  {#if def.tip}
    <div class="info">{def.tip}</div>
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
        <span class="titl">{def.title}</span>
      {/if}

      {#each def.definitions as d}
        {#if d.CN}
          <span class="CN">{d.CN}</span>
        {/if}
        {#if d.EN}
          <span class="EN">{d.EN}</span>
        {/if}
      {/each}
    {/if}
  </div>
</div>

<style lang="scss">
  .explain {
    margin-bottom: 10px;
    .example {
      margin-left: 25px;
      margin-top: 5px;
    }
    .info {
      color: #78b89f;
      border-radius: 5px;
    }
    .def_basic {
      background: rgba(165, 165, 165, 0.199);
      padding: 5px;
      border-radius: 5px;
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
      color: #000000d3;
      // background: rgba(255, 255, 255, 0.953);
      // padding: 4px;
      // border-radius: 2px;
      padding-top: 0;
      padding-bottom: 0;
      margin-top: 5px;
      margin-bottom: 5px;
      padding-left: 15px;
      padding-right: 10px;
      border-radius: 5px;
    }
    .related {
      margin-top: 15px;
    }
  }
  :global(span) {
    display: inline;
    padding-top: 2px;
    padding-bottom: 2px;
    white-space: pre-line;
    overflow-wrap: break-word;
    word-break: break-word;
  }
  .chip {
    background: rgba(0, 54, 116, 0.352);
    margin-top: 4px;
    color: white;
    padding: 2px;
    padding-left: 8px;
    padding-right: 8px;
  }
  :global(body.dark) {
    $text: rgba(255, 255, 255, 0.966);
    .explain {
      color: $text;
      background: transparent;
      span {
        &.CN,
        &.EN {
          background: rgba(245, 245, 245, 0);
          color: $text;
        }
        &.titl {
          color: $text;
        }
      }
      .unknown {
        :nth-child(odd) {
          &.CN,
          &.EN {
            color: #b3e4d3d8;
          }
        }
        :nth-child(even) {
          &.CN,
          &.EN {
            color: #c9d4cde5;
          }
        }
      }

      div.def_basic,
      .def_basic .EN {
        color: $text;
      }
    }
    .chip {
      background: rgba(170, 115, 64, 0.5);
      color: $text;
    }
  }
</style>
