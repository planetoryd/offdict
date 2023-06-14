// ==UserScript==
// @name        Offdict integration
// @namespace   Violentmonkey Scripts
// @match       *://*/*
// @grant       GM_xmlhttpRequest
// @version     1.0
// @author      -
// @description 4/2/2023, 2:36:47 AM
// ==/UserScript==

(function () {
  "use strict";
  let offdictIP = "172.20.18.1"
  console.log("offdict ==");
  document.addEventListener("mouseup", function () {
    let selectedText = window.getSelection().toString();
    if (selectedText) {
      console.log("text: " + selectedText);
      GM_xmlhttpRequest({
        method: "GET",
        url: `http://${offdictIP}:3030/set/` + encodeURIComponent(selectedText),
        headers: {
          "Content-Type": "text/plain",
        },
        onload: function (response) {
          console.log(response.responseText);
        },
        onerror: console.log,
      });
    }
  });
})();
