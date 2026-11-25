(() => {
  const post = (msg) => {
    if (window.ipc && typeof window.ipc.postMessage === "function") {
      window.ipc.postMessage(msg);
    }
  };

  const RUTHENIUM_FAVICON_SVG = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"><polygon points="32,4 56,18 56,46 32,60 8,46 8,18" fill="#0f2347"/><polygon points="32,4 56,18 32,32 8,18" fill="#1e3f78"/><polygon points="32,32 56,18 56,46 32,60" fill="#09152b"/><path d="M23 20H35C39.5 20 42.5 22.5 42.5 26.5C42.5 29.8 40.3 32.1 37 32.8L43 43H36.8L31.5 33.4H28.2V43H23V20ZM28.2 24.4V29.2H34.2C36.3 29.2 37.5 28.2 37.5 26.8C37.5 25.3 36.3 24.4 34.2 24.4H28.2Z" fill="#7dd3fc"/></svg>`;
  const SPINNER_SVG = `<path d="M7 1.5a5.5 5.5 0 1 1-4.6 2.5"></path>`;
  const CLOSE_SVG = `<svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><line x1="3" y1="3" x2="9" y2="9"></line><line x1="9" y1="3" x2="3" y2="9"></line></svg>`;
  const PREFS_STORAGE_KEY = "ruthenium_chrome_prefs_v3";
  const LEGACY_V2_PREFS_KEY = "ruthenium_chrome_prefs_v2";
  const LEGACY_V1_PREFS_KEY = "ruthenium_chrome_prefs_v1";
  const SHELL_NOTES_KEY = "ruthenium_shell_notes_v1";
  const SHELL_HISTORY_KEY = "ruthenium_shell_search_history_v1";
  const MAX_SHELL_HISTORY = 10;
  const VALID_THEMES = new Set(["arctic", "midnight", "aurora"]);
  const THEME_LABELS = {
    arctic: "Tema Arctic aktif",
    midnight: "Tema Midnight aktif",
    aurora: "Tema Aurora aktif",
  };
  const ENGINE_LABELS = {
    google: "Google",
    duckduckgo: "DuckDuckGo",
    bing: "Bing",
  };

  function normalizeTheme(raw) {
    if (typeof raw !== "string") return "arctic";
    const val = raw.trim().toLowerCase();
    if (val === "light" || val === "arctic") return "arctic";
    if (val === "dark" || val === "midnight") return "midnight";
    if (val === "aurora") return "aurora";
    return "arctic";
  }

  function engineDisplayName(key) {
    return ENGINE_LABELS[key] || "Google";
  }

  function searchPlaceholderFor(engineKey) {
    return `Cari di ${engineDisplayName(engineKey)} atau masukkan URL`;
  }

  function loadChromePrefs() {
    try {
      const rawV3 = localStorage.getItem(PREFS_STORAGE_KEY);
      if (rawV3) {
        const parsed = JSON.parse(rawV3);
        if (parsed && typeof parsed === "object") {
          const searchEngine = typeof parsed.searchEngine === "string" && ENGINE_LABELS[parsed.searchEngine]
            ? parsed.searchEngine
            : "google";
          return { searchEngine, theme: normalizeTheme(parsed.theme) };
        }
      }
      const rawLegacy = localStorage.getItem(LEGACY_V2_PREFS_KEY)
        || localStorage.getItem(LEGACY_V1_PREFS_KEY);
      if (rawLegacy) {
        const parsed = JSON.parse(rawLegacy);
        if (parsed && typeof parsed === "object") {
          const searchEngine = typeof parsed.searchEngine === "string" && ENGINE_LABELS[parsed.searchEngine]
            ? parsed.searchEngine
            : "google";
          return { searchEngine, theme: "arctic" };
        }
      }
      return null;
    } catch (_) {
      return null;
    }
  }

  function saveChromePrefs() {
    try {
      localStorage.setItem(
        PREFS_STORAGE_KEY,
        JSON.stringify({
          searchEngine: ENGINE_LABELS[currentState.searchEngine] ? currentState.searchEngine : "google",
          theme: VALID_THEMES.has(currentState.theme) ? currentState.theme : "arctic",
        }),
      );
    } catch (_) {
      // Ignore storage quota errors so controls stay responsive.
    }
  }

  const shellRoot = document.getElementById("shell-root");
  const sidebar = document.getElementById("sidebar");
  const sidebarDefaultView = document.getElementById("sidebar-default-view");
  const sidebarResizer = document.getElementById("sidebar-resizer");
  const sidebarTabList = document.getElementById("sidebar-tab-list");
  const sidebarNewTabBtn = document.getElementById("sidebar-new-tab-btn");
  const sidebarCloseBtn = document.getElementById("sidebar-close-btn");
  const switcherBtns = Array.from(document.querySelectorAll("[data-panel-tab]"));

  const settingsPanel = document.getElementById("panel-settings");
  const themeButtons = Array.from(document.querySelectorAll("[data-shell-theme]"));
  const engineSelect = document.getElementById("engine-select");
  const settingsStatus = document.getElementById("settings-status");
  const shellQuickNotes = document.getElementById("shell-quick-notes");

  const tabList = document.getElementById("tab-list");
  const btnSidebar = document.getElementById("btn-sidebar");
  const btnSettings = document.getElementById("btn-settings");
  const btnBack = document.getElementById("btn-back");
  const btnForward = document.getElementById("btn-forward");
  const btnReload = document.getElementById("btn-reload");
  const iconReload = document.getElementById("icon-reload");
  const iconStop = document.getElementById("icon-stop");

  const btnShare = document.getElementById("btn-share");
  const iconShare = document.getElementById("icon-share");
  const iconShareCheck = document.getElementById("icon-share-check");
  const btnNewTab = document.getElementById("btn-new-tab");

  const addressForm = document.getElementById("address-form");
  const addressInput = document.getElementById("address-input");
  const addressDatalist = document.getElementById("omnibox-suggestions");
  const originDomain = document.getElementById("origin-domain");
  const originPath = document.getElementById("origin-path");
  const securityBadge = document.getElementById("security-badge");
  const securityChip = document.getElementById("security-chip");
  const iconSearch = document.getElementById("icon-search");
  const iconLock = document.getElementById("icon-lock");
  const iconWarn = document.getElementById("icon-warn");

  const loadingSeam = document.getElementById("loading-seam");
  const btnMinimize = document.getElementById("btn-minimize");
  const btnMaximize = document.getElementById("btn-maximize");
  const iconMaximize = document.getElementById("icon-maximize");
  const iconRestore = document.getElementById("icon-restore");
  const btnClose = document.getElementById("btn-close");

  function setHidden(el, hidden) {
    if (!el) return;
    if (hidden) {
      el.setAttribute("hidden", "");
    } else {
      el.removeAttribute("hidden");
    }
  }

  let currentState = {
    tabs: [],
    rawUrl: "",
    origin: "",
    path: "",
    title: "Tab Baru",
    security: "internal",
    canGoBack: false,
    canGoForward: false,
    isLoading: false,
    sidebarWidth: 0,
    isCompact: true,
    isMaximized: false,
    windowFocused: true,
    theme: "arctic",
    searchEngine: "google",
  };

  let loadingTimer = null;
  let shareCheckTimer = null;
  let settingsStatusTimer = null;
  let activeSidebarPanel = "workspace";
  const tabElements = new Map();
  const sidebarTabElements = new Map();
  const omniboxSuggestCache = new Map();
  const currentOmniboxOptions = new Set();
  let omniboxSuggestTimer = null;
  let omniboxSuggestAbort = null;
  let omniboxSuggestSeq = 0;

  function setLoadingVisual(isLoading) {
    if (isLoading) {
      clearTimeout(loadingTimer);
      loadingTimer = setTimeout(() => {
        loadingSeam.classList.add("is-active");
        loadingSeam.style.width = "82%";
      }, 120);
    } else {
      clearTimeout(loadingTimer);
      if (loadingSeam.classList.contains("is-active")) {
        loadingSeam.style.width = "100%";
        setTimeout(() => {
          loadingSeam.classList.remove("is-active");
          loadingSeam.style.width = "0%";
        }, 120);
      } else {
        loadingSeam.style.width = "0%";
      }
    }
  }

  function renderSecurity(security) {
    const isInternal = security === "internal";
    const isInsecure = security === "insecure";
    setHidden(iconSearch, !isInternal);
    setHidden(iconLock, isInternal || isInsecure);
    setHidden(iconWarn, !isInsecure);
    setHidden(securityChip, !isInsecure);
    securityBadge.classList.toggle("is-internal", isInternal);
    securityBadge.classList.toggle("is-insecure", isInsecure);
  }

  function isPublicHost(hostname) {
    const host = (hostname || "").trim().toLowerCase();
    if (
      !host
      || host === "localhost"
      || host === "browser.localhost"
      || host === "::1"
      || host === "[::1]"
      || host.startsWith("127.")
      || !host.includes(".")
      || /^\d+(\.\d+){3}$/.test(host)
    ) {
      return false;
    }
    return true;
  }

  function faviconCandidatesForTab(tab) {
    const origin = (tab && tab.origin) || "";
    if (!origin || origin.startsWith("browser://")) {
      return [];
    }
    const candidates = [];
    const pushUnique = (url) => {
      if (typeof url === "string" && url && !candidates.includes(url)) {
        candidates.push(url);
      }
    };

    pushUnique(tab.faviconUrl || "");

    const hostOnly = origin.startsWith("[")
      ? origin.slice(0, origin.indexOf("]") + 1)
      : origin.split(":")[0];
    const isLocal = hostOnly === "localhost" || hostOnly === "[::1]" || hostOnly.startsWith("127.");

    if (isPublicHost(hostOnly)) {
      pushUnique(`https://www.google.com/s2/favicons?domain=${encodeURIComponent(hostOnly)}&sz=64`);
    }
    pushUnique(`${isLocal ? "http" : "https"}://${origin}/favicon.ico`);
    return candidates;
  }

  function syncFaviconSlot(slot, tab, sizePx) {
    const origin = (tab && tab.origin) || "";
    const isBrand = !origin || origin.startsWith("browser://");
    const favKey = isBrand ? "__brand__" : `${origin}|${tab.faviconUrl || ""}`;
    if (slot.dataset.faviconKey === favKey) return;

    slot.dataset.faviconKey = favKey;
    slot.replaceChildren();

    if (isBrand) {
      slot.classList.add("is-brand");
      slot.classList.remove("has-favicon");
      slot.innerHTML = RUTHENIUM_FAVICON_SVG;
      return;
    }

    slot.classList.remove("is-brand", "has-favicon");

    const fallback = document.createElement("span");
    fallback.className = "favicon-fallback";
    fallback.textContent = origin.replace(/^www\./i, "").charAt(0).toUpperCase() || "W";
    slot.appendChild(fallback);

    const candidates = faviconCandidatesForTab(tab);
    if (candidates.length === 0) return;

    const img = document.createElement("img");
    img.className = "favicon-img";
    img.alt = "";
    img.width = sizePx;
    img.height = sizePx;
    img.decoding = "async";
    img.referrerPolicy = "no-referrer";
    setHidden(img, true);

    let idx = 0;
    const tryNext = () => {
      if (slot.dataset.faviconKey !== favKey) return;
      if (idx >= candidates.length) {
        img.remove();
        return;
      }
      img.src = candidates[idx];
      idx += 1;
    };

    img.addEventListener("load", () => {
      if (slot.dataset.faviconKey !== favKey) return;
      if (img.naturalWidth > 0) {
        slot.classList.add("has-favicon");
        setHidden(img, false);
        setHidden(fallback, true);
      } else {
        tryNext();
      }
    });
    img.addEventListener("error", tryNext);

    slot.appendChild(img);
    tryNext();
  }

  function createTabElement(tabId) {
    const row = document.createElement("div");
    row.className = "tab-item";
    row.setAttribute("role", "tab");
    row.dataset.tabId = String(tabId);

    const favicon = document.createElement("span");
    favicon.className = "tab-favicon";
    favicon.setAttribute("aria-hidden", "true");

    const spinner = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    spinner.setAttribute("class", "tab-spinner");
    spinner.setAttribute("viewBox", "0 0 14 14");
    spinner.setAttribute("fill", "none");
    spinner.setAttribute("stroke", "currentColor");
    spinner.setAttribute("stroke-width", "1.6");
    spinner.setAttribute("stroke-linecap", "round");
    spinner.setAttribute("aria-hidden", "true");
    spinner.setAttribute("hidden", "");
    spinner.innerHTML = SPINNER_SVG;

    const title = document.createElement("span");
    title.className = "tab-title";

    const closeBtn = document.createElement("button");
    closeBtn.type = "button";
    closeBtn.className = "tab-close-btn";
    closeBtn.innerHTML = CLOSE_SVG;

    closeBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      post(`tab:close:${tabId}`);
    });

    row.addEventListener("click", () => {
      if (row.classList.contains("is-active")) {
        post("content:focus");
      } else {
        post(`tab:select:${tabId}`);
      }
    });

    row.addEventListener("auxclick", (e) => {
      if (e.button === 1) {
        e.preventDefault();
        post(`tab:close:${tabId}`);
      }
    });

    row.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        post(`tab:select:${tabId}`);
      } else if (e.key === "Delete") {
        e.preventDefault();
        post(`tab:close:${tabId}`);
      } else if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
        e.preventDefault();
        const items = Array.from(tabList.querySelectorAll(".tab-item"));
        const idx = items.indexOf(row);
        if (idx === -1 || items.length === 0) return;
        const nextIdx = e.key === "ArrowRight"
          ? (idx + 1) % items.length
          : (idx - 1 + items.length) % items.length;
        items[nextIdx].focus();
      }
    });

    row.append(favicon, spinner, title, closeBtn);
    return { row, favicon, spinner, title, closeBtn };
  }

  function createSidebarTabElement(tabId) {
    const row = document.createElement("button");
    row.type = "button";
    row.className = "sidebar-item sidebar-tab-row";

    const icon = document.createElement("span");
    icon.className = "sidebar-tab-icon";
    icon.setAttribute("aria-hidden", "true");

    const spinner = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    spinner.setAttribute("class", "tab-spinner");
    spinner.setAttribute("viewBox", "0 0 14 14");
    spinner.setAttribute("fill", "none");
    spinner.setAttribute("stroke", "currentColor");
    spinner.setAttribute("stroke-width", "1.6");
    spinner.setAttribute("stroke-linecap", "round");
    spinner.setAttribute("aria-hidden", "true");
    spinner.setAttribute("hidden", "");
    spinner.innerHTML = SPINNER_SVG;

    const meta = document.createElement("span");
    meta.className = "sidebar-tab-meta";

    const title = document.createElement("span");
    title.className = "sidebar-tab-title";

    const domain = document.createElement("span");
    domain.className = "sidebar-tab-domain";

    meta.append(title, domain);
    row.append(icon, spinner, meta);
    row.addEventListener("click", () => post(`tab:select:${tabId}`));

    return { row, icon, spinner, title, domain };
  }

  function renderSidebarTabs(tabs) {
    const incomingIds = new Set(tabs.map((t) => t.id));
    for (const [id, entry] of sidebarTabElements.entries()) {
      if (!incomingIds.has(id)) {
        entry.row.remove();
        sidebarTabElements.delete(id);
      }
    }

    let previousNode = null;
    for (const tab of tabs) {
      let entry = sidebarTabElements.get(tab.id);
      if (!entry) {
        entry = createSidebarTabElement(tab.id);
        sidebarTabElements.set(tab.id, entry);
      }

      const displayTitle = tab.title || tab.origin || "Beranda";
      const displayDomain = tab.origin || "browser://newtab";

      entry.row.classList.toggle("is-active", Boolean(tab.isActive));
      entry.row.setAttribute("title", `${displayTitle} (${displayDomain})`);
      entry.title.textContent = displayTitle;
      entry.domain.textContent = displayDomain;

      setHidden(entry.spinner, !tab.isLoading);
      setHidden(entry.icon, Boolean(tab.isLoading));
      syncFaviconSlot(entry.icon, tab, 18);

      const expectedNext = previousNode ? previousNode.nextSibling : sidebarTabList.firstChild;
      if (entry.row !== expectedNext) {
        sidebarTabList.insertBefore(entry.row, expectedNext);
      }
      previousNode = entry.row;
    }
  }

  function renderTabs(tabs) {
    const incomingIds = new Set(tabs.map((t) => t.id));
    for (const [id, entry] of tabElements.entries()) {
      if (!incomingIds.has(id)) {
        entry.row.remove();
        tabElements.delete(id);
      }
    }

    let previousNode = null;
    for (const tab of tabs) {
      let entry = tabElements.get(tab.id);
      if (!entry) {
        entry = createTabElement(tab.id);
        tabElements.set(tab.id, entry);
      }

      const displayTitle = tab.title || tab.origin || "Tab Baru";
      const tooltip = tab.origin ? `${displayTitle} (${tab.origin})` : displayTitle;

      entry.row.classList.toggle("is-active", Boolean(tab.isActive));
      entry.row.setAttribute("aria-selected", String(Boolean(tab.isActive)));
      entry.row.setAttribute("tabindex", tab.isActive ? "0" : "-1");
      entry.row.setAttribute("title", tooltip);
      entry.row.setAttribute("aria-label", displayTitle);

      entry.title.textContent = displayTitle;
      entry.closeBtn.setAttribute("aria-label", `Tutup ${displayTitle}`);
      entry.closeBtn.setAttribute("title", "Tutup tab (Ctrl+W)");

      setHidden(entry.spinner, !tab.isLoading);
      setHidden(entry.favicon, Boolean(tab.isLoading));
      syncFaviconSlot(entry.favicon, tab, 14);

      const expectedNext = previousNode ? previousNode.nextSibling : tabList.firstChild;
      if (entry.row !== expectedNext) {
        tabList.insertBefore(entry.row, expectedNext);
      }
      previousNode = entry.row;
    }

    renderSidebarTabs(tabs);
  }

  function focusAddressBar() {
    addressForm.classList.add("is-focused");
    addressInput.value = currentState.rawUrl || "";
    addressInput.focus();
    addressInput.select();
  }

  function blurAddressBar(revert) {
    if (revert) {
      addressInput.value = currentState.rawUrl || "";
    }
    addressForm.classList.remove("is-focused");
    addressInput.blur();
  }

  window.__focusAddressBar = focusAddressBar;

  function renderPanelViews(sidebarOpen) {
    const isWorkspaceView = sidebarOpen && activeSidebarPanel === "workspace";
    const isSettingsView = sidebarOpen && activeSidebarPanel === "settings";

    setHidden(sidebarDefaultView, !isWorkspaceView);
    setHidden(settingsPanel, !isSettingsView);

    for (const btn of switcherBtns) {
      btn.setAttribute("aria-pressed", String(btn.dataset.panelTab === activeSidebarPanel));
    }

    btnSidebar.setAttribute("aria-expanded", String(isWorkspaceView));
    btnSettings.setAttribute("aria-expanded", String(isSettingsView));

    const activeTheme = VALID_THEMES.has(currentState.theme) ? currentState.theme : "arctic";
    document.documentElement.dataset.theme = activeTheme;
    for (const btn of themeButtons) {
      btn.setAttribute("aria-pressed", String(btn.dataset.shellTheme === activeTheme));
    }

    const activeEngine = ENGINE_LABELS[currentState.searchEngine]
      ? currentState.searchEngine
      : "google";
    if (document.activeElement !== engineSelect) {
      engineSelect.value = activeEngine;
    }
  }

  function openSidebarPanel(panel) {
    const sidebarOpen = currentState.sidebarWidth > 0 && !currentState.isCompact;
    if (sidebarOpen && activeSidebarPanel === panel) {
      post("sidebar:close");
      return;
    }
    activeSidebarPanel = panel;
    if (sidebarOpen) {
      renderPanelViews(true);
    } else {
      post("sidebar:open");
    }
  }

  function flashSettingsStatus(text) {
    settingsStatus.textContent = text;
    clearTimeout(settingsStatusTimer);
    settingsStatusTimer = setTimeout(() => {
      settingsStatus.textContent = "";
    }, 1800);
  }

  function setThemeMode(themeName) {
    const theme = VALID_THEMES.has(themeName) ? themeName : "arctic";
    currentState.theme = theme;
    saveChromePrefs();
    renderPanelViews(currentState.sidebarWidth > 0 && !currentState.isCompact);
    post(`theme:${theme}`);
    flashSettingsStatus(THEME_LABELS[theme]);
  }

  window.__shellSync = (next) => {
    const prevRawUrl = currentState.rawUrl;
    const prevEngine = currentState.searchEngine;
    const prevTheme = currentState.theme;
    currentState = { ...currentState, ...next };

    if (prevEngine !== currentState.searchEngine || prevTheme !== currentState.theme) {
      saveChromePrefs();
    }

    document.documentElement.dataset.windowFocused = String(Boolean(currentState.windowFocused));
    shellRoot.style.setProperty("--sidebar-width", `${currentState.sidebarWidth}px`);

    const sidebarOpen = currentState.sidebarWidth > 0 && !currentState.isCompact;
    setHidden(sidebar, !sidebarOpen);
    renderPanelViews(sidebarOpen);

    btnBack.disabled = !currentState.canGoBack;
    btnForward.disabled = !currentState.canGoForward;

    setHidden(iconReload, Boolean(currentState.isLoading));
    setHidden(iconStop, !currentState.isLoading);
    btnReload.setAttribute("aria-label", currentState.isLoading ? "Hentikan" : "Muat ulang");
    btnReload.setAttribute("title", currentState.isLoading ? "Hentikan (Escape)" : "Muat ulang (Ctrl+R)");

    setLoadingVisual(Boolean(currentState.isLoading));
    renderSecurity(currentState.security);

    const placeholderText = searchPlaceholderFor(currentState.searchEngine);
    addressInput.placeholder = placeholderText;
    if (currentState.origin) {
      originDomain.textContent = currentState.origin;
      originDomain.classList.remove("is-placeholder");
    } else {
      originDomain.textContent = placeholderText;
      originDomain.classList.add("is-placeholder");
    }
    originPath.textContent = currentState.path || "";
    setHidden(btnShare, !currentState.rawUrl);

    if (!addressForm.classList.contains("is-focused") || prevRawUrl !== currentState.rawUrl) {
      addressInput.value = currentState.rawUrl || "";
    }

    renderTabs(Array.isArray(currentState.tabs) ? currentState.tabs : []);

    setHidden(iconMaximize, Boolean(currentState.isMaximized));
    setHidden(iconRestore, !currentState.isMaximized);
    const maxLabel = currentState.isMaximized ? "Pulihkan" : "Perbesar";
    btnMaximize.setAttribute("aria-label", maxLabel);
    btnMaximize.setAttribute("title", maxLabel);
  };

  btnSidebar.addEventListener("click", () => openSidebarPanel("workspace"));
  btnSettings.addEventListener("click", () => openSidebarPanel("settings"));
  sidebarCloseBtn.addEventListener("click", () => post("sidebar:close"));

  for (const btn of switcherBtns) {
    btn.addEventListener("click", () => {
      activeSidebarPanel = btn.dataset.panelTab || "workspace";
      renderPanelViews(currentState.sidebarWidth > 0 && !currentState.isCompact);
    });
  }

  for (const btn of themeButtons) {
    btn.addEventListener("click", () => setThemeMode(btn.dataset.shellTheme));
  }

  engineSelect.addEventListener("change", () => {
    const nextEngine = ENGINE_LABELS[engineSelect.value] ? engineSelect.value : "google";
    currentState.searchEngine = nextEngine;
    saveChromePrefs();
    post(`prefs:engine:${nextEngine}`);
    flashSettingsStatus("Mesin pencari disimpan");
  });

  sidebarNewTabBtn.addEventListener("click", () => post("tab:new"));

  if (shellQuickNotes) {
    try {
      shellQuickNotes.value = localStorage.getItem(SHELL_NOTES_KEY) || "";
    } catch (_) {}
    shellQuickNotes.addEventListener("input", () => {
      try {
        localStorage.setItem(SHELL_NOTES_KEY, shellQuickNotes.value);
      } catch (_) {}
    });
  }

  btnBack.addEventListener("click", () => post("nav:back"));
  btnForward.addEventListener("click", () => post("nav:forward"));
  btnReload.addEventListener("click", (e) => {
    e.stopPropagation();
    post(currentState.isLoading ? "nav:stop" : "nav:reload");
  });

  btnShare.addEventListener("click", () => {
    const text = currentState.rawUrl || currentState.origin || "browser://newtab";
    const done = () => {
      setHidden(iconShare, true);
      setHidden(iconShareCheck, false);
      clearTimeout(shareCheckTimer);
      shareCheckTimer = setTimeout(() => {
        setHidden(iconShare, false);
        setHidden(iconShareCheck, true);
      }, 1400);
    };
    if (navigator.clipboard && typeof navigator.clipboard.writeText === "function") {
      navigator.clipboard.writeText(text).then(done, done);
    } else {
      done();
    }
  });

  btnNewTab.addEventListener("click", () => post("tab:new"));
  btnMinimize.addEventListener("click", () => post("win:minimize"));
  btnMaximize.addEventListener("click", () => post("win:maximize"));
  btnClose.addEventListener("click", () => post("win:close"));

  addressForm.addEventListener("mousedown", (e) => {
    if (e.target.closest("#btn-share")) return;
    if (!addressForm.classList.contains("is-focused")) {
      e.preventDefault();
      focusAddressBar();
    }
  });

  addressInput.addEventListener("focus", () => {
    if (!addressForm.classList.contains("is-focused")) {
      focusAddressBar();
    }
  });

  function loadShellHistory() {
    try {
      const raw = localStorage.getItem(SHELL_HISTORY_KEY);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed
        .filter((entry) => typeof entry === "string" && entry.trim())
        .slice(0, MAX_SHELL_HISTORY);
    } catch (_) {
      return [];
    }
  }

  function recordShellHistory(raw) {
    const query = raw.trim();
    if (shouldSkipOmniboxSuggest(query) || /\S+\.\S+/.test(query)) {
      return;
    }
    const lower = query.toLowerCase();
    const next = [query, ...loadShellHistory().filter((item) => item.toLowerCase() !== lower)]
      .slice(0, MAX_SHELL_HISTORY);
    try {
      localStorage.setItem(SHELL_HISTORY_KEY, JSON.stringify(next));
    } catch (_) {}
  }

  function clearOmniboxSuggestions() {
    clearTimeout(omniboxSuggestTimer);
    omniboxSuggestTimer = null;
    if (omniboxSuggestAbort) {
      omniboxSuggestAbort.abort();
      omniboxSuggestAbort = null;
    }
    currentOmniboxOptions.clear();
    if (addressDatalist) {
      addressDatalist.replaceChildren();
    }
  }

  function renderOmniboxSuggestions(items) {
    if (!addressDatalist) return;
    currentOmniboxOptions.clear();
    addressDatalist.replaceChildren();
    for (const text of items) {
      currentOmniboxOptions.add(text);
      const option = document.createElement("option");
      option.value = text;
      addressDatalist.appendChild(option);
    }
  }

  function shouldSkipOmniboxSuggest(trimmed) {
    const lower = trimmed.toLowerCase();
    return (
      !trimmed
      || lower.startsWith("http://")
      || lower.startsWith("https://")
      || lower.startsWith("browser://")
      || lower === "localhost"
      || lower.startsWith("localhost:")
      || lower.startsWith("localhost/")
      || lower.startsWith("127.0.0.1")
      || lower.startsWith("[::1]")
    );
  }

  function mergeOmniboxCandidates(trimmed, remoteItems) {
    const out = [];
    const seen = new Set();
    const pushUnique = (text) => {
      const clean = text.trim();
      if (!clean) return;
      const lower = clean.toLowerCase();
      if (seen.has(lower)) return;
      seen.add(lower);
      out.push(clean);
    };

    const lowerQuery = trimmed.toLowerCase();
    for (const hist of loadShellHistory()) {
      if (!lowerQuery || hist.toLowerCase().includes(lowerQuery)) {
        pushUnique(hist);
        if (out.length >= 3) break;
      }
    }
    if (Array.isArray(remoteItems)) {
      for (const s of remoteItems) {
        if (out.length >= 8) break;
        if (typeof s === "string") pushUnique(s);
      }
    }
    return out.slice(0, 8);
  }

  function tryInlineComplete(typed, candidates) {
    if (
      document.activeElement !== addressInput
      || typed.length < 2
      || addressInput.value !== typed
      || addressInput.selectionStart !== typed.length
      || addressInput.selectionEnd !== typed.length
    ) {
      return;
    }
    const lowerTyped = typed.toLowerCase();
    const match = candidates.find(
      (item) => item.toLowerCase().startsWith(lowerTyped) && item.length > typed.length,
    );
    if (!match) return;
    const completed = typed + match.slice(typed.length);
    addressInput.value = completed;
    addressInput.setSelectionRange(typed.length, completed.length);
  }

  function isForwardTypingInput(inputType) {
    return inputType === "insertText" || inputType === "insertCompositionText";
  }

  function scheduleOmniboxSuggest(rawValue, inputType) {
    clearTimeout(omniboxSuggestTimer);
    if (omniboxSuggestAbort) {
      omniboxSuggestAbort.abort();
      omniboxSuggestAbort = null;
    }

    const trimmed = rawValue.trim();
    if (shouldSkipOmniboxSuggest(trimmed)) {
      currentOmniboxOptions.clear();
      if (addressDatalist) {
        addressDatalist.replaceChildren();
      }
      return;
    }

    const allowInline = isForwardTypingInput(inputType)
      && rawValue === trimmed
      && addressInput.selectionStart === rawValue.length
      && addressInput.selectionEnd === rawValue.length;

    const cacheKey = trimmed.toLowerCase();
    const cachedRemote = omniboxSuggestCache.get(cacheKey);
    const localCandidates = mergeOmniboxCandidates(trimmed, cachedRemote || []);
    if (localCandidates.length > 0) {
      renderOmniboxSuggestions(localCandidates);
      if (allowInline) {
        tryInlineComplete(rawValue, localCandidates);
      }
    }

    if (cachedRemote) {
      return;
    }

    const seq = ++omniboxSuggestSeq;
    const rawSnapshot = rawValue;
    omniboxSuggestTimer = setTimeout(() => {
      const controller = new AbortController();
      omniboxSuggestAbort = controller;
      fetch(`/suggest?q=${encodeURIComponent(trimmed)}`, {
        signal: controller.signal,
        cache: "no-store",
      })
        .then((res) => (res.ok ? res.json() : []))
        .then((list) => {
          if (seq !== omniboxSuggestSeq) {
            return;
          }
          if (!Array.isArray(list)) return;
          const remoteItems = list
            .filter((entry) => typeof entry === "string" && entry.trim())
            .slice(0, 8);
          if (omniboxSuggestCache.size >= 40) {
            const oldest = omniboxSuggestCache.keys().next().value;
            if (oldest !== undefined) {
              omniboxSuggestCache.delete(oldest);
            }
          }
          omniboxSuggestCache.set(cacheKey, remoteItems);
          if (addressInput.value !== rawSnapshot) {
            return;
          }
          const merged = mergeOmniboxCandidates(trimmed, remoteItems);
          renderOmniboxSuggestions(merged);
          if (allowInline) {
            tryInlineComplete(rawSnapshot, merged);
          }
        })
        .catch(() => {
          // Ignore aborts or offline errors cleanly.
        });
    }, 80);
  }

  addressInput.addEventListener("input", (e) => {
    const val = addressInput.value;
    const trimmed = val.trim();
    const inputType = e && typeof e.inputType === "string" ? e.inputType : "";
    if (
      (inputType === "insertReplacementText" || !inputType)
      && trimmed
      && currentOmniboxOptions.has(trimmed)
    ) {
      recordShellHistory(trimmed);
      clearOmniboxSuggestions();
      blurAddressBar(false);
      post(`nav:go:${trimmed}`);
      return;
    }
    scheduleOmniboxSuggest(val, inputType);
  });

  addressInput.addEventListener("blur", () => {
    clearOmniboxSuggestions();
    blurAddressBar(false);
  });

  addressForm.addEventListener("submit", (e) => {
    e.preventDefault();
    const query = addressInput.value.trim();
    recordShellHistory(query);
    clearOmniboxSuggestions();
    blurAddressBar(false);
    post(`nav:go:${query}`);
  });

  addressInput.addEventListener("keydown", (e) => {
    if (
      (e.key === "Tab" || e.key === "ArrowRight")
      && addressInput.selectionStart !== addressInput.selectionEnd
      && addressInput.selectionEnd === addressInput.value.length
    ) {
      const end = addressInput.value.length;
      addressInput.setSelectionRange(end, end);
      if (e.key === "Tab") {
        e.preventDefault();
      }
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      clearOmniboxSuggestions();
      blurAddressBar(true);
      post("content:focus");
    }
  });

  document.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    const edge = detectWindowResizeEdge(e.clientX, e.clientY);
    if (edge) {
      e.preventDefault();
      post(`win:resize:${edge}`);
      return;
    }
    if (
      e.target.closest("[data-drag-region='true']")
      && !e.target.closest("button, [role='tab'], input, select, .tab-list")
    ) {
      if (e.detail === 2) {
        post("win:maximize");
      } else {
        post("win:drag");
      }
    }
  });

  let resizingSidebar = false;

  sidebarResizer.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    resizingSidebar = true;
    sidebarResizer.setPointerCapture(e.pointerId);
  });

  sidebarResizer.addEventListener("pointermove", (e) => {
    if (!resizingSidebar) return;
    post(`sidebar:width:${Math.round(e.clientX)}`);
  });

  const endSidebarResize = (e) => {
    if (!resizingSidebar) return;
    resizingSidebar = false;
    if (e && sidebarResizer.hasPointerCapture(e.pointerId)) {
      sidebarResizer.releasePointerCapture(e.pointerId);
    }
  };

  sidebarResizer.addEventListener("pointerup", endSidebarResize);
  sidebarResizer.addEventListener("pointercancel", endSidebarResize);
  sidebarResizer.addEventListener("dblclick", (e) => {
    e.preventDefault();
    e.stopPropagation();
    post("sidebar:reset_width");
  });

  function detectWindowResizeEdge(x, y) {
    if (currentState.isMaximized || resizingSidebar) return null;
    const inset = 5;
    const w = window.innerWidth;
    const h = window.innerHeight;
    if (x >= w - 134 && y < 44) return null;

    const onLeft = x < inset;
    const onTop = y < inset;
    const onBottom = y >= h - inset && x < currentState.sidebarWidth;

    if (onTop && onLeft) return "nw";
    if (onBottom && onLeft) return "sw";
    if (onLeft) return "w";
    if (onTop) return "n";
    if (onBottom) return "s";
    return null;
  }

  document.addEventListener("mousemove", (e) => {
    if (resizingSidebar) return;
    const edge = detectWindowResizeEdge(e.clientX, e.clientY);
    const cursorMap = {
      nw: "nwse-resize",
      sw: "nesw-resize",
      w: "ew-resize",
      n: "ns-resize",
      s: "ns-resize",
    };
    document.body.style.cursor = edge ? cursorMap[edge] : "";
  });

  document.addEventListener("contextmenu", (e) => {
    if (e.target !== addressInput) {
      e.preventDefault();
    }
  });

  window.addEventListener("keydown", (e) => {
    const key = e.key;
    const lower = key.toLowerCase();

    if (e.ctrlKey && !e.altKey) {
      if (key === "Tab") {
        e.preventDefault();
        blurAddressBar(false);
        post(e.shiftKey ? "tab:prev" : "tab:next");
        return;
      }
      if (!e.shiftKey && lower === "t") {
        e.preventDefault();
        post("tab:new");
        return;
      }
      if ((!e.shiftKey && lower === "w") || key === "F4") {
        e.preventDefault();
        blurAddressBar(false);
        post("tab:close_active");
        return;
      }
      if (!e.shiftKey && lower === "l") {
        e.preventDefault();
        focusAddressBar();
        return;
      }
      if (!e.shiftKey && lower === "b") {
        e.preventDefault();
        activeSidebarPanel = "workspace";
        post("sidebar:toggle");
        return;
      }
      if (!e.shiftKey && lower === "r") {
        e.preventDefault();
        post("nav:reload");
        return;
      }
      if (!e.shiftKey && key >= "1" && key <= "9") {
        e.preventDefault();
        blurAddressBar(false);
        post(`tab:slot:${key}`);
        return;
      }
    }

    if (e.altKey && !e.ctrlKey && !e.shiftKey) {
      if (lower === "d") {
        e.preventDefault();
        focusAddressBar();
        return;
      }
      if (key === "ArrowLeft") {
        e.preventDefault();
        post("nav:back");
        return;
      }
      if (key === "ArrowRight") {
        e.preventDefault();
        post("nav:forward");
        return;
      }
    }

    if (!e.ctrlKey && !e.altKey && !e.shiftKey) {
      if (key === "F5") {
        e.preventDefault();
        post("nav:reload");
      } else if (key === "Escape" && currentState.isLoading) {
        e.preventDefault();
        post("nav:stop");
      }
    }
  });

  const savedPrefs = loadChromePrefs();
  const initialTheme = savedPrefs ? savedPrefs.theme : "arctic";
  if (savedPrefs && savedPrefs.searchEngine) {
    currentState.searchEngine = savedPrefs.searchEngine;
    post(`prefs:engine:${savedPrefs.searchEngine}`);
  }
  currentState.theme = initialTheme;
  document.documentElement.dataset.theme = initialTheme;
  post(`theme:${initialTheme}`);

  post("shell:ready");
})();
