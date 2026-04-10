const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

let focusedIndex = -1;
let projectRows = [];

// --- Render ---

function renderProjects(projects) {
  const listEl = document.getElementById("project-list");
  const emptyEl = document.getElementById("empty-state");
  const permEl = document.getElementById("permission-card");

  listEl.innerHTML = "";
  projectRows = [];
  focusedIndex = -1;

  if (projects.length === 0) {
    listEl.style.display = "none";
    emptyEl.style.display = "flex";
    permEl.style.display = "none";
    return;
  }

  listEl.style.display = "block";
  emptyEl.style.display = "none";
  permEl.style.display = "none";

  // Group by app
  const groups = {};
  for (const p of projects) {
    if (!groups[p.app_name]) {
      groups[p.app_name] = { icon: p.icon_data_uri, projects: [] };
    }
    groups[p.app_name].projects.push(p);
  }

  for (const [appName, group] of Object.entries(groups)) {
    const groupEl = document.createElement("div");
    groupEl.className = "app-group";

    // App header with window count badge
    const headerEl = document.createElement("div");
    headerEl.className = "app-header";
    if (group.icon) {
      const iconEl = document.createElement("img");
      iconEl.src = group.icon;
      iconEl.alt = appName;
      headerEl.appendChild(iconEl);
    }
    const nameEl = document.createElement("span");
    nameEl.textContent = appName;
    headerEl.appendChild(nameEl);

    // Window count badge
    const badgeEl = document.createElement("span");
    badgeEl.className = "badge";
    badgeEl.textContent = group.projects.length;
    headerEl.appendChild(badgeEl);

    groupEl.appendChild(headerEl);

    // Project rows
    for (const p of group.projects) {
      const rowEl = document.createElement("div");
      rowEl.className = "project-row";
      rowEl.tabIndex = 0;
      rowEl.setAttribute("role", "button");
      rowEl.setAttribute(
        "aria-label",
        `${appName}, ${p.project_name}${p.full_path ? ", " + p.full_path : ""}`
      );

      // Gradient pill indicator (replaces dot)
      const indicatorEl = document.createElement("div");
      indicatorEl.className = "indicator";
      rowEl.appendChild(indicatorEl);

      const pNameEl = document.createElement("span");
      pNameEl.className = "project-name";
      pNameEl.textContent = p.project_name;
      rowEl.appendChild(pNameEl);

      if (p.full_path) {
        const pathEl = document.createElement("span");
        pathEl.className = "project-path";
        pathEl.textContent = shortenPath(p.full_path);
        pathEl.title = p.full_path;
        rowEl.appendChild(pathEl);
      }

      rowEl.addEventListener("click", () => focusProject(p));
      rowEl.addEventListener("keydown", (e) => {
        if (e.key === "Enter") focusProject(p);
      });

      groupEl.appendChild(rowEl);
      projectRows.push(rowEl);
    }

    listEl.appendChild(groupEl);
  }
}

function shortenPath(fullPath) {
  const home = fullPath.replace(/^\/Users\/[^/]+/, "~");
  if (home.length > 30) {
    const parts = home.split("/");
    if (parts.length > 3) {
      return parts[0] + "/.../" + parts.slice(-2).join("/");
    }
  }
  return home;
}

// --- Actions ---

async function focusProject(project) {
  try {
    await invoke("focus_window", {
      pid: project.pid,
      windowIndex: project.window_index,
    });
    const win = getCurrentWindow();
    await win.hide();
  } catch (e) {
    console.error("Failed to focus window:", e);
  }
}

async function showPermissionCard() {
  document.getElementById("permission-card").style.display = "flex";
  document.getElementById("empty-state").style.display = "none";
  document.getElementById("project-list").style.display = "none";
}

// --- Keyboard navigation ---

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    getCurrentWindow().hide();
    return;
  }

  if (projectRows.length === 0) return;

  if (e.key === "ArrowDown") {
    e.preventDefault();
    focusedIndex = Math.min(focusedIndex + 1, projectRows.length - 1);
    updateFocus();
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    focusedIndex = Math.max(focusedIndex - 1, 0);
    updateFocus();
  }
});

function updateFocus() {
  projectRows.forEach((r, i) => {
    r.classList.toggle("focused", i === focusedIndex);
  });
  if (focusedIndex >= 0 && projectRows[focusedIndex]) {
    projectRows[focusedIndex].focus();
  }
}

// --- Init ---

async function init() {
  const trusted = await invoke("check_accessibility_permission");
  if (!trusted) {
    showPermissionCard();

    document.getElementById("open-settings-btn").addEventListener("click", async () => {
      await invoke("request_accessibility_permission");
    });

    const permInterval = setInterval(async () => {
      const nowTrusted = await invoke("check_accessibility_permission");
      if (nowTrusted) {
        clearInterval(permInterval);
        loadProjects();
      }
    }, 2000);

    return;
  }

  loadProjects();
}

async function loadProjects() {
  const projects = await invoke("get_projects");
  renderProjects(projects);
}

// Listen for updates from the Rust backend
listen("projects-updated", async () => {
  const trusted = await invoke("check_accessibility_permission");
  if (!trusted) return;

  const win = getCurrentWindow();
  if (await win.isVisible()) {
    loadProjects();
  }
});

window.addEventListener("DOMContentLoaded", init);
