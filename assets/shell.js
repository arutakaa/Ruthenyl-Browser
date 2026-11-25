(() => {
  const post = (msg) => {
    if (window.ipc && typeof window.ipc.postMessage === "function") {
      window.ipc.postMessage(msg);
    }
  };

  const RUTHENIUM_FAVICON_SVG = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"><polygon points="32,4 56,18 56,46 32,60 8,46 8,18" fill="#0f2347"/><polygon points="32,4 56,18 32,32 8,18" fill="#1e3f78"/><polygon points="32,32 56,18 56,46 32,60" fill="#09152b"/><path d="M23 20H35C39.5 20 42.5 22.5 42.5 26.5C42.5 29.8 40.3 32.1 37 32.8L43 43H36.8L31.5 33.4H28.2V43H23V20ZM28.2 24.4V29.2H34.2C36.3 29.2 37.5 28.2 37.5 26.8C37.5 25.3 36.3 24.4 34.2 24.4H28.2Z" fill="#7dd3fc"/></svg>`;
  const SPINNER_SVG = `<path d="M7 1.5a5.5 5.5 0 1 1-4.6 2.5"></path>`;
  const CLOSE_SVG = `<svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><line x1="3" y1="3" x2="9" y2="9"></line><line x1="9" y1="3" x2="3" y2="9"></line></svg>`;
  const PREFS_STORAGE_KEY = "ruthenium_chrome_prefs_v4";
  const LEGACY_V3_PREFS_KEY = "ruthenium_chrome_prefs_v3";
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
      const rawV4 = localStorage.getItem(PREFS_STORAGE_KEY);
      if (rawV4) {
        const parsed = JSON.parse(rawV4);
        if (parsed && typeof parsed === "object") {
          const searchEngine = typeof parsed.searchEngine === "string" && ENGINE_LABELS[parsed.searchEngine]
            ? parsed.searchEngine
            : "google";
          const adblockEnabled = typeof parsed.adblockEnabled === "boolean"
            ? parsed.adblockEnabled
            : true;
          return { searchEngine, theme: normalizeTheme(parsed.theme), adblockEnabled };
        }
      }
      const rawV3 = localStorage.getItem(LEGACY_V3_PREFS_KEY);
      if (rawV3) {
        const parsed = JSON.parse(rawV3);
        if (parsed && typeof parsed === "object") {
          const searchEngine = typeof parsed.searchEngine === "string" && ENGINE_LABELS[parsed.searchEngine]
            ? parsed.searchEngine
            : "google";
          return { searchEngine, theme: normalizeTheme(parsed.theme), adblockEnabled: true };
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
          return { searchEngine, theme: "arctic", adblockEnabled: true };
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
          adblockEnabled: Boolean(currentState.adblockEnabled),
        }),
      );
    } catch (_) {
      // Ignore storage quota errors so controls stay responsive.
    }
  }

  const shellRoot = document.getElementById("shell-root");
  const sidebar = document.getElementById("sidebar");
  const sidebarResizer = document.getElementById("sidebar-resizer");
  const sidebarTabList = document.getElementById("sidebar-tab-list");
  const sidebarNewTabBtn = document.getElementById("sidebar-new-tab-btn");
  const sidebarCloseBtn = document.getElementById("sidebar-close-btn");
  const shellQuickNotes = document.getElementById("shell-quick-notes");

  const tabList = document.getElementById("tab-list");
  const btnAdblock = document.getElementById("btn-adblock");
  const adblockCountBadge = document.getElementById("adblock-count-badge");
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
  const omniboxClearBtn = document.getElementById("btn-omnibox-clear");
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
    adblockEnabled: true,
    adblockBlockedTab: 0,
    adblockBlockedTotal: 0,
    adblockStatus: "Aktif (Agresif) · Filter dasar siap",
    adblockRefreshing: false,
    adblockSiteWhitelisted: false,
    suggestEnabled: true,
    isDefaultBrowser: false,
  };

  let loadingTimer = null;
  let shareCheckTimer = null;
  const tabElements = new Map();
  const sidebarTabElements = new Map();
  const omniboxSuggestCache = new Map();
  const MAX_VISIBLE_SUGGESTIONS = 8;
  const SUGGEST_DEBOUNCE_MS = 65;
  const MAX_SUGGEST_CACHE = 40;
  let omniboxSuggestTimer = null;
  let omniboxSuggestAbort = null;
  let omniboxSuggestSeq = 0;
  let omniboxOpen = false;
  let activeOmniboxItems = [];
  let selectedOmniboxIndex = -1;
  let draftOmniboxQuery = "";

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

  function focusAddressBar(openDropdown = true) {
    const shouldOpen = typeof openDropdown === "boolean"
      ? openDropdown
      : !(openDropdown && openDropdown.openDropdown === false);
    addressForm.classList.add("is-focused");
    syncOmniboxLeadingIcon();
    addressInput.value = currentState.rawUrl || "";
    draftOmniboxQuery = addressInput.value;
    syncOmniboxClearBtn();
    addressInput.focus();
    addressInput.select();
    if (shouldOpen) {
      requestOmniboxSuggestions(draftOmniboxQuery, { immediate: true });
    } else {
      closeOmniboxPopup({ notifyHost: true });
    }
  }

  function blurAddressBar(revert) {
    cancelPendingOmniboxSuggest();
    closeOmniboxPopup({ notifyHost: false });
    if (revert) {
      addressInput.value = currentState.rawUrl || "";
    }
    addressForm.classList.remove("is-focused");
    syncOmniboxLeadingIcon();
    syncOmniboxClearBtn();
    addressInput.blur();
  }

  window.__focusAddressBar = focusAddressBar;
  window.__omniboxCloseUi = (blurInput = false) => {
    if (blurInput) {
      blurAddressBar(true);
    } else {
      closeOmniboxPopup({ notifyHost: false });
    }
  };
  window.__omniboxFill = (text) => {
    if (typeof text !== "string") return;
    addressForm.classList.add("is-focused");
    syncOmniboxLeadingIcon();
    addressInput.value = text;
    draftOmniboxQuery = text;
    syncOmniboxClearBtn();
    addressInput.focus();
    const end = addressInput.value.length;
    addressInput.setSelectionRange(end, end);
    requestOmniboxSuggestions(text, { immediate: true });
  };
  window.__omniboxRefresh = (focusInput = false) => {
    if (focusInput && addressForm.classList.contains("is-focused")) {
      addressInput.focus();
    }
    if (addressForm.classList.contains("is-focused")) {
      requestOmniboxSuggestions(draftOmniboxQuery, { immediate: true });
    }
  };
  window.__syncHistoryAdd = (query) => {
    recordShellHistory(query);
  };
  window.__syncHistoryRemove = (query) => {
    removeShellHistoryItem(query);
  };
  window.__syncHistoryClear = () => {
    clearShellHistory();
  };

  function renderPanelViews(sidebarOpen) {
    btnSidebar.setAttribute("aria-expanded", String(Boolean(sidebarOpen)));

    const activeTheme = VALID_THEMES.has(currentState.theme) ? currentState.theme : "arctic";
    document.documentElement.dataset.theme = activeTheme;

    renderAdblockControls();
  }

  function renderAdblockControls() {
    const enabled = Boolean(currentState.adblockEnabled);
    const siteWhitelisted = Boolean(currentState.adblockSiteWhitelisted);
    const siteActive = enabled && !siteWhitelisted;
    const tabCount = Number(currentState.adblockBlockedTab) || 0;

    if (btnAdblock) {
      btnAdblock.classList.toggle("is-enabled", siteActive);
      btnAdblock.setAttribute("aria-pressed", String(siteActive));
      let label = "Pemblokir iklan nonaktif secara global";
      if (enabled && siteWhitelisted) {
        label = "Pemblokir iklan dijeda untuk situs ini (klik untuk aktifkan)";
      } else if (enabled && tabCount > 0) {
        label = `Pemblokir iklan aktif (${tabCount} diblokir di tab ini, klik untuk kecualikan situs ini)`;
      } else if (enabled) {
        label = "Pemblokir iklan agresif aktif (klik untuk kecualikan situs ini)";
      }
      btnAdblock.setAttribute("aria-label", label);
      btnAdblock.setAttribute("title", label);
    }

    if (adblockCountBadge) {
      const showCount = siteActive && tabCount > 0;
      setHidden(adblockCountBadge, !showCount);
      if (showCount) {
        adblockCountBadge.textContent = tabCount > 99 ? "99+" : String(tabCount);
      }
    }
  }

  window.__shellSync = (next) => {
    const prevRawUrl = currentState.rawUrl;
    const prevEngine = currentState.searchEngine;
    const prevTheme = currentState.theme;
    const prevAdblock = currentState.adblockEnabled;
    currentState = { ...currentState, ...next };

    if (
      prevEngine !== currentState.searchEngine
      || prevTheme !== currentState.theme
      || prevAdblock !== currentState.adblockEnabled
    ) {
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
      syncOmniboxClearBtn();
    }
    syncOmniboxLeadingIcon();

    renderTabs(Array.isArray(currentState.tabs) ? currentState.tabs : []);

    setHidden(iconMaximize, Boolean(currentState.isMaximized));
    setHidden(iconRestore, !currentState.isMaximized);
    const maxLabel = currentState.isMaximized ? "Pulihkan" : "Perbesar";
    btnMaximize.setAttribute("aria-label", maxLabel);
    btnMaximize.setAttribute("title", maxLabel);
  };

  if (btnAdblock) {
    btnAdblock.addEventListener("click", () => {
      post("adblock:toggle_site");
    });
  }

  btnSidebar.addEventListener("click", () => post("sidebar:toggle"));
  btnSettings.addEventListener("click", () => post("settings:open"));
  sidebarCloseBtn.addEventListener("click", () => post("sidebar:close"));

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
    if (e.target.closest("#btn-share") || e.target.closest("#btn-omnibox-clear")) {
      return;
    }
    if (!addressForm.classList.contains("is-focused")) {
      e.preventDefault();
      focusAddressBar(true);
    } else if (!omniboxOpen) {
      requestOmniboxSuggestions(addressInput.value, { immediate: true });
    }
  });

  addressInput.addEventListener("focus", () => {
    if (!addressForm.classList.contains("is-focused")) {
      focusAddressBar(true);
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

  function saveShellHistory(list) {
    try {
      localStorage.setItem(SHELL_HISTORY_KEY, JSON.stringify(list.slice(0, MAX_SHELL_HISTORY)));
    } catch (_) {}
  }

  function isExplicitUrlInput(raw) {
    const trimmed = raw.trim();
    const lower = trimmed.toLowerCase();
    return (
      lower.startsWith("http://")
      || lower.startsWith("https://")
      || lower.startsWith("browser://")
      || lower === "localhost"
      || lower.startsWith("localhost:")
      || lower.startsWith("localhost/")
      || lower.startsWith("127.0.0.1")
      || lower.startsWith("[::1]")
    );
  }

  function looksLikeDomainOrUrl(raw) {
    const trimmed = raw.trim();
    if (!trimmed || /\s/.test(trimmed)) return false;
    if (isExplicitUrlInput(trimmed)) return true;
    const hostPort = trimmed.split("/")[0];
    const host = hostPort.split(":")[0];
    return host.includes(".") && !host.startsWith(".") && !host.endsWith(".");
  }

  function recordShellHistory(raw) {
    if (typeof raw !== "string") return;
    const query = raw.trim();
    if (!query || looksLikeDomainOrUrl(query)) {
      return;
    }
    const lower = query.toLowerCase();
    const next = [query, ...loadShellHistory().filter((item) => item.toLowerCase() !== lower)]
      .slice(0, MAX_SHELL_HISTORY);
    saveShellHistory(next);
  }

  function removeShellHistoryItem(target) {
    if (typeof target !== "string") return;
    const lower = target.trim().toLowerCase();
    const next = loadShellHistory().filter((item) => item.toLowerCase() !== lower);
    saveShellHistory(next);
  }

  function clearShellHistory() {
    saveShellHistory([]);
  }

  function syncOmniboxClearBtn() {
    const visible = addressForm.classList.contains("is-focused") && addressInput.value.length > 0;
    setHidden(omniboxClearBtn, !visible);
  }

  function syncOmniboxLeadingIcon() {
    if (addressForm.classList.contains("is-focused")) {
      setHidden(iconSearch, false);
      setHidden(iconLock, true);
      setHidden(iconWarn, true);
      setHidden(securityChip, true);
      securityBadge.classList.add("is-internal");
      securityBadge.classList.remove("is-insecure");
    } else {
      renderSecurity(currentState.security);
    }
  }

  function cancelPendingOmniboxSuggest() {
    clearTimeout(omniboxSuggestTimer);
    omniboxSuggestTimer = null;
    if (omniboxSuggestAbort) {
      omniboxSuggestAbort.abort();
      omniboxSuggestAbort = null;
    }
  }

  function computePopupHeight(rect, items, showHistoryFooter) {
    const contentHeight = 7 + items.length * 36 + (showHistoryFooter ? 30 : 0) + 9;
    const maxAvailable = Math.max(80, Math.min(360, Math.floor(window.innerHeight - rect.bottom - 28)));
    return Math.min(contentHeight, maxAvailable);
  }

  function closeOmniboxPopup({ notifyHost = true } = {}) {
    const wasOpen = omniboxOpen;
    omniboxOpen = false;
    activeOmniboxItems = [];
    selectedOmniboxIndex = -1;
    addressForm.classList.remove("is-open");
    addressInput.setAttribute("aria-expanded", "false");
    if (notifyHost && wasOpen) {
      post("omnibox:close");
    }
  }

  function openOrUpdateOmniboxPopup(query, items) {
    if (!items || items.length === 0 || !addressForm.classList.contains("is-focused")) {
      closeOmniboxPopup({ notifyHost: true });
      return;
    }

    activeOmniboxItems = items;
    selectedOmniboxIndex = -1;
    omniboxOpen = true;
    addressForm.classList.add("is-open");
    addressInput.setAttribute("aria-expanded", "true");

    const trimmedQuery = query.trim();
    const showHistoryFooter = trimmedQuery.length === 0 && items.some((entry) => entry.kind === "history");
    const rect = addressForm.getBoundingClientRect();
    const popupHeight = computePopupHeight(rect, items, showHistoryFooter);

    const payload = JSON.stringify({
      theme: VALID_THEMES.has(currentState.theme) ? currentState.theme : "arctic",
      query,
      items,
      showHistoryFooter,
      selectedIndex: -1,
    });
    post(`omnibox:open:${rect.left.toFixed(1)},${rect.bottom.toFixed(1)},${rect.width.toFixed(1)},${popupHeight}|${payload}`);
  }

  function cacheOmniboxSuggestions(cacheKey, items) {
    if (omniboxSuggestCache.size >= MAX_SUGGEST_CACHE) {
      const oldest = omniboxSuggestCache.keys().next().value;
      if (oldest !== undefined) {
        omniboxSuggestCache.delete(oldest);
      }
    }
    omniboxSuggestCache.set(cacheKey, items);
  }

  function mergeOmniboxSuggestions(trimmed, remoteList) {
    const items = [];
    const seen = new Set();
    const pushItem = (text, kind) => {
      const clean = text.trim();
      if (!clean) return;
      const key = clean.toLowerCase();
      if (seen.has(key)) return;
      seen.add(key);
      items.push({ text: clean, kind });
    };

    if (looksLikeDomainOrUrl(trimmed)) {
      pushItem(trimmed, "url");
    }

    const lower = trimmed.toLowerCase();
    const history = loadShellHistory();
    let historyAdded = 0;
    for (const h of history) {
      if (!lower || h.toLowerCase().includes(lower)) {
        pushItem(h, "history");
        historyAdded += 1;
        if (lower && historyAdded >= 3) break;
        if (!lower && items.length >= MAX_VISIBLE_SUGGESTIONS) break;
      }
    }

    if (Array.isArray(remoteList)) {
      for (const s of remoteList) {
        if (items.length >= MAX_VISIBLE_SUGGESTIONS) break;
        if (typeof s === "string") {
          pushItem(s, "suggest");
        }
      }
    }

    return items.slice(0, MAX_VISIBLE_SUGGESTIONS);
  }

  function findBestCachedRemote(lowerQuery) {
    const exact = omniboxSuggestCache.get(lowerQuery);
    if (exact) return exact;
    for (let len = lowerQuery.length - 1; len >= 1; len -= 1) {
      const prefix = lowerQuery.slice(0, len);
      const cachedPrefix = omniboxSuggestCache.get(prefix);
      if (cachedPrefix) {
        return cachedPrefix.filter((item) => item.toLowerCase().includes(lowerQuery));
      }
    }
    return [];
  }

  function requestOmniboxSuggestions(rawInput, { immediate = false } = {}) {
    cancelPendingOmniboxSuggest();
    const trimmed = rawInput.trim();

    if (!trimmed) {
      const recentItems = mergeOmniboxSuggestions("", []);
      openOrUpdateOmniboxPopup("", recentItems);
      return;
    }

    if (isExplicitUrlInput(trimmed)) {
      openOrUpdateOmniboxPopup(trimmed, [{ text: trimmed, kind: "url" }]);
      return;
    }

    const cacheKey = trimmed.toLowerCase();
    const exactCached = omniboxSuggestCache.get(cacheKey);
    const localItems = mergeOmniboxSuggestions(trimmed, findBestCachedRemote(cacheKey));
    if (localItems.length > 0) {
      openOrUpdateOmniboxPopup(trimmed, localItems);
    } else {
      closeOmniboxPopup({ notifyHost: true });
    }

    if (!currentState.suggestEnabled || exactCached) {
      return;
    }

    const seq = ++omniboxSuggestSeq;
    const runFetch = () => {
      const controller = new AbortController();
      omniboxSuggestAbort = controller;
      fetch(`/suggest?q=${encodeURIComponent(trimmed)}`, {
        signal: controller.signal,
        cache: "no-store",
      })
        .then((res) => (res.ok ? res.json() : []))
        .then((list) => {
          if (seq !== omniboxSuggestSeq || draftOmniboxQuery.trim() !== trimmed) {
            return;
          }
          if (!Array.isArray(list)) return;
          const cleanRemote = list
            .filter((entry) => typeof entry === "string" && entry.trim())
            .slice(0, MAX_VISIBLE_SUGGESTIONS);
          cacheOmniboxSuggestions(cacheKey, cleanRemote);
          if (selectedOmniboxIndex === -1 && addressForm.classList.contains("is-focused")) {
            openOrUpdateOmniboxPopup(trimmed, mergeOmniboxSuggestions(trimmed, cleanRemote));
          }
        })
        .catch(() => {
          // Ignore aborts or offline errors cleanly.
        });
    };

    if (immediate) {
      runFetch();
    } else {
      omniboxSuggestTimer = setTimeout(runFetch, SUGGEST_DEBOUNCE_MS);
    }
  }

  if (omniboxClearBtn) {
    omniboxClearBtn.addEventListener("mousedown", (e) => {
      e.preventDefault();
      e.stopPropagation();
    });
    omniboxClearBtn.addEventListener("click", () => {
      cancelPendingOmniboxSuggest();
      addressInput.value = "";
      draftOmniboxQuery = "";
      syncOmniboxClearBtn();
      addressInput.focus();
      requestOmniboxSuggestions("", { immediate: true });
    });
  }

  addressInput.addEventListener("input", () => {
    draftOmniboxQuery = addressInput.value;
    syncOmniboxClearBtn();
    requestOmniboxSuggestions(draftOmniboxQuery);
  });

  addressInput.addEventListener("blur", () => {
    cancelPendingOmniboxSuggest();
    post("omnibox:blur");
  });

  addressForm.addEventListener("submit", (e) => {
    e.preventDefault();
    cancelPendingOmniboxSuggest();
    const query = addressInput.value.trim();
    recordShellHistory(query);
    blurAddressBar(false);
    post("omnibox:close");
    post(`nav:go:${query}`);
  });

  addressInput.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (!omniboxOpen) {
        requestOmniboxSuggestions(addressInput.value, { immediate: true });
        return;
      }
      if (activeOmniboxItems.length > 0) {
        selectedOmniboxIndex = selectedOmniboxIndex + 1 >= activeOmniboxItems.length
          ? -1
          : selectedOmniboxIndex + 1;
        addressInput.value = selectedOmniboxIndex === -1
          ? draftOmniboxQuery
          : activeOmniboxItems[selectedOmniboxIndex].text;
        syncOmniboxClearBtn();
        post(`omnibox:select:${selectedOmniboxIndex}`);
      }
      return;
    }

    if (e.key === "ArrowUp" && omniboxOpen && activeOmniboxItems.length > 0) {
      e.preventDefault();
      selectedOmniboxIndex = selectedOmniboxIndex <= -1
        ? activeOmniboxItems.length - 1
        : selectedOmniboxIndex - 1;
      addressInput.value = selectedOmniboxIndex === -1
        ? draftOmniboxQuery
        : activeOmniboxItems[selectedOmniboxIndex].text;
      syncOmniboxClearBtn();
      post(`omnibox:select:${selectedOmniboxIndex}`);
      return;
    }

    if (
      (e.key === "Tab" || e.key === "ArrowRight")
      && selectedOmniboxIndex === -1
      && omniboxOpen
      && activeOmniboxItems.length > 0
      && addressInput.selectionStart === addressInput.value.length
      && addressInput.selectionEnd === addressInput.value.length
    ) {
      const top = activeOmniboxItems[0].text;
      const typed = addressInput.value.trim();
      if (typed && top.toLowerCase().startsWith(typed.toLowerCase()) && top.length > typed.length) {
        e.preventDefault();
        addressInput.value = top;
        draftOmniboxQuery = top;
        syncOmniboxClearBtn();
        requestOmniboxSuggestions(top, { immediate: true });
        return;
      }
    }

    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      cancelPendingOmniboxSuggest();
      if (omniboxOpen) {
        addressInput.value = draftOmniboxQuery;
        syncOmniboxClearBtn();
        closeOmniboxPopup({ notifyHost: true });
        return;
      }
      blurAddressBar(true);
      post("omnibox:close");
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
        focusAddressBar(true);
        return;
      }
      if (!e.shiftKey && lower === "b") {
        e.preventDefault();
        post("sidebar:toggle");
        return;
      }
      if (!e.shiftKey && key === ",") {
        e.preventDefault();
        post("settings:open");
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
        focusAddressBar(true);
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
  currentState.theme = initialTheme;
  document.documentElement.dataset.theme = initialTheme;
  renderAdblockControls();

  post("shell:ready");
})();
