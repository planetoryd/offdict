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
  // import DefUnit from "./lib/DefUnit.svelte";
  import { inview } from "svelte-inview";
  // import DefsOfType from "./lib/DefsOfType.svelte";

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
  let inputheader = false;
  let inputWord: string;
  let extensive;
  let def_list = []; // Final list of defs
  let show_not_found = true; // clip input was supposed to show explicit error
  let notfound = false;
  let welcome = true;


  listen("error", (e) => {});
  listen("clip", (e) => {
    inputWord = e.payload as string;
    show_not_found = true;
    welcome = false;
    onInput();
  });
  listen("set_input", (e: any) => {
    welcome = false;
    window.scrollTo({
      top: 0,
      behavior: "auto",
    });
    inputWord = e.payload.inputWord;
    extensive = e.payload.extensive;
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
        welcome = false;
        def_list = r as [];
      })
      .catch((e) => {
        console.log(e);
      });
  }

  document.addEventListener("keydown", (e) => {
    if (e.key === "ArrowRight") {
      scroll();
      e.stopPropagation();
      e.preventDefault();
    }
    if (e.key === "ArrowLeft") {
      scroll(true);
      e.stopPropagation();
      e.preventDefault();
    }
    document.querySelector("input.form-input")?.focus();
  });

  listen("def_list", (e) => {
    if (inputWord.trim() === "") {
      def_list = [];
      welcome = true;
      notfound = false;
    } else {
      def_list = e.payload as [any];
      console.log(def_list);
      if (def_list.length === 0) {
        notfound = true;
      } else {
        notfound = false;
      }
    }
  });

  async function onInput() {
    await invoke("input", {
      query: inputWord,
      expensive: false,
    });
  }

  function scroll(goPrev = false) {
    if (!goPrev)
      window.scrollTo({
        top: 200 + window.scrollY,
        behavior: "smooth",
      });
    else
      window.scrollTo({
        top: window.scrollY - 200,
        behavior: "smooth",
      });
  }
</script>

