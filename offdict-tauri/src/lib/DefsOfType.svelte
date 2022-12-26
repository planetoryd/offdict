<script>
  import Def from "./Def.svelte";
  import { Tabs, Tab, TabList, TabPanel } from "svelte-tabs";
  //   import DefUnit from "./DefUnit.svelte";

  export let def;
  export let root = false;

  // Definitions with type placed in tabs
  // Definitions without type placed below
</script>

<div bind:this={def.deft}>
  {#if def.definitions && def.definitions[0]?.definitions && def.definitions[0].definitions[0] && (def.type_groups = def.definitions.filter((x) => x.type))}
    <!-- grouped by type -->
    <Tabs>
      <TabList>
        {#each def.type_groups as defi}
          <Tab>{defi.type}</Tab>
        {/each}
      </TabList>
      {#each def.type_groups as defi}
        <TabPanel>
          <Def def={defi} root={false} />
        </TabPanel>
      {/each}
    </Tabs>
    {#if (def.rest_defs = def.definitions.filter((x) => !x.type)).length > 0}
      <Def
        def={{
          ...def,
          definitions: def.rest_defs,
          type_groups: undefined,
          rest_defs: undefined,
        }}
        untyped={true}
        {root}
      />
    {/if}
  {:else}
    <Def {def} untyped={true} {root} />
  {/if}
</div>
