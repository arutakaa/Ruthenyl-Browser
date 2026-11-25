(() => {
  const THEME_KEY = "ruthenium_ntp_theme_v2";
  const VALID_THEMES = new Set(["arctic", "midnight", "aurora"]);
  const VALID_ENGINES = new Set(["google", "duckduckgo", "bing"]);

  const step1El = document.getElementById("welcome-step-1");
  const step2El = document.getElementById("welcome-step-2");
  const stepLabelEl = document.getElementById("welcome-step-label");
  const seg1El = document.getElementById("progress-seg-1");
  const seg2El = document.getElementById("progress-seg-2");

  const defaultBoxEl = document.getElementById("welcome-default-box");
  const defaultDescEl = document.getElementById("welcome-default-desc");
  const iconGlobeEl = document.getElementById("icon-default-globe");
  const iconCheckEl = document.getElementById("icon-default-check");
  const btnDefaultEl = document.getElementById("btn-welcome-default");

  const btnSkipTop = document.getElementById("btn-welcome-skip-top");
  const btnStep1Skip = document.getElementById("btn-step1-skip");
  const btnStep1Next = document.getElementById("btn-step1-next");
  const btnStep2Back = document.getElementById("btn-step2-back");
  const btnStep2Skip = document.getElementById("btn-step2-skip");
  const btnStep2Finish = document.getElementById("btn-step2-finish");

  const themeCards = Array.from(document.querySelectorAll("[data-welcome-theme]"));
  const engineCards = Array.from(document.querySelectorAll("[data-welcome-engine]"));

  function setHidden(el, hidden) {
    if (!el) return;
    if (hidden) {
      el.setAttribute("hidden", "");
    } else {
      el.removeAttribute("hidden");
    }
  }

  function normalizeTheme(raw) {
    if (raw === "dark") return "midnight";
    if (raw === "light") return "arctic";
    return VALID_THEMES.has(raw) ? raw : "arctic";
  }

  function applyThemeVisual(themeName) {
    const mode = normalizeTheme(themeName);
    document.documentElement.dataset.theme = mode;
    try {
      localStorage.setItem(THEME_KEY, mode);
    } catch (_) {}

    for (const card of themeCards) {
      const active = card.dataset.welcomeTheme === mode;
      card.classList.toggle("is-selected", active);
      card.setAttribute("aria-pressed", String(active));
    }
  }

  function applyEngineVisual(engineKey) {
    const engine = VALID_ENGINES.has(engineKey) ? engineKey : "google";
    for (const card of engineCards) {
      const active = card.dataset.welcomeEngine === engine;
      card.classList.toggle("is-selected", active);
      card.setAttribute("aria-pressed", String(active));
    }
  }

  function applyDefaultBrowserVisual(isDefault) {
    const verified = Boolean(isDefault);
    if (defaultBoxEl) {
      defaultBoxEl.classList.toggle("is-default", verified);
    }
    setHidden(iconGlobeEl, verified);
    setHidden(iconCheckEl, !verified);
    if (defaultDescEl) {
      defaultDescEl.textContent = verified
        ? "Ruthenium sudah menjadi browser default"
        : "Buka tautan dari aplikasi lain langsung di Ruthenium";
    }
    if (btnDefaultEl) {
      btnDefaultEl.classList.toggle("is-default", verified);
      btnDefaultEl.textContent = verified ? "Sudah Default" : "Jadikan Default";
    }
  }

  function showStep(stepNumber) {
    const isStep1 = stepNumber === 1;
    setHidden(step1El, !isStep1);
    setHidden(step2El, isStep1);

    if (stepLabelEl) {
      stepLabelEl.textContent = isStep1 ? "Langkah 1 dari 2" : "Langkah 2 dari 2";
    }
    if (seg1El) seg1El.classList.toggle("is-active", true);
    if (seg2El) seg2El.classList.toggle("is-active", !isStep1);
  }

  function finishOnboarding() {
    if (typeof window.__welcomeComplete === "function") {
      window.__welcomeComplete();
    } else {
      location.href = "http://browser.localhost/newtab";
    }
  }

  window.__syncTheme = (theme) => {
    applyThemeVisual(theme);
  };

  window.__syncSearchEngine = (engine) => {
    applyEngineVisual(engine);
  };

  window.__syncDefaultBrowser = (isDefault) => {
    applyDefaultBrowserVisual(isDefault);
  };

  if (btnStep1Next) {
    btnStep1Next.addEventListener("click", () => showStep(2));
  }
  if (btnStep2Back) {
    btnStep2Back.addEventListener("click", () => showStep(1));
  }
  if (btnDefaultEl) {
    btnDefaultEl.addEventListener("click", () => {
      if (typeof window.__welcomeSetDefaultBrowser === "function") {
        window.__welcomeSetDefaultBrowser();
      }
    });
  }

  for (const btn of [btnSkipTop, btnStep1Skip, btnStep2Skip, btnStep2Finish]) {
    if (btn) {
      btn.addEventListener("click", finishOnboarding);
    }
  }

  for (const card of themeCards) {
    card.addEventListener("click", () => {
      const mode = normalizeTheme(card.dataset.welcomeTheme);
      applyThemeVisual(mode);
      if (typeof window.__newtabSetTheme === "function") {
        window.__newtabSetTheme(mode);
      }
    });
  }

  for (const card of engineCards) {
    card.addEventListener("click", () => {
      const engine = card.dataset.welcomeEngine || "google";
      applyEngineVisual(engine);
      if (typeof window.__newtabSetSearchEngine === "function") {
        window.__newtabSetSearchEngine(engine);
      }
    });
  }

  try {
    applyThemeVisual(localStorage.getItem(THEME_KEY) || "arctic");
  } catch (_) {
    applyThemeVisual("arctic");
  }
  applyEngineVisual("google");
  applyDefaultBrowserVisual(false);
  showStep(1);
})();
