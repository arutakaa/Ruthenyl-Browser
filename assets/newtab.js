(() => {
  const SHORTCUTS_KEY = "ruthenium_shortcuts_v2";
  const NTP_THEME_KEY = "ruthenium_ntp_theme_v2";
  const NTP_DARK_PRESET_KEY = "ruthenium_ntp_dark_preset_v1";
  const SEARCH_HISTORY_KEY = "ruthenium_search_history_v1";
  const MAX_SHORTCUTS = 10;
  const MAX_HISTORY = 10;
  const MAX_VISIBLE_SUGGESTIONS = 8;
  const THEMES = ["arctic", "midnight", "aurora"];
  const TILE_PALETTES = ["sky", "rose", "slate", "amber", "cyan"];
  const SUGGEST_DEBOUNCE_MS = 65;
  const MAX_SUGGEST_CACHE = 40;

  const ENGINES = [
    { key: "google", name: "Google", icon: "G", placeholder: "Cari di Google atau masukkan URL..." },
    { key: "duckduckgo", name: "DuckDuckGo", icon: "D", placeholder: "Cari secara privat di DuckDuckGo..." },
    { key: "bing", name: "Bing", icon: "B", placeholder: "Cari di Microsoft Bing..." },
  ];

  const RUTHENIUM_MINI_SVG = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"><polygon points="32,4 56,18 56,46 32,60 8,46 8,18" fill="#0f2347"/><polygon points="32,4 56,18 32,32 8,18" fill="#1e3f78"/><polygon points="32,32 56,18 56,46 32,60" fill="#09152b"/><path d="M23 20H35C39.5 20 42.5 22.5 42.5 26.5C42.5 29.8 40.3 32.1 37 32.8L43 43H36.8L31.5 33.4H28.2V43H23V20ZM28.2 24.4V29.2H34.2C36.3 29.2 37.5 28.2 37.5 26.8C37.5 25.3 36.3 24.4 34.2 24.4H28.2Z" fill="#7dd3fc"/></svg>`;
  const REMOVE_ICON_SVG = `<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><line x1="4" y1="4" x2="12" y2="12"></line><line x1="12" y1="4" x2="4" y2="12"></line></svg>`;
  const SUGGEST_SEARCH_SVG = `<svg class="suggestion-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="8"></circle><path d="m21 21-4.3-4.3"></path></svg>`;
  const SUGGEST_HISTORY_SVG = `<svg class="suggestion-icon suggestion-icon--history" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><polyline points="12 7 12 12 15.5 14"></polyline></svg>`;
  const SUGGEST_GLOBE_SVG = `<svg class="suggestion-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M3.6 9h16.8M3.6 15h16.8"></path><path d="M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"></path></svg>`;
  const FILL_ARROW_SVG = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M17 17 7 7"></path><path d="M7 17V7h10"></path></svg>`;

  const DEFAULT_SHORTCUTS = [
    { id: "sc-blog", name: "Ruthenium Notes", url: "http://browser.localhost/blog", tile: "sky", badge: "R" },
    { id: "sc-youtube", name: "YouTube", url: "https://www.youtube.com", tile: "rose", badge: "\u25B6" },
    { id: "sc-github", name: "GitHub", url: "https://github.com", tile: "slate", badge: "GH" },
    { id: "sc-gmail", name: "Gmail", url: "https://mail.google.com", tile: "amber", badge: "M" },
  ];

  const liveClockEl = document.getElementById("live-clock");
  const liveDateEl = document.getElementById("live-date");

  const searchBox = document.getElementById("search-box");
  const searchForm = document.getElementById("main-search-form");
  const searchInput = document.getElementById("main-search-input");
  const searchClearBtn = document.getElementById("btn-search-clear");
  const searchClearSep = document.getElementById("search-clear-sep");
  const searchErrorEl = document.getElementById("search-error");
  const searchDropdown = document.getElementById("search-dropdown");
  const suggestionsList = document.getElementById("search-suggestions");
  const historyFooter = document.getElementById("history-footer");
  const clearHistoryBtn = document.getElementById("btn-clear-history");
  const engineCycleBtn = document.getElementById("btn-engine-cycle");
  const engineIconEl = document.getElementById("engine-icon");
  const engineNameEl = document.getElementById("engine-name");

  const shortcutsGrid = document.getElementById("shortcuts-grid");
  const backdrop = document.getElementById("shortcut-backdrop");
  const shortcutForm = document.getElementById("shortcut-form");
  const shortcutNameInput = document.getElementById("shortcut-name");
  const shortcutUrlInput = document.getElementById("shortcut-url");
  const shortcutErrorEl = document.getElementById("shortcut-error");
  const cancelShortcutBtn = document.getElementById("btn-cancel-shortcut");

  const toastBox = document.getElementById("toast-box");

  let currentEngineIndex = 0;
  let toastTimer = null;

  const suggestCache = new Map();
  let suggestTimer = null;
  let suggestAbort = null;
  let suggestSeq = 0;
  let activeSuggestions = [];
  let selectedSuggestIndex = -1;
  let draftQuery = "";

  function setHidden(el, hidden) {
    if (!el) return;
    if (hidden) {
      el.setAttribute("hidden", "");
    } else {
      el.removeAttribute("hidden");
    }
  }

  function showToast(message) {
    toastBox.textContent = message;
    setHidden(toastBox, false);
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      setHidden(toastBox, true);
    }, 2400);
  }

  function readStorage(key) {
    try {
      return localStorage.getItem(key);
    } catch (_) {
      return null;
    }
  }

  function writeStorage(key, value) {
    try {
      localStorage.setItem(key, value);
    } catch (_) {
      // Ignore storage quota or restricted-context errors.
    }
  }

  function getSavedDarkPreset() {
    const preset = readStorage(NTP_DARK_PRESET_KEY);
    return preset === "aurora" ? "aurora" : "midnight";
  }

  function resolveInitialTheme() {
    const saved = readStorage(NTP_THEME_KEY);
    return THEMES.includes(saved) ? saved : "arctic";
  }

  function applyTheme(name, { persist = true, notifyHost = true } = {}) {
    const theme = THEMES.includes(name) ? name : "arctic";
    document.documentElement.dataset.theme = theme;

    if (persist) {
      writeStorage(NTP_THEME_KEY, theme);
      if (theme === "midnight" || theme === "aurora") {
        writeStorage(NTP_DARK_PRESET_KEY, theme);
      }
    }

    if (notifyHost && typeof window.__newtabSetTheme === "function") {
      window.__newtabSetTheme(theme);
    }
  }

  function updateEngineUi(engine) {
    engineIconEl.textContent = engine.icon;
    engineNameEl.textContent = engine.name;
    searchInput.placeholder = engine.placeholder;
    engineCycleBtn.setAttribute("aria-label", `Ganti mesin pencari: ${engine.name}`);
  }

  function setSearchEngineByKey(engineKey, { notifyHost = false, announce = false } = {}) {
    const idx = ENGINES.findIndex((item) => item.key === engineKey);
    currentEngineIndex = idx >= 0 ? idx : 0;
    const engine = ENGINES[currentEngineIndex];
    updateEngineUi(engine);

    if (notifyHost && typeof window.__newtabSetSearchEngine === "function") {
      window.__newtabSetSearchEngine(engine.key);
    }
    if (announce) {
      showToast(`Mesin pencari: ${engine.name}`);
    }
  }

  function loadSearchHistory() {
    try {
      const raw = readStorage(SEARCH_HISTORY_KEY);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed
        .filter((entry) => typeof entry === "string" && entry.trim())
        .slice(0, MAX_HISTORY);
    } catch (_) {
      return [];
    }
  }

  function saveSearchHistory(list) {
    writeStorage(SEARCH_HISTORY_KEY, JSON.stringify(list.slice(0, MAX_HISTORY)));
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

  function recordSearchHistory(raw) {
    const query = raw.trim();
    if (!query || looksLikeDomainOrUrl(query)) return;
    const lower = query.toLowerCase();
    const next = [query, ...loadSearchHistory().filter((item) => item.toLowerCase() !== lower)]
      .slice(0, MAX_HISTORY);
    saveSearchHistory(next);
  }

  function removeSearchHistoryItem(target) {
    const lower = target.trim().toLowerCase();
    const next = loadSearchHistory().filter((item) => item.toLowerCase() !== lower);
    saveSearchHistory(next);
  }

  function clearSearchHistory() {
    saveSearchHistory([]);
  }

  function syncClearButton() {
    const hasValue = searchInput.value.length > 0;
    setHidden(searchClearBtn, !hasValue);
    setHidden(searchClearSep, !hasValue);
  }

  function cancelPendingSuggest() {
    clearTimeout(suggestTimer);
    suggestTimer = null;
    if (suggestAbort) {
      suggestAbort.abort();
      suggestAbort = null;
    }
  }

  function closeSuggestions({ resetSelection = true } = {}) {
    searchBox.classList.remove("is-open");
    setHidden(searchDropdown, true);
    setHidden(historyFooter, true);
    suggestionsList.replaceChildren();
    searchInput.setAttribute("aria-expanded", "false");
    searchInput.removeAttribute("aria-activedescendant");
    activeSuggestions = [];
    if (resetSelection) {
      selectedSuggestIndex = -1;
    }
  }

  function updateSuggestionSelection() {
    const rows = suggestionsList.querySelectorAll(".suggestion-row");
    const buttons = suggestionsList.querySelectorAll(".suggestion-main-btn");
    rows.forEach((row, idx) => {
      const selected = idx === selectedSuggestIndex;
      row.classList.toggle("is-selected", selected);
      if (buttons[idx]) {
        buttons[idx].setAttribute("aria-selected", String(selected));
        if (selected) {
          searchInput.setAttribute("aria-activedescendant", buttons[idx].id);
        }
      }
    });
    if (selectedSuggestIndex < 0) {
      searchInput.removeAttribute("aria-activedescendant");
    }
  }

  function renderSuggestions(query, items) {
    suggestionsList.replaceChildren();
    activeSuggestions = items;
    selectedSuggestIndex = -1;
    searchInput.removeAttribute("aria-activedescendant");

    if (!items || items.length === 0) {
      closeSuggestions({ resetSelection: true });
      return;
    }

    const trimmedQuery = query.trim();
    const lowerQuery = trimmedQuery.toLowerCase();
    const showHistoryFooter = trimmedQuery.length === 0 && items.some((entry) => entry.kind === "history");

    items.forEach((item, idx) => {
      const li = document.createElement("li");
      li.className = "suggestion-row";
      li.setAttribute("role", "presentation");

      const mainBtn = document.createElement("button");
      mainBtn.type = "button";
      mainBtn.id = `suggest-opt-${idx}`;
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
        cancelPendingSuggest();
        recordSearchHistory(item.text);
        closeSuggestions();
        navigateActiveTab(item.text);
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
          removeSearchHistoryItem(item.text);
          searchInput.focus();
          requestSuggestions(draftQuery, { immediate: true });
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
          searchInput.value = item.text;
          draftQuery = item.text;
          syncClearButton();
          searchInput.focus();
          const end = searchInput.value.length;
          searchInput.setSelectionRange(end, end);
          requestSuggestions(item.text, { immediate: true });
        });
        li.appendChild(fillBtn);
      }

      suggestionsList.appendChild(li);
    });

    setHidden(historyFooter, !showHistoryFooter);
    setHidden(searchDropdown, false);
    searchBox.classList.add("is-open");
    searchInput.setAttribute("aria-expanded", "true");
  }

  function cacheSuggestions(cacheKey, items) {
    if (suggestCache.size >= MAX_SUGGEST_CACHE) {
      const oldest = suggestCache.keys().next().value;
      if (oldest !== undefined) {
        suggestCache.delete(oldest);
      }
    }
    suggestCache.set(cacheKey, items);
  }

  function mergeSuggestions(trimmed, remoteList) {
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
    const history = loadSearchHistory();
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
    const exact = suggestCache.get(lowerQuery);
    if (exact) return exact;
    for (let len = lowerQuery.length - 1; len >= 1; len -= 1) {
      const prefix = lowerQuery.slice(0, len);
      const cachedPrefix = suggestCache.get(prefix);
      if (cachedPrefix) {
        return cachedPrefix.filter((item) => item.toLowerCase().includes(lowerQuery));
      }
    }
    return [];
  }

  function requestSuggestions(rawInput, { immediate = false } = {}) {
    cancelPendingSuggest();
    const trimmed = rawInput.trim();

    if (!trimmed) {
      const recentItems = mergeSuggestions("", []);
      renderSuggestions("", recentItems);
      return;
    }

    if (isExplicitUrlInput(trimmed)) {
      renderSuggestions(trimmed, [{ text: trimmed, kind: "url" }]);
      return;
    }

    const cacheKey = trimmed.toLowerCase();
    const exactCached = suggestCache.get(cacheKey);
    const localItems = mergeSuggestions(trimmed, findBestCachedRemote(cacheKey));
    if (localItems.length > 0) {
      renderSuggestions(trimmed, localItems);
    } else {
      closeSuggestions();
    }

    if (exactCached) {
      return;
    }

    const seq = ++suggestSeq;
    const runFetch = () => {
      const controller = new AbortController();
      suggestAbort = controller;
      fetch(`/suggest?q=${encodeURIComponent(trimmed)}`, {
        signal: controller.signal,
        cache: "no-store",
      })
        .then((res) => (res.ok ? res.json() : []))
        .then((list) => {
          if (seq !== suggestSeq || draftQuery.trim() !== trimmed) {
            return;
          }
          if (!Array.isArray(list)) return;
          const cleanRemote = list
            .filter((entry) => typeof entry === "string" && entry.trim())
            .slice(0, MAX_VISIBLE_SUGGESTIONS);
          cacheSuggestions(cacheKey, cleanRemote);
          if (selectedSuggestIndex === -1 && document.activeElement === searchInput) {
            renderSuggestions(trimmed, mergeSuggestions(trimmed, cleanRemote));
          }
        })
        .catch(() => {
          // Ignore aborts or transient offline errors; search input remains usable.
        });
    };

    if (immediate) {
      runFetch();
    } else {
      suggestTimer = setTimeout(runFetch, SUGGEST_DEBOUNCE_MS);
    }
  }

  function tilePaletteFor(item, fallbackIndex) {
    if (item && TILE_PALETTES.includes(item.tile)) {
      return item.tile;
    }
    const seed = (item && (item.url || item.name)) || String(fallbackIndex);
    let hash = 0;
    for (let i = 0; i < seed.length; i += 1) {
      hash = (hash * 31 + seed.charCodeAt(i)) >>> 0;
    }
    return TILE_PALETTES[hash % TILE_PALETTES.length];
  }

  function loadShortcuts() {
    try {
      const raw = readStorage(SHORTCUTS_KEY);
      if (!raw) return DEFAULT_SHORTCUTS.map((s) => ({ ...s }));
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return DEFAULT_SHORTCUTS.map((s) => ({ ...s }));
      return parsed
        .filter((item) => item && typeof item.name === "string" && typeof item.url === "string")
        .slice(0, MAX_SHORTCUTS)
        .map((item, idx) => ({
          id: typeof item.id === "string" && item.id ? item.id : `sc-${idx}`,
          name: item.name,
          url: item.url,
          tile: tilePaletteFor(item, idx),
          badge: typeof item.badge === "string" ? item.badge : "",
        }));
    } catch (_) {
      return DEFAULT_SHORTCUTS.map((s) => ({ ...s }));
    }
  }

  function saveShortcuts(list) {
    writeStorage(SHORTCUTS_KEY, JSON.stringify(list.slice(0, MAX_SHORTCUTS)));
  }

  let shortcuts = loadShortcuts();

  function normalizeShortcutUrl(raw) {
    const trimmed = raw.trim();
    if (!trimmed) return "";
    if (/^https?:\/\//i.test(trimmed) || trimmed.startsWith("browser://")) {
      return trimmed;
    }
    const lower = trimmed.toLowerCase();
    if (
      lower === "localhost"
      || lower.startsWith("localhost:")
      || lower.startsWith("localhost/")
      || lower.startsWith("127.0.0.1")
      || lower.startsWith("[::1]")
    ) {
      return `http://${trimmed}`;
    }
    if (/^[a-z0-9-]+$/i.test(trimmed)) {
      return `https://${trimmed}.com`;
    }
    return `https://${trimmed}`;
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

  function faviconCandidatesForUrl(url) {
    try {
      const parsed = new URL(url);
      if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
        return [];
      }
      if (parsed.hostname.toLowerCase() === "browser.localhost") {
        return [];
      }
      if (isPublicHost(parsed.hostname)) {
        return [
          `https://www.google.com/s2/favicons?domain=${encodeURIComponent(parsed.hostname)}&sz=64`,
          `${parsed.origin}/favicon.ico`,
        ];
      }
      return [`${parsed.origin}/favicon.ico`];
    } catch (_) {
      return [];
    }
  }

  function populateShortcutTile(tile, item) {
    if (item.url.includes("browser.localhost/")) {
      tile.classList.add("shortcut-tile--brand");
      tile.innerHTML = RUTHENIUM_MINI_SVG;
      return;
    }

    const fallback = document.createElement("span");
    fallback.className = "shortcut-fallback";
    fallback.textContent = badgeForShortcut(item);
    tile.appendChild(fallback);

    const candidates = faviconCandidatesForUrl(item.url);
    if (candidates.length === 0) return;

    const img = document.createElement("img");
    img.className = "shortcut-favicon-img";
    img.alt = "";
    img.width = 26;
    img.height = 26;
    img.decoding = "async";
    img.referrerPolicy = "no-referrer";
    setHidden(img, true);

    let candidateIndex = 0;
    const tryNext = () => {
      if (candidateIndex >= candidates.length) {
        img.remove();
        return;
      }
      img.src = candidates[candidateIndex];
      candidateIndex += 1;
    };

    img.addEventListener("load", () => {
      if (img.naturalWidth > 0) {
        tile.classList.add("has-favicon");
        setHidden(img, false);
        setHidden(fallback, true);
      } else {
        tryNext();
      }
    });
    img.addEventListener("error", tryNext);

    tile.appendChild(img);
    tryNext();
  }

  function badgeForShortcut(item) {
    if (item && typeof item.badge === "string" && item.badge.trim()) {
      return item.badge.trim().slice(0, 3);
    }
    const url = (item && item.url) || "";
    const name = (item && item.name) || "";
    if (url.includes("browser.localhost/")) return "R";
    try {
      const host = new URL(url).hostname.replace(/^www\./i, "").toLowerCase();
      if (host === "youtube.com" || host === "youtu.be") return "\u25B6";
      if (host === "github.com") return "GH";
      if (host === "mail.google.com") return "M";
      if (host) return host.charAt(0).toUpperCase();
    } catch (_) {
      // Fall back to name initial below.
    }
    return name.trim().charAt(0).toUpperCase() || "W";
  }

  function openShortcutDialog() {
    closeSuggestions();
    shortcutNameInput.value = "";
    shortcutUrlInput.value = "";
    setHidden(shortcutErrorEl, true);
    setHidden(backdrop, false);
    shortcutNameInput.focus();
  }

  function closeShortcutDialog() {
    if (backdrop.hidden) return;
    setHidden(backdrop, true);
    const addBtn = document.getElementById("btn-add-shortcut");
    if (addBtn) addBtn.focus();
  }

  function renderShortcuts() {
    shortcutsGrid.replaceChildren();

    shortcuts.forEach((item, index) => {
      const wrapper = document.createElement("div");
      wrapper.className = "shortcut-item";

      const link = document.createElement("a");
      link.className = "shortcut-link";
      link.href = item.url;

      const tile = document.createElement("span");
      const palette = tilePaletteFor(item, index);
      tile.className = `shortcut-tile shortcut-tile--${palette}`;
      tile.setAttribute("aria-hidden", "true");
      populateShortcutTile(tile, item);

      const label = document.createElement("span");
      label.className = "shortcut-label";
      label.textContent = item.name;

      link.append(tile, label);

      const removeBtn = document.createElement("button");
      removeBtn.type = "button";
      removeBtn.className = "shortcut-remove-btn";
      removeBtn.setAttribute("aria-label", `Hapus pintasan ${item.name}`);
      removeBtn.title = "Hapus pintasan";
      removeBtn.innerHTML = REMOVE_ICON_SVG;
      removeBtn.addEventListener("click", (e) => {
        e.preventDefault();
        e.stopPropagation();
        shortcuts = shortcuts.filter((entry) => entry.id !== item.id);
        saveShortcuts(shortcuts);
        renderShortcuts();
      });

      wrapper.append(link, removeBtn);
      shortcutsGrid.appendChild(wrapper);
    });

    if (shortcuts.length < MAX_SHORTCUTS) {
      const addBtn = document.createElement("button");
      addBtn.type = "button";
      addBtn.id = "btn-add-shortcut";
      addBtn.className = "shortcut-add-btn";

      const tile = document.createElement("span");
      tile.className = "shortcut-tile shortcut-tile--add";
      tile.setAttribute("aria-hidden", "true");
      tile.textContent = "+";

      const label = document.createElement("span");
      label.className = "shortcut-label";
      label.textContent = "Tambah";

      addBtn.append(tile, label);
      addBtn.addEventListener("click", openShortcutDialog);
      shortcutsGrid.appendChild(addBtn);
    }
  }

  function updateClock() {
    const now = new Date();
    const hours = String(now.getHours()).padStart(2, "0");
    const minutes = String(now.getMinutes()).padStart(2, "0");
    liveClockEl.textContent = `${hours}:${minutes}`;

    const formattedDate = now.toLocaleDateString("id-ID", {
      weekday: "short",
      day: "numeric",
      month: "short",
    });
    liveDateEl.textContent = formattedDate;
  }

  function navigateActiveTab(query) {
    if (typeof window.__newtabNavigate === "function") {
      window.__newtabNavigate(query);
    } else {
      window.location.href = query;
    }
  }

  searchForm.addEventListener("submit", (e) => {
    e.preventDefault();
    cancelPendingSuggest();
    const raw = searchInput.value.trim();
    if (!raw) {
      closeSuggestions();
      searchErrorEl.textContent = "Ketik kata kunci atau alamat URL terlebih dahulu.";
      setHidden(searchErrorEl, false);
      searchInput.focus();
      return;
    }
    setHidden(searchErrorEl, true);
    recordSearchHistory(raw);
    closeSuggestions();
    navigateActiveTab(raw);
  });

  searchClearBtn.addEventListener("mousedown", (e) => {
    e.preventDefault();
  });

  searchClearBtn.addEventListener("click", () => {
    cancelPendingSuggest();
    searchInput.value = "";
    draftQuery = "";
    syncClearButton();
    setHidden(searchErrorEl, true);
    searchInput.focus();
    requestSuggestions("", { immediate: true });
  });

  clearHistoryBtn.addEventListener("mousedown", (e) => {
    e.preventDefault();
  });

  clearHistoryBtn.addEventListener("click", () => {
    clearSearchHistory();
    closeSuggestions();
    searchInput.focus();
  });

  searchInput.addEventListener("input", () => {
    if (!searchErrorEl.hidden) {
      setHidden(searchErrorEl, true);
    }
    draftQuery = searchInput.value;
    syncClearButton();
    requestSuggestions(draftQuery);
  });

  searchInput.addEventListener("focus", () => {
    if (searchDropdown.hidden && backdrop.hidden) {
      draftQuery = searchInput.value;
      syncClearButton();
      requestSuggestions(draftQuery, { immediate: true });
    }
  });

  searchInput.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown" && activeSuggestions.length > 0) {
      e.preventDefault();
      selectedSuggestIndex = selectedSuggestIndex + 1 >= activeSuggestions.length
        ? -1
        : selectedSuggestIndex + 1;
      updateSuggestionSelection();
      searchInput.value = selectedSuggestIndex === -1
        ? draftQuery
        : activeSuggestions[selectedSuggestIndex].text;
      syncClearButton();
      return;
    }

    if (e.key === "ArrowUp" && activeSuggestions.length > 0) {
      e.preventDefault();
      selectedSuggestIndex = selectedSuggestIndex <= -1
        ? activeSuggestions.length - 1
        : selectedSuggestIndex - 1;
      updateSuggestionSelection();
      searchInput.value = selectedSuggestIndex === -1
        ? draftQuery
        : activeSuggestions[selectedSuggestIndex].text;
      syncClearButton();
      return;
    }

    if (
      (e.key === "Tab" || e.key === "ArrowRight")
      && selectedSuggestIndex === -1
      && !searchDropdown.hidden
      && activeSuggestions.length > 0
      && searchInput.selectionStart === searchInput.value.length
      && searchInput.selectionEnd === searchInput.value.length
    ) {
      const top = activeSuggestions[0].text;
      const typed = searchInput.value.trim();
      if (typed && top.toLowerCase().startsWith(typed.toLowerCase()) && top.length > typed.length) {
        e.preventDefault();
        searchInput.value = top;
        draftQuery = top;
        syncClearButton();
        requestSuggestions(top, { immediate: true });
        return;
      }
    }

    if (e.key === "Escape" && !searchDropdown.hidden) {
      e.preventDefault();
      e.stopPropagation();
      cancelPendingSuggest();
      searchInput.value = draftQuery;
      syncClearButton();
      closeSuggestions();
    }
  });

  engineCycleBtn.addEventListener("click", () => {
    const nextIdx = (currentEngineIndex + 1) % ENGINES.length;
    setSearchEngineByKey(ENGINES[nextIdx].key, { notifyHost: true, announce: true });
  });

  shortcutForm.addEventListener("submit", (e) => {
    e.preventDefault();
    const name = shortcutNameInput.value.trim().slice(0, 32);
    const normalizedUrl = normalizeShortcutUrl(shortcutUrlInput.value);
    if (!name || !normalizedUrl) {
      shortcutErrorEl.textContent = "Isi nama dan URL pintasan terlebih dahulu.";
      setHidden(shortcutErrorEl, false);
      return;
    }
    try {
      new URL(normalizedUrl);
    } catch (_) {
      shortcutErrorEl.textContent = "Format URL tidak valid.";
      setHidden(shortcutErrorEl, false);
      return;
    }

    shortcuts.push({
      id: `sc-${Date.now()}-${Math.random().toString(36).slice(2, 5)}`,
      name,
      url: normalizedUrl,
      tile: "cyan",
      badge: "",
    });
    saveShortcuts(shortcuts);
    renderShortcuts();
    closeShortcutDialog();
    showToast(`Pintasan "${name}" berhasil ditambahkan`);
  });

  cancelShortcutBtn.addEventListener("click", closeShortcutDialog);

  backdrop.addEventListener("mousedown", (e) => {
    if (e.target === backdrop) {
      closeShortcutDialog();
    }
  });

  document.addEventListener("mousedown", (e) => {
    if (!searchDropdown.hidden && !searchBox.contains(e.target)) {
      closeSuggestions();
    }
  });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !backdrop.hidden) {
      e.preventDefault();
      closeShortcutDialog();
      return;
    }

    const activeTag = document.activeElement ? document.activeElement.tagName.toLowerCase() : "";
    const inEditable = activeTag === "input" || activeTag === "textarea";
    if (
      e.key === "/"
      && !e.ctrlKey
      && !e.altKey
      && !e.metaKey
      && !inEditable
      && backdrop.hidden
    ) {
      e.preventDefault();
      searchInput.focus();
    }
  });

  window.addEventListener("storage", (e) => {
    if (e.key === SHORTCUTS_KEY) {
      shortcuts = loadShortcuts();
      renderShortcuts();
    } else if (e.key === NTP_THEME_KEY && THEMES.includes(e.newValue)) {
      applyTheme(e.newValue, { persist: false, notifyHost: false });
    } else if (e.key === SEARCH_HISTORY_KEY && document.activeElement === searchInput && !searchDropdown.hidden) {
      requestSuggestions(draftQuery, { immediate: true });
    }
  });

  window.__syncSearchEngine = (engine) => {
    setSearchEngineByKey(engine, { notifyHost: false, announce: false });
  };

  window.__syncTheme = (themeOrDark) => {
    let nextTheme = "arctic";
    if (typeof themeOrDark === "string") {
      if (THEMES.includes(themeOrDark)) {
        nextTheme = themeOrDark;
      } else if (themeOrDark === "light") {
        nextTheme = "arctic";
      } else if (themeOrDark === "dark") {
        nextTheme = getSavedDarkPreset();
      }
    } else {
      nextTheme = themeOrDark ? getSavedDarkPreset() : "arctic";
    }
    applyTheme(nextTheme, { persist: true, notifyHost: false });
  };

  applyTheme(resolveInitialTheme(), { persist: false, notifyHost: false });
  setSearchEngineByKey("google", { notifyHost: false, announce: false });
  syncClearButton();
  renderShortcuts();
  updateClock();
  setInterval(updateClock, 15000);
})();
