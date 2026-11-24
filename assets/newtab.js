(() => {
  const STORAGE_KEY = "ruthenyl_home_v1";

  const DEFAULT_TODOS = [
    { id: "todo-1", text: "Review wireframe Ruthenyl", done: false },
    { id: "todo-2", text: "Sync komponen UI di Figma", done: false },
    { id: "todo-3", text: "Kirim update ke tim GitHub", done: false },
  ];

  const DAYS_ID = ["Minggu", "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu"];
  const MONTHS_ID = ["Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des"];

  function createDefaultState() {
    return {
      userName: "Budi",
      isCelsius: true,
      todos: DEFAULT_TODOS.map((item) => ({ ...item })),
    };
  }

  let state = loadState();
  let toastTimer = null;

  const searchForm = document.getElementById("main-search-form");
  const searchInput = document.getElementById("main-search-input");
  const searchErrorEl = document.getElementById("search-error");

  const weatherCardBtn = document.getElementById("weather-card-btn");
  const weatherTempEl = document.getElementById("weather-temp");
  const calendarDayEl = document.getElementById("calendar-day");
  const calendarDateEl = document.getElementById("calendar-date");

  const todoCardBtn = document.getElementById("todo-card-btn");
  const todoCountLabelEl = document.getElementById("todo-count-label");
  const todoPopoverEl = document.getElementById("todo-popover");
  const closeTodoBtn = document.getElementById("close-todo-btn");
  const todoAddForm = document.getElementById("todo-add-form");
  const todoNewInput = document.getElementById("todo-new-input");
  const todoListEl = document.getElementById("todo-list");
  const todoEmptyEl = document.getElementById("todo-empty");

  const greetingTextEl = document.getElementById("greeting-text");
  const editNameBtn = document.getElementById("edit-name-btn");
  const nameEditForm = document.getElementById("name-edit-form");
  const nameEditInput = document.getElementById("name-edit-input");
  const hourHandEl = document.getElementById("hour-hand");
  const minuteHandEl = document.getElementById("minute-hand");
  const digitalClockEl = document.getElementById("digital-clock");

  const toastBannerEl = document.getElementById("toast-banner");
  const toastMessageEl = document.getElementById("toast-message");

  function setHidden(el, hidden) {
    if (!el) return;
    if (hidden) {
      el.setAttribute("hidden", "");
    } else {
      el.removeAttribute("hidden");
    }
  }

  function loadState() {
    const fallback = createDefaultState();
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return fallback;
      const parsed = JSON.parse(raw);
      if (!parsed || typeof parsed !== "object") return fallback;
      return {
        userName: typeof parsed.userName === "string" ? parsed.userName : fallback.userName,
        isCelsius: typeof parsed.isCelsius === "boolean" ? parsed.isCelsius : true,
        todos: Array.isArray(parsed.todos)
          ? parsed.todos.filter((t) => t && typeof t.text === "string")
          : fallback.todos,
      };
    } catch (_) {
      return fallback;
    }
  }

  function saveState() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch (_) {
      // Ignore storage quota errors so controls keep working.
    }
  }

  function showToast(message) {
    toastMessageEl.textContent = message;
    setHidden(toastBannerEl, false);
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      setHidden(toastBannerEl, true);
    }, 2400);
  }

  function navigateActiveTab(query) {
    if (typeof window.__newtabNavigate === "function") {
      window.__newtabNavigate(query);
    } else {
      window.location.href = query;
    }
  }

  function renderWeather() {
    weatherTempEl.textContent = state.isCelsius ? "20°C" : "68°F";
  }

  function renderClockGreetingCalendar() {
    const now = new Date();
    const hours = now.getHours();
    const minutes = now.getMinutes();

    calendarDayEl.textContent = DAYS_ID[now.getDay()];
    calendarDateEl.textContent = `${now.getDate()} ${MONTHS_ID[now.getMonth()]}`;

    let greet = "Selamat Malam";
    if (hours >= 4 && hours < 11) greet = "Selamat Pagi";
    else if (hours >= 11 && hours < 15) greet = "Selamat Siang";
    else if (hours >= 15 && hours < 18) greet = "Selamat Sore";

    const cleanName = state.userName.trim();
    greetingTextEl.textContent = cleanName ? `${greet}, ${cleanName}` : greet;

    digitalClockEl.textContent = `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
    digitalClockEl.setAttribute("datetime", now.toISOString());

    const minuteDeg = minutes * 6;
    const hourDeg = (hours % 12) * 30 + minutes * 0.5;
    hourHandEl.setAttribute("transform", `rotate(${hourDeg} 50 50)`);
    minuteHandEl.setAttribute("transform", `rotate(${minuteDeg} 50 50)`);
  }

  function renderTodos() {
    const remaining = state.todos.filter((t) => !t.done).length;
    todoCountLabelEl.textContent = `${remaining} Todo`;

    todoListEl.replaceChildren();
    const hasTodos = state.todos.length > 0;
    setHidden(todoListEl, !hasTodos);
    setHidden(todoEmptyEl, hasTodos);
    if (!hasTodos) return;

    for (const item of state.todos) {
      const li = document.createElement("li");
      li.className = item.done ? "todo-item is-done" : "todo-item";

      const label = document.createElement("label");
      label.className = "todo-item-label";

      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.className = "todo-checkbox";
      checkbox.checked = Boolean(item.done);
      checkbox.addEventListener("change", () => {
        item.done = checkbox.checked;
        saveState();
        renderTodos();
      });

      const textSpan = document.createElement("span");
      textSpan.className = "todo-item-text";
      textSpan.textContent = item.text;

      label.append(checkbox, textSpan);

      const delBtn = document.createElement("button");
      delBtn.type = "button";
      delBtn.className = "todo-del-btn";
      delBtn.setAttribute("aria-label", `Hapus tugas: ${item.text}`);
      delBtn.title = "Hapus tugas";
      delBtn.innerHTML = `<svg class="icon-14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><line x1="4" y1="4" x2="12" y2="12"></line><line x1="12" y1="4" x2="4" y2="12"></line></svg>`;
      delBtn.addEventListener("click", () => {
        state.todos = state.todos.filter((t) => t.id !== item.id);
        saveState();
        renderTodos();
      });

      li.append(label, delBtn);
      todoListEl.appendChild(li);
    }
  }

  function setTodoPopoverOpen(open) {
    setHidden(todoPopoverEl, !open);
    todoCardBtn.setAttribute("aria-expanded", String(open));
    if (open) {
      todoNewInput.focus();
    } else {
      todoCardBtn.focus();
    }
  }

  searchForm.addEventListener("submit", (e) => {
    e.preventDefault();
    const raw = searchInput.value.trim();
    if (!raw) {
      searchErrorEl.textContent = "Ketik kata kunci atau alamat URL terlebih dahulu.";
      setHidden(searchErrorEl, false);
      searchInput.focus();
      return;
    }
    setHidden(searchErrorEl, true);
    navigateActiveTab(raw);
  });

  searchInput.addEventListener("input", () => {
    if (!searchErrorEl.hidden) {
      setHidden(searchErrorEl, true);
    }
  });

  weatherCardBtn.addEventListener("click", () => {
    state.isCelsius = !state.isCelsius;
    saveState();
    renderWeather();
    showToast(`Suhu diubah ke ${state.isCelsius ? "Celsius" : "Fahrenheit"}`);
  });

  todoCardBtn.addEventListener("click", () => {
    const isOpen = !todoPopoverEl.hasAttribute("hidden");
    setTodoPopoverOpen(!isOpen);
  });

  closeTodoBtn.addEventListener("click", () => {
    setTodoPopoverOpen(false);
  });

  todoAddForm.addEventListener("submit", (e) => {
    e.preventDefault();
    const text = todoNewInput.value.trim();
    if (!text) return;
    state.todos.unshift({
      id: `todo-${Date.now()}-${Math.random().toString(36).slice(2, 5)}`,
      text,
      done: false,
    });
    todoNewInput.value = "";
    saveState();
    renderTodos();
  });

  editNameBtn.addEventListener("click", () => {
    nameEditInput.value = state.userName;
    setHidden(nameEditForm, false);
    nameEditInput.focus();
    nameEditInput.select();
  });

  nameEditForm.addEventListener("submit", (e) => {
    e.preventDefault();
    const clean = nameEditInput.value.trim().slice(0, 32) || "Budi";
    state.userName = clean;
    saveState();
    renderClockGreetingCalendar();
    setHidden(nameEditForm, true);
    editNameBtn.focus();
    if (typeof window.__newtabSetProfileName === "function") {
      window.__newtabSetProfileName(clean);
    }
    showToast("Nama sapaan disimpan");
  });

  document.addEventListener("mousedown", (e) => {
    if (!todoPopoverEl.hasAttribute("hidden")) {
      if (!todoPopoverEl.contains(e.target) && !todoCardBtn.contains(e.target)) {
        setHidden(todoPopoverEl, true);
        todoCardBtn.setAttribute("aria-expanded", "false");
      }
    }
  });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      if (!todoPopoverEl.hasAttribute("hidden")) {
        e.preventDefault();
        setTodoPopoverOpen(false);
        return;
      }
      if (!nameEditForm.hasAttribute("hidden")) {
        e.preventDefault();
        setHidden(nameEditForm, true);
        editNameBtn.focus();
        return;
      }
    }

    const activeTag = document.activeElement ? document.activeElement.tagName.toLowerCase() : "";
    const inEditable = activeTag === "input" || activeTag === "textarea";
    if (e.key === "/" && !e.ctrlKey && !e.altKey && !e.metaKey && !inEditable) {
      e.preventDefault();
      searchInput.focus();
    }
  });

  window.addEventListener("storage", (e) => {
    if (e.key === STORAGE_KEY) {
      state = loadState();
      renderWeather();
      renderTodos();
      renderClockGreetingCalendar();
    }
  });

  const ENGINE_LABELS = {
    google: "Google",
    duckduckgo: "DuckDuckGo",
    bing: "Bing",
  };

  window.__syncProfileName = (name) => {
    if (typeof name !== "string") return;
    const clean = name.trim().slice(0, 32) || "Budi";
    if (state.userName !== clean) {
      state.userName = clean;
      saveState();
      renderClockGreetingCalendar();
    }
  };

  window.__syncSearchEngine = (engine) => {
    const label = ENGINE_LABELS[engine] || "Google";
    searchInput.placeholder = `Cari di ${label} atau masukkan URL`;
  };

  renderWeather();
  renderTodos();
  renderClockGreetingCalendar();
  setInterval(renderClockGreetingCalendar, 1000);
})();