<main>
  {#if inputheader}
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
            // if (e.key === "ArrowRight") {
            //   navNextDef();
            //   e.stopPropagation();
            //   e.preventDefault();
            // }
            // if (e.key === "ArrowLeft") {
            //   navNextDef(true);
            //   e.stopPropagation();
            //   e.preventDefault();
            // }
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
  {/if}

  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div
    class="container"
    on:click={() => {
      dropdown = false;
    }}
  >
    {#if welcome}
      <div class="card welcome">type anything to start</div>
    {/if}
    {#if notfound}
      <div class="card notfound">
        Not found, <div class="chip">{inputWord}</div>
        <br />
        {#if !extensive}
          Try hitting
          <div class="chip">Enter</div>
          to initiate an extensive search
        {/if}
      </div>
    {/if}

    {#each def_list as def}
      <div
        class="card {def.in ? 'glow' : ''} {inputheader
          ? 'enable-inputheader'
          : ''}"
        bind:this={def.card}
        use:inview={{}}
        on:change={(event) => {
          const { inView, entry, scrollDirection, observer, node } =
            event.detail;
          def.in = inView;
          // console.log(def);
        }}
      >
        <!-- <div class="card-image">
        <img src="public/svelte.svg">
      </div> -->
        <div class="card-header" on:click={() => (def.showFooter = true)}>
          <div class="card-title h5">
            {def?.word}
          </div>
          <div class="card-subtitle text-gray">
            {(def?.pronunciation?.join && def?.pronunciation.join(" / ")) || ""}
          </div>
        </div>
        <div class="card-body">
          {#if def}
            <Def {def} root={true} />
          {/if}
          <!-- <pre class="code">
            <code>{JSON.stringify(def, null, 2)}</code>
          </pre> -->
        </div>
        {#if def.showFooter}
          <div class="card-footer">{def?.dictName}</div>
        {/if}
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
  div.container {
    padding-right: 25px;
    padding-bottom: 50px;
  }
  .card {
    margin: 0px;
    margin-left: 8px;
    border: none;
    padding-top: 0;
    .card-body {
      padding: 5px;
      padding-left: 15px;
      padding-right: 15px;
      padding-bottom: 2px;
    }
    &:first-child {
      margin-top: 20px;
      .card-header {
        margin-top: 10px;
      }
    }
    &:first-child.enable-inputheader {
      margin-top: 75px;
    }
  }
  .menu {
    margin-top: 5px;
    margin-bottom: 25px;
  }

  :global(.svelte-tabs li.svelte-tabs__tab:focus) {
    outline: none;
  }
  :global(li.svelte-tabs__selected) {
    border-bottom: 0 solid rgba(128, 128, 128, 0.514) !important;
    background: rgba(9, 79, 158, 0.425) !important;
    color: rgba(255, 255, 255, 1) !important;
    margin: 2px;
    margin-right: 8px;
    margin-left: 0;
    padding-top: 2px !important;
    padding-bottom: 2px !important;
    &:hover {
      color: rgba(255, 253, 253, 0.76) !important;
    }
  }
  :global(.svelte-tabs__tab) {
    background: rgba(0, 101, 160, 0.062);
    color: rgba(0, 0, 0, 0.925) !important;
    margin: 2px;
    margin-right: 8px;
    margin-left: 0;
    padding-top: 2px !important;
    padding-bottom: 2px !important;
    border-radius: 2px;
    border-bottom: 0;
  }
  :global(.svelte-tabs__tab:hover) {
    color: rgba(40, 78, 148, 0.801) !important;
  }
  :global(.svelte-tabs__tab-list) {
    border-bottom: 0 !important;
  }

  .card-footer {
    color: lightgrey;
    padding-top: 0;
    padding-bottom: 0;
  }
  .card-header {
    padding-top: 0;
    &:hover {
      text-decoration-line: underline;
      text-decoration-style: wavy;
      text-decoration-color: rgba(105, 128, 0, 0.466);
      text-underline-offset: 4px;
      text-decoration-thickness: 2px;
    }
  }
  .notfound {
    padding: 15px;
    margin: 10px;
    background: rgba(128, 128, 128, 0.075);
  }
  .welcome {
    padding: 15px;
    margin: 10px;
    background: rgba(0, 119, 255, 0.158);
  }

  :global(body) {
    $bg: rgb(21, 63, 68);
    $text: rgba(255, 255, 255, 0.692);
    background: $bg;
    color: $text;
    .card.welcome {
      background: rgba(255, 255, 255, 0.15);
      color: rgba(255, 255, 255, 0.829);
    }
    .card {
      background-color: rgba(255, 255, 255, 0);
    }
    .card-header {
      color: rgba(255, 255, 255, 0.9);
      &:hover {
        text-decoration-line: underline;
        text-decoration-style: wavy;
        text-decoration-color: rgba(233, 181, 11, 0.8);
        text-underline-offset: 4px;
        text-decoration-thickness: 2px;
      }
    }
    .card-footer {
      color: rgba(212, 212, 212, 0.37);
    }
    :global(.svelte-tabs li.svelte-tabs__tab:focus) {
      outline: none;
    }
    :global(li.svelte-tabs__selected) {
      background: rgba(158, 123, 9, 0.425) !important;
      color: rgba(255, 255, 255, 1) !important;
      &:hover {
        color: rgba(255, 253, 253, 0.76) !important;
      }
    }
    :global(.svelte-tabs__tab) {
      background: rgba(158, 123, 9, 0.425);
      color: rgba(190, 190, 190, 0.925) !important;
      border: 0;
    }
    :global(.svelte-tabs__tab:hover) {
      color: rgba(182, 182, 182, 0.582) !important;
    }
  }
  div.notfound {
    display: block;
  }
  .chip {
    background: rgba(170, 115, 64, 0.5);
    color: rgba(255, 255, 255, 0.9);
  }
</style>
