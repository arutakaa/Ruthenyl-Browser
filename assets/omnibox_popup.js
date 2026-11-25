(() => {
  const post = (msg) => {
    if (window.ipc && typeof window.ipc.postMessage === "function") {
      window.ipc.postMessage(msg);
    }
  };

  const VALID_THEMES = new Set(["arctic", "midnight", "aurora"]);

  const SUGGEST_SEARCH_SVG = `<svg class="suggestion-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="8"></circle><path d="m21 21-4.3-4.3"></path></svg>`;
  const SUGGEST_HISTORY_SVG = `<svg class="suggestion-icon suggestion-icon--history" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><polyline points="12 7 12 12 15.5 14"></polyline></svg>`;
  const SUGGEST_GLOBE_SVG = `<svg class="suggestion-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M3.6 9h16.8M3.6 15h16.8"></path><path d="M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"></path></svg>`;
  const FILL_ARROW_SVG = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M17 17 7 7"></path><path d="M7 17V7h10"></path></svg>`;

  const popupCard = document.getElementById("popup-card");
  const popupList = document.getElementById("popup-list");
  const popupFooter = document.getElementById("popup-footer");
  const clearHistoryBtn = document.getElementById("btn-clear-history");

  function setHidden(el, hidden) {
    if (!el) return;
    if (hidden) {
      el.setAttribute("hidden", "");
    } else {
      el.removeAttribute("hidden");
    }
  }

  function applyTheme(theme) {
    const valid = VALID_THEMES.has(theme) ? theme : "arctic";
    document.documentElement.dataset.theme = valid;
  }

  function updateSelection(selectedIdx) {
    const rows = popupList.querySelectorAll(".suggestion-row");
    const buttons = popupList.querySelectorAll(".suggestion-main-btn");
    rows.forEach((row, idx) => {
      const selected = idx === selectedIdx;
      row.classList.toggle("is-selected", selected);
      if (buttons[idx]) {
        buttons[idx].setAttribute("aria-selected", String(selected));
      }
      if (selected) {
        row.scrollIntoView({ block: "nearest" });
      }
    });
  }

  function renderPayload(payload) {
    if (!payload || typeof payload !== "object") return;
    if (typeof payload.theme === "string") {
      applyTheme(payload.theme);
    }

    const query = typeof payload.query === "string" ? payload.query : "";
    const trimmedQuery = query.trim();
    const lowerQuery = trimmedQuery.toLowerCase();
    const items = Array.isArray(payload.items) ? payload.items : [];
    const selectedIdx = Number.isInteger(payload.selectedIndex) ? payload.selectedIndex : -1;

    popupList.replaceChildren();

    items.forEach((item, idx) => {
      if (!item || typeof item.text !== "string" || !item.text) return;

      const li = document.createElement("li");
      li.className = "suggestion-row";
      li.setAttribute("role", "presentation");

      const mainBtn = document.createElement("button");
      mainBtn.type = "button";
      mainBtn.id = `omnibox-opt-${idx}`;
      mainBtn.className = "suggestion-main-btn";
      mainBtn.setAttribute("role", "option");
      mainBtn.setAttribute("aria-selected", "false");

      const iconWrap = document.createElement("span");
      iconWrap.setAttribute("aria-hidden", "true");
      if (item.kind === "url") {
        iconWrap.innerHTML = SUGGEST_GLOBE_SVG;
      } else if (item.kind === "history") {
        iconWrap.innerHTML = SUGGEST_HISTORY_SVG;
      } else {
        iconWrap.innerHTML = SUGGEST_SEARCH_SVG;
      }

      const textWrap = document.createElement("span");
      textWrap.className = "suggestion-text";

      const lowerItem = item.text.toLowerCase();
      if (lowerQuery && lowerItem.startsWith(lowerQuery)) {
        const prefixLen = trimmedQuery.length;
        const matchSpan = document.createElement("span");
        matchSpan.className = "suggestion-match";
        matchSpan.textContent = item.text.slice(0, prefixLen);

        const restSpan = document.createElement("span");
        restSpan.className = "suggestion-rest";
        restSpan.textContent = item.text.slice(prefixLen);

        textWrap.append(matchSpan, restSpan);
      } else {
        const wholeSpan = document.createElement("span");
        wholeSpan.className = item.kind === "history" ? "suggestion-match" : "suggestion-rest";
        wholeSpan.textContent = item.text;
        textWrap.appendChild(wholeSpan);
      }

      mainBtn.append(iconWrap, textWrap);

      if (item.kind === "url") {
        const badge = document.createElement("span");
        badge.className = "suggestion-badge";
        badge.textContent = "Buka situs";
        mainBtn.appendChild(badge);
      }

      mainBtn.addEventListener("mousedown", (e) => {
        e.preventDefault();
      });
      mainBtn.addEventListener("click", () => {
        post(`popup:go:${item.text}`);
      });

      li.appendChild(mainBtn);

      if (item.kind === "history") {
        const delBtn = document.createElement("button");
        delBtn.type = "button";
        delBtn.className = "suggestion-action-btn";
        delBtn.setAttribute("aria-label", `Hapus riwayat ${item.text}`);
        delBtn.textContent = "Hapus";
        delBtn.addEventListener("mousedown", (e) => {
          e.preventDefault();
          e.stopPropagation();
        });
        delBtn.addEventListener("click", (e) => {
          e.stopPropagation();
          post(`popup:del_history:${item.text}`);
        });
        li.appendChild(delBtn);
      } else if (item.kind === "suggest") {
        const fillBtn = document.createElement("button");
        fillBtn.type = "button";
        fillBtn.className = "suggestion-action-btn";
        fillBtn.setAttribute("aria-label", `Isi ke kolom pencarian: ${item.text}`);
        fillBtn.title = "Tambahkan ke kolom pencarian";
        fillBtn.innerHTML = FILL_ARROW_SVG;
        fillBtn.addEventListener("mousedown", (e) => {
          e.preventDefault();
          e.stopPropagation();
        });
        fillBtn.addEventListener("click", (e) => {
          e.stopPropagation();
          post(`popup:fill:${item.text}`);
        });
        li.appendChild(fillBtn);
      }

      popupList.appendChild(li);
    });

    setHidden(popupFooter, !payload.showHistoryFooter);
    updateSelection(selectedIdx);
  }

  clearHistoryBtn.addEventListener("mousedown", (e) => {
    e.preventDefault();
  });

  clearHistoryBtn.addEventListener("click", () => {
    post("popup:clear_history");
  });

  document.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    if (popupCard && !popupCard.contains(e.target)) {
      post("popup:dismiss");
    }
  });

  document.addEventListener("contextmenu", (e) => {
    e.preventDefault();
  });

  window.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      post("popup:dismiss");
    }
  });

  window.__omniboxSetTheme = applyTheme;
  window.__omniboxSelect = updateSelection;
  window.__omniboxRenderJson = (jsonStr) => {
    try {
      const parsed = JSON.parse(jsonStr);
      renderPayload(parsed);
    } catch (_) {
      // Ignore malformed payloads.
    }
  };
})();
