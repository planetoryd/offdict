<script lang="ts">
  // import { slide } from "svelte/transition";
  // import { sineInOut } from "svelte/easing";
  // Transition sucks

  import { invoke } from "@tauri-apps/api/tauri";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import toast, { Toaster } from "svelte-french-toast";
  import Def from "./lib/Def.svelte";
  import { Tabs, Tab, TabList, TabPanel } from "svelte-tabs";
  import DefUnit from "./lib/DefUnit.svelte";
  let dropdown = false;
  let candidates = [
    {
      word: "something",
      id: 0,
    },
  ];
  let currentWord = undefined;
  let inputWord;
  let word_defs; // Def obj for single word
  let def_list = []; // Final list of defs

  listen("error", (e) => {});

  // show defs for currentWord
  function show() {
    invoke("defs", { query: currentWord })
      .then((r) => {
        console.log(r);
        word_defs = r;
        if (word_defs) {
          def_list = word_defs.definitions;
          console.log(def_list);
        }
      })
      .catch((e) => {
        console.log(e);
      });
  }
</script>

<main>
  <div class="first">
    <div class="input-group">
      <input
        type="text"
        class="form-input"
        placeholder={currentWord}
        bind:value={inputWord}
        on:input={async () => {
          // candidates = ;
          let res = await invoke("candidates", { query: inputWord });
          // console.log(await invoke("defs", { query: currentWord }));
          candidates = res.map((e, i) => ({ word: e, id: i }));
          // console.log(candidates);
          // toast.success("Always at the bottom.", {
          //   position: "bottom-center",
          // });
        }}
        on:keydown={(e) => {
          dropdown = true;
          if (e.key === "ArrowDown") {
            document.querySelector(".first .menu-item").firstChild.focus();
            e.stopPropagation();
            e.preventDefault();
          }
          if (e.key === "Enter") {
            document.querySelector(".first .menu-item").firstChild.click();
            e.stopPropagation();
            e.preventDefault();
          }
        }}
        on:focus={() => (dropdown = true)}
        on:click={(e) => {
          e.stopPropagation();
          e.preventDefault();
        }}
      />
      <button class="btn btn-secondary input-group-btn">settings</button>
    </div>
    {#if dropdown}
      <!-- <ul class="menu" transition:slide={{ duration: 5, easing: sineInOut }}> -->
      <ul class="menu">
        <!-- menu header text -->
        <li class="divider" data-content="Local" />
        <!-- menu item standard -->
        {#each candidates as candi (candi.id)}
          <!-- svelte-ignore a11y-no-noninteractive-tabindex -->
          <li
            class="menu-item"
            on:click={() => {
              dropdown = false;
              currentWord = candi.word;
              inputWord = currentWord;
              show();
            }}
            tabindex={candi.id}
            on:keydown={(e) => {
              if (e.key === "ArrowDown") {
                e.target.parentElement?.nextSibling?.firstChild?.focus();
                e.stopPropagation();
                e.preventDefault();
              }
              if (e.key === "ArrowUp") {
                e.target.parentElement?.previousSibling?.firstChild?.focus();
                e.stopPropagation();
                e.preventDefault();
              }
            }}
          >
            <a href="#">
              <i class="icon icon-link" />
              {candi.word}
            </a>
          </li>
        {/each}
        <!-- menu item with form control -->
        <!-- menu divider -->
        <!-- <li class="divider" /> -->
        <!-- menu item with badge -->
        <!-- <li class="menu-item">
          <a href="#">
            <i class="icon icon-link" /> Settings
          </a>
          <div class="menu-badge">
            <label class="label label-primary">2</label>
          </div>
        </li> -->
      </ul>
    {/if}
  </div>

  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div
    class="container"
    on:click={() => {
      dropdown = false;
    }}
  >
    {#each def_list as def}
      <div class="card">
        <!-- <div class="card-image">
        <img src="public/svelte.svg">
      </div> -->
        <div class="card-header">
          <div class="card-title h5">
            {def?.word}
          </div>
          <div class="card-subtitle text-gray">
            {def?.pronunciation?.join(" / ") || ""}
          </div>
        </div>
        <div class="card-body">
          {#if def && def.definitions}
            {#if def.definitions[0]?.definitions && def.definitions[0].definitions[0] && (def.type_groups = def.definitions.filter((x) => x.type))}
              <!-- grouped by type -->
              <Tabs>
                <TabList>
                  {#each def.type_groups as defi}
                    <Tab>{defi.type}</Tab>
                  {/each}
                </TabList>
                {#each def.type_groups as defi}
                  <TabPanel>
                    <Def def={defi} />
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
                />
              {/if}
            {:else}
              <Def {def} />
            {/if}
          {/if}
          <!-- <pre class="code">
            <code>{JSON.stringify(def, null, 2)}</code>
          </pre> -->
          {#if def.etymology}
            {#each def.etymology as ety}
              <div>{ety}</div>
            {/each}
          {/if}
        </div>
        <div class="card-footer">{def?.dictName}</div>
      </div>
    {/each}
  </div>
  <Toaster />
</main>

<style type="text/scss">
  .first {
    z-index: 99;
    padding-top: 10px;
    position: fixed;
    top: 0;
    width: 100%;
    padding-right: 18px;
    padding-left: 15px;
    padding-top: 15px;
    -webkit-backdrop-filter: blur(5px);
    backdrop-filter: blur(10px);
    padding-bottom: 15px;
    background-color: rgba($color: rgb(0, 68, 255), $alpha: 0.02);
  }

  html,
  body {
    margin: 0;
    padding: 0;
  }
  .card {
    margin: 10px;
    margin-left: 8px;
  }
  .menu {
    margin-top: 5px;
    margin-bottom: 25px;
  }
  .card:first-child {
    margin-top: 75px;
  }
  :global(.svelte-tabs li.svelte-tabs__tab:focus) {
    outline: none;
  }

  .card-footer {
    color: lightgrey;
  }
</style>
