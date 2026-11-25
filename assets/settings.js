(() => {
  const THEME_KEY = "ruthenium_ntp_theme_v2";
  const VALID_THEMES = new Set(["arctic", "midnight", "aurora"]);

  const filterInput = document.getElementById("settings-filter-input");
  const searchEmpty = document.getElementById("settings-search-empty");
  const toastEl = document.getElementById("settings-toast");
  const railButtons = Array.from(document.querySelectorAll("[data-nav-section]"));
  const sections = Array.from(document.querySelectorAll(".settings-section"));
  const allRows = Array.from(document.querySelectorAll(".setting-row"));

  const startupButtons = Array.from(document.querySelectorAll("[data-startup]"));
  const startupUrlForm = document.getElementById("startup-url-form");
  const startupUrlInput = document.getElementById("startup-url-input");
  const defaultBrowserHint = document.getElementById("default-browser-hint");
  const btnDefaultBrowser = document.getElementById("btn-default-browser");
  const btnOpenWelcome = document.getElementById("btn-open-welcome");

  const themeCards = Array.from(document.querySelectorAll("[data-theme-choice]"));
  const zoomButtons = Array.from(document.querySelectorAll("[data-zoom]"));
  const switchHomeClock = document.getElementById("switch-home-clock");
  const switchHomeShortcuts = document.getElementById("switch-home-shortcuts");

  const engineButtons = Array.from(document.querySelectorAll("[data-engine]"));
  const switchSuggest = document.getElementById("switch-suggest");
  const btnClearSearchHistory = document.getElementById("btn-clear-search-history");

  const switchAdblock = document.getElementById("switch-adblock");
  const adblockStatusHint = document.getElementById("adblock-status-hint");
  const btnRefreshFilters = document.getElementById("btn-refresh-filters");
  const whitelistForm = document.getElementById("whitelist-form");
  const whitelistInput = document.getElementById("whitelist-input");
  const whitelistList = document.getElementById("whitelist-list");
  const whitelistEmpty = document.getElementById("whitelist-empty");

  const chkClearCache = document.getElementById("chk-clear-cache");
  const chkClearCookies = document.getElementById("chk-clear-cookies");
  const chkClearHistory = document.getElementById("chk-clear-history");
  const btnClearBrowsingData = document.getElementById("btn-clear-browsing-data");
  const btnCancelClearData = document.getElementById("btn-cancel-clear-data");

  const downloadFolderPath = document.getElementById("download-folder-path");
  const btnPickDownloads = document.getElementById("btn-pick-downloads");
  const btnOpenDownloads = document.getElementById("btn-open-downloads");

  const aboutVersionDesc = document.getElementById("about-version-desc");
  const aboutWebview2Badge = document.getElementById("about-webview2-badge");
  const btnResetSettings = document.getElementById("btn-reset-settings");
  const btnCancelReset = document.getElementById("btn-cancel-reset");

  let toastTimer = null;
  let clearDataArmed = false;
  let resetArmed = false;

  function setHidden(el, hidden) {
    if (!el) return;
    if (hidden) {
      el.setAttribute("hidden", "");
    } else {
      el.removeAttribute("hidden");
    }
  }

  function showToast(msg) {
    if (!toastEl || !msg) return;
    toastEl.textContent = msg;
    setHidden(toastEl, false);
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      setHidden(toastEl, true);
    }, 2400);
  }

  function sendCommand(cmd) {
    if (typeof window.__settingsCommand === "function") {
      window.__settingsCommand(cmd);
    }
  }

  function applyThemeLocal(raw) {
    const mode = VALID_THEMES.has(raw) ? raw : "arctic";
    document.documentElement.dataset.theme = mode;
    try {
      localStorage.setItem(THEME_KEY, mode);
    } catch (_) {}
    for (const card of themeCards) {
      const selected = card.dataset.themeChoice === mode;
      card.classList.toggle("is-selected", selected);
      card.setAttribute("aria-pressed", String(selected));
    }
  }

  function renderWhitelist(items) {
    if (!whitelistList || !whitelistEmpty) return;
    whitelistList.replaceChildren();
    const list = Array.isArray(items) ? items : [];
    setHidden(whitelistEmpty, list.length > 0);
    setHidden(whitelistList, list.length === 0);

    for (const domain of list) {
      const row = document.createElement("div");
      row.className = "whitelist-item";

      const label = document.createElement("span");
      label.className = "whitelist-domain";
      label.textContent = domain;

      const delBtn = document.createElement("button");
      delBtn.type = "button";
      delBtn.className = "btn-ghost";
      delBtn.textContent = "Hapus";
      delBtn.addEventListener("click", () => {
        sendCommand(`whitelist:remove:${domain}`);
      });

      row.append(label, delBtn);
      whitelistList.appendChild(row);
    }
  }

  function syncSettingsState(state) {
    if (!state || typeof state !== "object") return;

    applyThemeLocal(state.theme);

    const mode = state.startupMode || "restore";
    for (const btn of startupButtons) {
      const active = btn.dataset.startup === mode;
      btn.classList.toggle("is-selected", active);
      btn.setAttribute("aria-pressed", String(active));
    }
    setHidden(startupUrlForm, mode !== "custom");
    if (startupUrlInput && document.activeElement !== startupUrlInput) {
      startupUrlInput.value = state.startupUrl || "";
    }

    const isDefault = Boolean(state.isDefaultBrowser);
    if (defaultBrowserHint) {
      defaultBrowserHint.textContent = isDefault
        ? "Aktif · Ruthenium adalah browser default"
        : "Belum menjadi browser default";
    }
    if (btnDefaultBrowser) {
      btnSetDefaultLabel(btnDefaultBrowser, isDefault);
    }

    const zoomVal = String(state.defaultZoom || 100);
    for (const btn of zoomButtons) {
      const active = btn.dataset.zoom === zoomVal;
      btn.classList.toggle("is-selected", active);
      btn.setAttribute("aria-pressed", String(active));
    }

    if (switchHomeClock) {
      switchHomeClock.setAttribute("aria-checked", String(Boolean(state.showHomeClock)));
    }
    if (switchHomeShortcuts) {
      switchHomeShortcuts.setAttribute("aria-checked", String(Boolean(state.showHomeShortcuts)));
    }

    const engine = state.searchEngine || "google";
    for (const btn of engineButtons) {
      const active = btn.dataset.engine === engine;
      btn.classList.toggle("is-selected", active);
      btn.setAttribute("aria-pressed", String(active));
    }
    if (switchSuggest) {
      switchSuggest.setAttribute("aria-checked", String(Boolean(state.suggestEnabled)));
    }

    const adbEnabled = Boolean(state.adblockEnabled);
    const totalBlocked = Number(state.adblockBlockedTotal) || 0;
    const refreshing = Boolean(state.adblockRefreshing);
    const statusText = state.adblockStatus || "Aktif (Agresif)";
    if (switchAdblock) {
      switchAdblock.setAttribute("aria-checked", String(adbEnabled));
    }
    if (adblockStatusHint) {
      if (!adbEnabled) {
        adblockStatusHint.textContent = "Nonaktif · Perlindungan iklan dijeda";
      } else if (totalBlocked > 0) {
        adblockStatusHint.textContent = `${statusText} · ${totalBlocked} diblokir sesi ini`;
      } else {
        adblockStatusHint.textContent = statusText;
      }
    }
    if (btnRefreshFilters) {
      btnRefreshFilters.disabled = refreshing;
      btnRefreshFilters.textContent = refreshing ? "Memperbarui filter..." : "Perbarui 10 filter";
    }

    renderWhitelist(state.adblockWhitelist);

    if (downloadFolderPath) {
      downloadFolderPath.textContent = state.downloadDir || "";
    }

    const wvVer = state.webview2Version || "WebView2";
    if (aboutVersionDesc) {
      aboutVersionDesc.textContent = `Ruthenium 0.1.0 · WebView2 ${wvVer} · adblock-rust`;
    }
    if (aboutWebview2Badge) {
      aboutWebview2Badge.textContent = wvVer;
    }
  }

  function btnSetDefaultLabel(btn, isDefault) {
    btn.textContent = isDefault ? "Sudah menjadi default" : "Jadikan browser default";
  }

  function selectSection(sectionId, scrollIntoView = true) {
    const valid = sections.some((s) => s.dataset.section === sectionId) ? sectionId : "general";
    for (const btn of railButtons) {
      const active = btn.dataset.navSection === valid;
      btn.classList.toggle("is-active", active);
      if (active) {
        btn.setAttribute("aria-current", "page");
      } else {
        btn.removeAttribute("aria-current");
      }
    }
    if (scrollIntoView) {
      const target = sections.find((s) => s.dataset.section === valid);
      if (target) {
        target.scrollIntoView({ behavior: "smooth", block: "start" });
      }
    }
  }

  function applySettingsFilter(query) {
    const q = (query || "").trim().toLowerCase();
    let visibleCount = 0;

    for (const sec of sections) {
      const rows = Array.from(sec.querySelectorAll(".setting-row"));
      let sectionMatches = 0;
      for (const row of rows) {
        const hay = `${row.textContent || ""} ${row.dataset.keywords || ""}`.toLowerCase();
        const match = !q || hay.includes(q);
        setHidden(row, !match);
        if (match) {
          sectionMatches += 1;
          visibleCount += 1;
        }
      }
      setHidden(sec, sectionMatches === 0);
    }

    setHidden(searchEmpty, visibleCount > 0 || !q);
  }

  function disarmClearData() {
    clearDataArmed = false;
    if (btnClearBrowsingData) {
      btnClearBrowsingData.classList.remove("is-armed");
      btnClearBrowsingData.textContent = "Hapus data terpilih";
    }
    setHidden(btnCancelClearData, true);
  }

  function disarmReset() {
    resetArmed = false;
    if (btnResetSettings) {
      btnResetSettings.classList.remove("is-armed");
      btnResetSettings.textContent = "Reset ke bawaan";
    }
    setHidden(btnCancelReset, true);
  }

  window.__syncSettings = (state) => {
    syncSettingsState(state);
  };
  window.__syncTheme = (theme) => {
    applyThemeLocal(theme);
  };
  window.__settingsFeedback = (msg) => {
    showToast(msg);
  };
  window.__selectSettingsSection = (section) => {
    if (filterInput && filterInput.value) {
      filterInput.value = "";
      applySettingsFilter("");
    }
    selectSection(section, true);
  };

  for (const btn of railButtons) {
    btn.addEventListener("click", () => {
      if (filterInput && filterInput.value) {
        filterInput.value = "";
        applySettingsFilter("");
      }
      selectSection(btn.dataset.navSection || "general", true);
    });
  }

  if (filterInput) {
    filterInput.addEventListener("input", () => applySettingsFilter(filterInput.value));
  }

  document.addEventListener("keydown", (e) => {
    if (e.ctrlKey && !e.altKey && e.key.toLowerCase() === "f" && filterInput) {
      e.preventDefault();
      filterInput.focus();
      filterInput.select();
    }
  });

  for (const btn of startupButtons) {
    btn.addEventListener("click", () => {
      const mode = btn.dataset.startup || "restore";
      sendCommand(`startup:${mode}`);
      showToast("Pengaturan startup diperbarui");
    });
  }

  if (startupUrlForm && startupUrlInput) {
    startupUrlForm.addEventListener("submit", (e) => {
      e.preventDefault();
      sendCommand(`startup_url:${startupUrlInput.value.trim()}`);
      showToast("URL startup disimpan");
    });
  }

  if (btnDefaultBrowser) {
    btnDefaultBrowser.addEventListener("click", () => sendCommand("default_browser"));
  }
  if (btnOpenWelcome) {
    btnOpenWelcome.addEventListener("click", () => sendCommand("open_welcome"));
  }

  for (const card of themeCards) {
    card.addEventListener("click", () => {
      const mode = card.dataset.themeChoice || "arctic";
      applyThemeLocal(mode);
      if (typeof window.__newtabSetTheme === "function") {
        window.__newtabSetTheme(mode);
      }
      showToast("Tema tampilan diubah");
    });
  }

  for (const btn of zoomButtons) {
    btn.addEventListener("click", () => {
      const z = btn.dataset.zoom || "100";
      sendCommand(`zoom:${z}`);
      showToast(`Zoom default diatur ke ${z}%`);
    });
  }

  if (switchHomeClock) {
    switchHomeClock.addEventListener("click", () => {
      const next = switchHomeClock.getAttribute("aria-checked") !== "true";
      sendCommand(next ? "home_clock:on" : "home_clock:off");
    });
  }
  if (switchHomeShortcuts) {
    switchHomeShortcuts.addEventListener("click", () => {
      const next = switchHomeShortcuts.getAttribute("aria-checked") !== "true";
      sendCommand(next ? "home_shortcuts:on" : "home_shortcuts:off");
    });
  }

  for (const btn of engineButtons) {
    btn.addEventListener("click", () => {
      const engine = btn.dataset.engine || "google";
      if (typeof window.__newtabSetSearchEngine === "function") {
        window.__newtabSetSearchEngine(engine);
      }
      showToast("Mesin pencari default diperbarui");
    });
  }

  if (switchSuggest) {
    switchSuggest.addEventListener("click", () => {
      const next = switchSuggest.getAttribute("aria-checked") !== "true";
      sendCommand(next ? "suggest:on" : "suggest:off");
    });
  }
  if (btnClearSearchHistory) {
    btnClearSearchHistory.addEventListener("click", () => {
      if (typeof window.__newtabClearSearchHistory === "function") {
        window.__newtabClearSearchHistory();
      }
      showToast("Riwayat pencarian dihapus");
    });
  }

  if (switchAdblock) {
    switchAdblock.addEventListener("click", () => {
      const next = switchAdblock.getAttribute("aria-checked") !== "true";
      sendCommand(next ? "adblock:on" : "adblock:off");
    });
  }
  if (btnRefreshFilters) {
    btnRefreshFilters.addEventListener("click", () => {
      sendCommand("adblock:refresh");
      showToast("Memperbarui 10 daftar filter di latar belakang...");
    });
  }

  if (whitelistForm && whitelistInput) {
    whitelistForm.addEventListener("submit", (e) => {
      e.preventDefault();
      const val = whitelistInput.value.trim();
      if (!val) return;
      sendCommand(`whitelist:add:${val}`);
      whitelistInput.value = "";
    });
  }

  if (btnClearBrowsingData) {
    btnClearBrowsingData.addEventListener("click", () => {
      const flags = [];
      if (chkClearCache && chkClearCache.checked) flags.push("cache");
      if (chkClearCookies && chkClearCookies.checked) flags.push("cookies");
      if (chkClearHistory && chkClearHistory.checked) flags.push("history");
      if (flags.length === 0) {
        showToast("Pilih minimal satu jenis data untuk dihapus");
        return;
      }
      if (!clearDataArmed) {
        clearDataArmed = true;
        btnClearBrowsingData.classList.add("is-armed");
        btnClearBrowsingData.textContent = "Konfirmasi hapus sekarang";
        setHidden(btnCancelClearData, false);
        return;
      }
      disarmClearData();
      sendCommand(`clear_data:${flags.join(",")}`);
    });
  }
  if (btnCancelClearData) {
    btnCancelClearData.addEventListener("click", disarmClearData);
  }

  if (btnPickDownloads) {
    btnPickDownloads.addEventListener("click", () => sendCommand("downloads:pick"));
  }
  if (btnOpenDownloads) {
    btnOpenDownloads.addEventListener("click", () => sendCommand("downloads:open"));
  }

  if (btnResetSettings) {
    btnResetSettings.addEventListener("click", () => {
      if (!resetArmed) {
        resetArmed = true;
        btnResetSettings.classList.add("is-armed");
        btnResetSettings.textContent = "Konfirmasi reset bawaan";
        setHidden(btnCancelReset, false);
        return;
      }
      disarmReset();
      sendCommand("reset");
    });
  }
  if (btnCancelReset) {
    btnCancelReset.addEventListener("click", disarmReset);
  }

  if (location.hash && location.hash.length > 1) {
    selectSection(location.hash.slice(1), true);
  }
  applySettingsFilter("");
  for (const row of allRows) {
    row.dataset.ready = "true";
  }
})();
