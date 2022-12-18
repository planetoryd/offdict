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
  import { inview } from "svelte-inview";

  export let dropdown = false;
  let candidates = [
    {
      word: "something",
      id: 0,
    },
  ];
  let qucik_res = [];
  let currentWord = undefined;
  let currentIndex = -1;
  let inputWord;
  let def_list = []; // Final list of defs
  let show_not_found = true;

  listen("error", (e) => {});
  listen("clip", (e) => {
    inputWord = e.payload;
    show_not_found = true;
    onInput();
  });

  listen("importing", (e) => {
    toast.loading("importing " + e.payload, {
      position: "bottom-center",
      id: e.payload as string,
    });
  });

  listen("imported", (e) => {
    toast.dismiss(e.payload as string);
    toast.success("imported " + e.payload, {
      position: "bottom-center",
      duration: 800,
    });
  });

  // show defs for currentWord
  function show() {
    invoke("defs", { query: currentWord })
      .then((r) => {
        console.log(r);
        def_list = r as [];
      })
      .catch((e) => {
        console.log(e);
      });
  }

  async function onInput() {
    // candidates = ;
    let qucik_res: [any] = await invoke("defs", {
      query: inputWord,
      fuzzy: false,
    });
    // console.log(await invoke("defs", { query: currentWord }));
    console.log(def_list);
    candidates = qucik_res.map((e, i) => ({ word: e.word, id: i }));
    if (qucik_res[0]) {
      def_list = qucik_res; // update only when results present
      currentWord = qucik_res[0].word;
      currentIndex = 0;
      // show();
    } else {
      // show loading status
      
      let fuzzy_res: [any] = await invoke("defs", {
        query: inputWord,
        fuzzy: true,
      });
      def_list = fuzzy_res;
      console.log("fuzzy", fuzzy_res);
      if (show_not_found)
        toast.error("not found", { position: "bottom-center", duration: 800 });
    }
    // console.log(candidates);
    // toast.success("Always at the bottom.", {
    //   position: "bottom-center",
    // });
  }

  function navNextDef(goPrev) {
    // let arr = Array.from(window.viewlist)
    //   .filter((x) => x[1]?.inView)
    //   .sort(
    //     (a, b) =>
    //       a[0].getBoundingClientRect().y - b[0].getBoundingClientRect().y
    //   );
    let lastVisiblePlusOne: number = -1;
    let arr = Array.from(window.viewlist);
    arr.sort(
      (a, b) => a[0].getBoundingClientRect().y - b[0].getBoundingClientRect().y
    );
    // Assumption: the Map iterates in the order of DOM elements
    if (goPrev) arr.reverse();
    for (let k in arr) {
      if (!arr[parseInt(k) + 1]) break; // js tf
      if (arr[k][1].inView && !arr[parseInt(k) + 1][1].inView) {
        lastVisiblePlusOne = parseInt(k) + 1;
        break;
      }
    }
    if (lastVisiblePlusOne > -1) {
      let e = arr[lastVisiblePlusOne][0];
      // e.scrollIntoView({
      //   behavior: "smooth",
      //   block: "start",
      // });
      if (!goPrev)
        window.scrollTo({
          top: e.getBoundingClientRect().y + window.scrollY - 400,
          behavior: "smooth",
        });
      else
        window.scrollTo({
          top: e.getBoundingClientRect().y + window.scrollY,
          behavior: "smooth",
        });
    } else {
      // should be
      if (!goPrev)
        window.scrollTo({
          top: document.body.scrollHeight,
          behavior: "smooth",
        });
      else window.scrollTo(0, 0);
    }
  }
  window.nav = navNextDef;
</script>

<main>
  <div class="first">
    <div class="input-group">
      <input
        type="text"
        class="form-input"
        placeholder={currentWord}
        bind:value={inputWord}
        on:input={(e) => {
          show_not_found = false;
          onInput();
        }}
        on:keydown={(e) => {
          // dropdown = true;
          if (e.key === "ArrowDown") {
            if (dropdown)
              document.querySelector(".first .menu-item").firstChild.focus();
            else if (qucik_res[currentIndex + 1]) {
              currentWord = qucik_res[++currentIndex];
              show();
            }
            e.stopPropagation();
            e.preventDefault();
          }
          if (e.key === "ArrowUp") {
            if (dropdown)
              document.querySelector(".first .menu-item").firstChild.focus();
            else if (qucik_res[currentIndex - 1]) {
              currentWord = qucik_res[--currentIndex];
              show();
            }
            e.stopPropagation();
            e.preventDefault();
          }
          if (e.key === "Enter") {
            if (dropdown)
              document.querySelector(".first .menu-item").firstChild.click();
            console.log(def_list);
            e.stopPropagation();
            e.preventDefault();
          }
          if (e.key === "ArrowRight") {
            navNextDef();
            e.stopPropagation();
            e.preventDefault();
          }
          if (e.key === "ArrowLeft") {
            navNextDef(true);
            e.stopPropagation();
            e.preventDefault();
          }
        }}
        on:focus={() => {
          // dropdown = true;
          //
        }}
        on:click={(e) => {
          e.stopPropagation();
          e.preventDefault();
        }}
      />
      <button
        class="btn btn-secondary input-group-btn"
        on:click={(e) => invoke("import")}>import</button
      >
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
      <div
        class="card {def.in ? 'glow' : ''}"
        bind:this={def.card}
        use:inview={{}}
        on:change={(event) => {
          const { inView, entry, scrollDirection, observer, node } =
            event.detail;
          def.in = inView;
          console.log(def);
        }}
      >
        <!-- <div class="card-image">
        <img src="public/svelte.svg">
      </div> -->
        <div class="card-header">
          <div class="card-title h5">
            {def?.word}
          </div>
          <div class="card-subtitle text-gray">
            {(def?.pronunciation?.join && def?.pronunciation.join(" / ")) || ""}
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
