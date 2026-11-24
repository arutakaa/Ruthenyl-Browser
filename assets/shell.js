(() => {
  const post = (msg) => {
    if (window.ipc && typeof window.ipc.postMessage === "function") {
      window.ipc.postMessage(msg);
    }
  };

  const GLOBE_SVG = `<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="8" r="5.5"></circle><line x1="2.5" y1="8" x2="13.5" y2="8"></line><path d="M8 2.5c1.6 1.5 2.4 3.4 2.4 5.5S9.6 12 8 13.5C6.4 12 5.6 10.1 5.6 8S6.4 4 8 2.5Z"></path></svg>`;
  const SPINNER_SVG = `<path d="M7 1.5a5.5 5.5 0 1 1-4.6 2.5"></path>`;
  const CLOSE_SVG = `<svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><line x1="3" y1="3" x2="9" y2="9"></line><line x1="9" y1="3" x2="3" y2="9"></line></svg>`;
  const PREFS_STORAGE_KEY = "ruthenyl_chrome_prefs_v1";
  const ENGINE_LABELS = {
    google: "Google",
    duckduckgo: "DuckDuckGo",
    bing: "Bing",
  };

  function engineDisplayName(key) {
    return ENGINE_LABELS[key] || "Google";
  }

  function searchPlaceholderFor(engineKey) {
    return `Cari di ${engineDisplayName(engineKey)} atau masukkan URL`;
  }

  function loadChromePrefs() {
    try {
      const raw = localStorage.getItem(PREFS_STORAGE_KEY);
      if (!raw) return null;
      const parsed = JSON.parse(raw);
      if (!parsed || typeof parsed !== "object") return null;
      const profileName = typeof parsed.profileName === "string"
        ? parsed.profileName.trim().slice(0, 32)
        : "";
      const searchEngine = typeof parsed.searchEngine === "string" && ENGINE_LABELS[parsed.searchEngine]
        ? parsed.searchEngine
        : "google";
      return { profileName, searchEngine };
    } catch (_) {
      return null;
    }
  }

  function saveChromePrefs(profileName, searchEngine) {
    try {
      localStorage.setItem(
        PREFS_STORAGE_KEY,
        JSON.stringify({
          profileName: (profileName || "Budi").trim().slice(0, 32) || "Budi",
          searchEngine: ENGINE_LABELS[searchEngine] ? searchEngine : "google",
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
  const bookmarksHeading = document.getElementById("bookmarks-heading");

  const profilePanel = document.getElementById("panel-profile");
  const profileCloseBtn = document.getElementById("profile-close-btn");
  const profilePanelAvatar = document.getElementById("profile-panel-avatar");
  const profilePanelName = document.getElementById("profile-panel-name");
  const profileForm = document.getElementById("profile-form");
  const profileNameInput = document.getElementById("profile-name-input");
  const engineSelect = document.getElementById("engine-select");
  const profileStatus = document.getElementById("profile-status");
  const profileNewTabBtn = document.getElementById("profile-new-tab-btn");

  const tabList = document.getElementById("tab-list");
  const btnSidebar = document.getElementById("btn-sidebar");
  const btnNavDrawer = document.getElementById("btn-nav-drawer");
  const btnBookmarks = document.getElementById("btn-bookmarks");
  const btnProfile = document.getElementById("btn-profile");
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
    title: "Beranda",
    security: "internal",
    canGoBack: false,
    canGoForward: false,
    isLoading: false,
    sidebarWidth: 0,
    isCompact: true,
    isMaximized: false,
    windowFocused: true,
    isDark: false,
    profileName: "Budi",
    searchEngine: "google",
  };

  let loadingTimer = null;
  let shareCheckTimer = null;
  let profileStatusTimer = null;
  let pendingBookmarksFocus = false;
  let pendingProfileFocus = false;
  let activeSidebarPanel = "workspace";
  const tabElements = new Map();

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
    securityBadge.classList.toggle("is-insecure", isInsecure);
  }

  function renderFaviconFallback(slot, origin) {
    slot.dataset.faviconUrl = "";
    if (origin && !origin.startsWith("browser://")) {
      const initial = origin.replace(/^www\./i, "").charAt(0).toUpperCase();
      slot.textContent = initial || "W";
    } else {
      slot.innerHTML = GLOBE_SVG;
    }
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

  function renderSidebarTabs(tabs) {
    sidebarTabList.replaceChildren();
    for (const tab of tabs) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = tab.isActive ? "sidebar-item is-active" : "sidebar-item";
      btn.textContent = tab.title || tab.origin || "Beranda";
      btn.addEventListener("click", () => post(`tab:select:${tab.id}`));
      sidebarTabList.appendChild(btn);
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

      const displayTitle = tab.title || tab.origin || "Beranda";
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

      const wantedIcon = tab.faviconUrl || "";
      if (wantedIcon && entry.favicon.dataset.faviconUrl !== wantedIcon) {
        entry.favicon.dataset.faviconUrl = wantedIcon;
        entry.favicon.textContent = "";
        const img = document.createElement("img");
        img.className = "tab-favicon-img";
        img.alt = "";
        img.width = 14;
        img.height = 14;
        img.src = wantedIcon;
        img.addEventListener("error", () => {
          renderFaviconFallback(entry.favicon, tab.origin);
        }, { once: true });
        entry.favicon.appendChild(img);
      } else if (!wantedIcon) {
        renderFaviconFallback(entry.favicon, tab.origin);
      }

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

  function renderProfileControls(sidebarOpen) {
    const displayName = (currentState.profileName || "Budi").trim() || "Budi";
    const initial = displayName.charAt(0).toUpperCase() || "B";
    btnProfile.textContent = initial;
    btnProfile.setAttribute("title", `Profil Ruthenyl (${displayName})`);
    btnProfile.setAttribute("aria-label", `Profil (${displayName})`);

    profilePanelAvatar.textContent = initial;
    profilePanelName.textContent = displayName;
    if (document.activeElement !== profileNameInput) {
      profileNameInput.value = displayName;
    }

    const activeEngine = ENGINE_LABELS[currentState.searchEngine]
      ? currentState.searchEngine
      : "google";
    if (document.activeElement !== engineSelect) {
      engineSelect.value = activeEngine;
    }

    const isProfileView = sidebarOpen && activeSidebarPanel === "profile";
    const isBookmarksView = sidebarOpen && activeSidebarPanel === "bookmarks";
    const isWorkspaceView = sidebarOpen && activeSidebarPanel === "workspace";

    setHidden(sidebarDefaultView, isProfileView);
    setHidden(profilePanel, !isProfileView);

    btnProfile.setAttribute("aria-expanded", String(isProfileView));
    btnBookmarks.setAttribute("aria-expanded", String(isBookmarksView));
    btnSidebar.setAttribute("aria-expanded", String(sidebarOpen && !isProfileView));
    btnNavDrawer.setAttribute("aria-expanded", String(isWorkspaceView));
  }

  function openSidebarPanel(panel) {
    const sidebarOpen = currentState.sidebarWidth > 0 && !currentState.isCompact;
    if (sidebarOpen && activeSidebarPanel === panel) {
      post("sidebar:close");
      return;
    }

    activeSidebarPanel = panel;
    pendingBookmarksFocus = panel === "bookmarks";
    pendingProfileFocus = panel === "profile";

    if (sidebarOpen) {
      renderProfileControls(true);
      if (pendingBookmarksFocus) {
        pendingBookmarksFocus = false;
        bookmarksHeading.focus();
      } else if (pendingProfileFocus) {
        pendingProfileFocus = false;
        profileNameInput.focus();
        profileNameInput.select();
      }
    } else {
      post("sidebar:open");
    }
  }

  window.__shellSync = (next) => {
    const prevRawUrl = currentState.rawUrl;
    const prevName = currentState.profileName;
    const prevEngine = currentState.searchEngine;
    currentState = { ...currentState, ...next };

    if (prevName !== currentState.profileName || prevEngine !== currentState.searchEngine) {
      saveChromePrefs(currentState.profileName, currentState.searchEngine);
    }

    document.documentElement.dataset.windowFocused = String(Boolean(currentState.windowFocused));
    shellRoot.style.setProperty("--sidebar-width", `${currentState.sidebarWidth}px`);

    const sidebarOpen = currentState.sidebarWidth > 0 && !currentState.isCompact;
    setHidden(sidebar, !sidebarOpen);
    renderProfileControls(sidebarOpen);

    if (sidebarOpen && pendingBookmarksFocus) {
      pendingBookmarksFocus = false;
      bookmarksHeading.focus();
    } else if (sidebarOpen && pendingProfileFocus) {
      pendingProfileFocus = false;
      profileNameInput.focus();
      profileNameInput.select();
    } else if (!sidebarOpen) {
      pendingBookmarksFocus = false;
      pendingProfileFocus = false;
    }

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
  btnNavDrawer.addEventListener("click", () => openSidebarPanel("workspace"));
  btnBookmarks.addEventListener("click", () => openSidebarPanel("bookmarks"));
  btnProfile.addEventListener("click", () => openSidebarPanel("profile"));
  profileCloseBtn.addEventListener("click", () => post("sidebar:close"));

  profileForm.addEventListener("submit", (e) => {
    e.preventDefault();
    const nextName = profileNameInput.value.trim().slice(0, 32) || "Budi";
    const nextEngine = ENGINE_LABELS[engineSelect.value] ? engineSelect.value : "google";
    profileNameInput.value = nextName;
    saveChromePrefs(nextName, nextEngine);
    post(`prefs:engine:${nextEngine}`);
    post(`prefs:name:${nextName}`);
    profileStatus.textContent = "Tersimpan";
    clearTimeout(profileStatusTimer);
    profileStatusTimer = setTimeout(() => {
      profileStatus.textContent = "";
    }, 2000);
  });

  engineSelect.addEventListener("change", () => {
    const nextName = (profileNameInput.value.trim().slice(0, 32) || currentState.profileName || "Budi");
    const nextEngine = ENGINE_LABELS[engineSelect.value] ? engineSelect.value : "google";
    saveChromePrefs(nextName, nextEngine);
    post(`prefs:engine:${nextEngine}`);
    profileStatus.textContent = "Mesin pencari aktif";
    clearTimeout(profileStatusTimer);
    profileStatusTimer = setTimeout(() => {
      profileStatus.textContent = "";
    }, 1800);
  });

  sidebar.querySelectorAll("[data-nav-url]").forEach((btn) => {
    btn.addEventListener("click", () => {
      post(`nav:go:${btn.dataset.navUrl}`);
    });
  });
  sidebarNewTabBtn.addEventListener("click", () => post("tab:new"));
  profileNewTabBtn.addEventListener("click", () => post("tab:new"));

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
    if (e.target.closest("#btn-reload")) return;
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

  addressInput.addEventListener("blur", () => {
    blurAddressBar(false);
  });

  addressForm.addEventListener("submit", (e) => {
    e.preventDefault();
    const query = addressInput.value.trim();
    blurAddressBar(false);
    post(`nav:go:${query}`);
  });

  addressInput.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
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
    if (e.target.closest("[data-drag-region='true']")) {
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
    if (x >= w - 138 && y < 40) return null;

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
  if (savedPrefs) {
    if (savedPrefs.searchEngine) {
      currentState.searchEngine = savedPrefs.searchEngine;
      post(`prefs:engine:${savedPrefs.searchEngine}`);
    }
    if (savedPrefs.profileName) {
      currentState.profileName = savedPrefs.profileName;
      post(`prefs:name:${savedPrefs.profileName}`);
    }
  }

  post("shell:ready");
})();
