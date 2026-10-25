// The demo library for the screenshots (scripts/screenshots.mjs): made-up games, runners and
// settings, the covers drawn for them, and a stand-in for Tauri's IPC that answers the frontend's
// commands from this data. Nothing here reads the real library or config.

const HOME = "/home/player";
const DATA = `${HOME}/.local/share/com.tristan.prefixr`;
const ART = `${DATA}/artwork`;

// Seconds since the Unix epoch, for a fixed day in 2026.
const day = (date) => Math.floor(Date.parse(`${date}T12:00:00Z`) / 1000);

const overrides = () => ({ performance: {}, graphics: {}, overlay: {}, proton: {} });

// name, runner, prefix folder, cover colors, store (if installed from one)
const GAMES = [
  ["starfall", "Starfall Odyssey", "GE-Proton10-4", ["#1b2a6b", "#e8743b"]],
  ["ironvale", "Ironvale Chronicles", "GE-Proton10-4", ["#3a2418", "#c9a227"]],
  ["neon-drift", "Neon Drift 2088", "GE-Proton9-27", ["#2b0b3f", "#ff2e88"]],
  ["lantern", "The Lantern Keeper", "wine-10.12-staging-amd64", ["#0e2a2a", "#f5c26b"]],
  ["hollow-tide", "Hollow Tide", "GE-Proton10-4", ["#06283d", "#47b5ff"]],
  ["pixel-garden", "Pixel Garden", "wine-10.12-staging-amd64", ["#1f3d1a", "#9be564"]],
  ["frostbound", "Frostbound Tactics", "GE-Proton9-27", ["#1c2b3a", "#cfe8ff"]],
  ["moonlit", "Moonlit Caravan", "GE-Proton10-4", ["#251a3d", "#f0d58a"], ["epic", "Cardamom"]],
  [
    "verdant",
    "Echoes of Verdant",
    "wine-10.12-staging-amd64",
    ["#123524", "#7fd1ae"],
    ["gog", "1427"],
  ],
  ["rustwood", "Rustwood Rally", "GE-Proton10-1", ["#3b1d0f", "#ff8c42"]],
];

// Store games not installed yet; the first two are downloading.
const STORE_ONLY = [
  ["epic", "Juniper", "Skyforge Arena", ["#14213d", "#fca311"]],
  ["gog", "2093", "Quiet Harbor", ["#0b3c49", "#a2d6f9"]],
  ["amazon", "amzn1.adg.product.demo", "Cinder Protocol", ["#2d0a0a", "#ff5a36"]],
];

export const COVERS = Object.fromEntries([
  ...GAMES.map(([id, name, , colors]) => [id, { name, colors }]),
  ...STORE_ONLY.map(([store, app, name, colors]) => [`${store}-${app}`, { name, colors }]),
]);

const games = GAMES.map(([id, name, runner, , store]) => {
  const folder = name.replace(/[^A-Za-z0-9]+/g, "");
  return {
    id,
    name,
    exe_path: `${HOME}/Games/${folder}/${folder}.exe`,
    prefix_path: `${HOME}/Games/Prefixes/${id}`,
    runner_id: runner,
    env_vars: {},
    launch_args: "",
    notes: "",
    icon: null,
    steamgriddb_id: null,
    cover_grid_id: null,
    cover_url: `https://cdn2.steamgriddb.com/grid/${id}.svg`,
    steamgriddb_icon_grid_id: null,
    steamgriddb_icon_url: null,
    artwork: {},
    umu_id: null,
    umu_store: null,
    overrides: overrides(),
    store: store
      ? {
          store: store[0],
          app_name: store[1],
          install_path: `${HOME}/Games/${folder}`,
          prerequisites: [],
        }
      : null,
  };
});

const key = (id) => {
  const game = games.find((g) => g.id === id);
  return game.store ? `${game.store.store}:${game.store.app_name}` : id;
};

const storeGames = [
  ...games
    .filter((g) => g.store)
    .map((g) => ({
      store: g.store.store,
      app_name: g.store.app_name,
      title: g.name,
      cover_path: `${ART}/${g.id}.svg`,
      game_id: g.id,
    })),
  ...STORE_ONLY.map(([store, app, title]) => ({
    store,
    app_name: app,
    title,
    cover_path: `${ART}/${store}-${app}.svg`,
    game_id: null,
  })),
];

const activity = (added, played, hours) => ({
  added_at: day(added),
  last_played: played ? day(played) : null,
  playtime_secs: Math.round(hours * 3600),
});

const library = {
  categories: [
    {
      id: "coop",
      name: "Co-op nights",
      games: [key("hollow-tide"), key("rustwood"), "epic:Juniper"],
    },
    { id: "cozy", name: "Cozy", games: [key("lantern"), key("pixel-garden"), key("verdant")] },
  ],
  favorites: [key("starfall"), key("hollow-tide"), key("lantern")],
  hidden: [],
  activity: {
    [key("starfall")]: activity("2026-03-02", "2026-06-14", 41.5),
    [key("ironvale")]: activity("2026-02-11", "2026-05-30", 18.2),
    [key("neon-drift")]: activity("2026-04-20", "2026-06-12", 9.75),
    [key("lantern")]: activity("2026-01-08", "2026-06-13", 26),
    [key("hollow-tide")]: activity("2026-05-01", "2026-06-10", 12.3),
    [key("pixel-garden")]: activity("2026-05-18", null, 0),
    [key("frostbound")]: activity("2026-03-27", "2026-04-02", 3.4),
    [key("moonlit")]: activity("2026-06-01", "2026-06-11", 5.1),
    [key("verdant")]: activity("2026-02-28", "2026-05-21", 14),
    [key("rustwood")]: activity("2026-06-05", null, 0),
  },
  tested: {
    [key("starfall")]: {
      status: "works",
      note: "Runs at a steady 60 FPS with FSR on. Controller works out of the box.",
      tested_at: day("2026-06-14"),
      runner_id: "GE-Proton10-4",
      exe_path: games[0].exe_path,
    },
    [key("lantern")]: {
      status: "works",
      note: "",
      tested_at: day("2026-05-20"),
      runner_id: "wine-10.12-staging-amd64",
      exe_path: games[3].exe_path,
    },
    [key("frostbound")]: {
      status: "issues",
      note: "Intro video stays black; skip it with Escape.",
      tested_at: day("2026-04-02"),
      runner_id: "GE-Proton9-27",
      exe_path: games[6].exe_path,
    },
  },
};

const compat = (tier, umu, steam = null) => ({
  umu_id: umu,
  umu_store: umu ? "none" : null,
  protondb_tier: tier,
  steam_app_id: steam,
});

const compatibility = {
  [key("starfall")]: compat("platinum", "umu-starfall"),
  [key("ironvale")]: compat("gold", null),
  [key("neon-drift")]: compat("platinum", null),
  [key("lantern")]: compat("silver", null),
  [key("hollow-tide")]: compat("gold", "umu-hollowtide"),
  [key("frostbound")]: compat("bronze", null),
  [key("moonlit")]: compat("platinum", "umu-moonlit"),
  [key("rustwood")]: compat("gold", null),
  "epic:Juniper": compat("gold", null),
  "gog:2093": compat("platinum", null),
};

const runner = (id, kind) => ({ id, name: id, path: `${DATA}/runners/${id}`, kind });
const runners = [
  runner("GE-Proton9-27", "proton"),
  runner("GE-Proton10-1", "proton"),
  runner("GE-Proton10-4", "proton"),
  runner("wine-9.22-amd64", "wine"),
  runner("wine-10.12-staging-amd64", "wine"),
];

const release = (source, tag, date, mb) => ({
  source,
  tag,
  name: tag,
  published_at: `${date}T16:00:00Z`,
  download_url: `https://github.com/example/${source}/releases/download/${tag}/${tag}.tar.gz`,
  size: mb * 1024 * 1024,
});

const releases = {
  "proton-ge": [
    release("proton-ge", "GE-Proton10-6", "2026-06-09", 512),
    release("proton-ge", "GE-Proton10-5", "2026-05-22", 509),
    release("proton-ge", "GE-Proton10-4", "2026-05-03", 507),
    release("proton-ge", "GE-Proton10-3", "2026-04-12", 498),
    release("proton-ge", "GE-Proton10-1", "2026-03-01", 495),
  ],
  "wine-kron4ek": [
    release("wine-kron4ek", "wine-10.12-staging-amd64", "2026-06-01", 96),
    release("wine-kron4ek", "wine-10.11-amd64", "2026-05-15", 92),
  ],
  "proton-cachyos": [],
};

const PROTON_OPTIONS = [
  ["PROTON_ENABLE_HDR", "hdr", ["PROTON_USE_HDR"]],
  ["PROTON_ENABLE_WAYLAND", "wayland", []],
  ["PROTON_USE_WINED3D", "wined3d", []],
  ["PROTON_DLSS_UPGRADE", "dlss", []],
  ["PROTON_FSR4_UPGRADE", "fsr4", []],
  ["PROTON_DISABLE_NVAPI", "disablenvapi", []],
  ["PROTON_NO_NTSYNC", "nontsync", []],
  ["PROTON_NO_FSYNC", "nofsync", []],
  ["PROTON_LOCAL_SHADER_CACHE", "localshadercache", []],
].map(([env, config, aliases]) => ({ env, config, aliases }));

const account = (store, connected, name, count, tool, version) => ({
  store,
  connected,
  account_name: name,
  game_count: count,
  tool_name: tool,
  tool_version: version,
  tool_source_url: `https://github.com/Heroic-Games-Launcher/${tool}`,
  tool_needs_python: false,
});

/**
 * Everything the stand-in answers with, keyed by command. Plain data, so it can be handed to the
 * page; commands that depend on their arguments are handled in `installDemo`.
 */
export function demoData(version) {
  return {
    version,
    games,
    storeGames,
    library,
    compatibility,
    runners,
    releases,
    answers: {
      list_games: games,
      list_store_games: storeGames,
      list_store_installs: [
        { store: "epic", app_name: "Juniper", percent: 63 },
        { store: "gog", app_name: "2093", percent: 18 },
      ],
      list_store_accounts: [
        account("epic", true, "player", 3, "legendary", "0.20.39"),
        account("gog", true, "player", 2, "gogdl", "1.1.2"),
        account("amazon", false, null, 0, "nile", "1.1.2"),
      ],
      list_steam_games: [games[0].id],
      list_active_games: [],
      // Flatpak-only answers; natively the app gets these.
      missing_flatpak_extensions: [],
      native_library: null,
      take_pending_launch: null,
      take_pending_install: null,
      get_library: library,
      list_runners: runners,
      list_runner_sources: [
        { id: "proton-ge", label: "Proton-GE", kind: "proton" },
        { id: "wine-kron4ek", label: "Wine (Kron4ek)", kind: "wine" },
        { id: "proton-cachyos", label: "Proton-CachyOS", kind: "proton" },
      ],
      list_prefixes: [...new Set(games.map((g) => g.prefix_path))].map((path) => ({ path })),
      prefix_states: [...new Set(games.map((g) => g.prefix_path))].map((path) => ({
        path,
        state: "ready",
      })),
      list_proton_options: PROTON_OPTIONS,
      get_github_config: { token: null },
      get_steamgriddb_config: { api_key: "demo-key" },
      get_umu_status: { installed: true, version: "1.2.9" },
      latest_umu_version: "1.2.9",
      get_directx_layers_status: [
        { label: "DXVK", installed: true, version: "v2.7.1" },
        { label: "VKD3D-Proton", installed: true, version: "v2.15" },
        { label: "wine-mono", installed: true, version: "wine-mono-10.2.0" },
      ],
      get_performance_config: {
        gamemode_enabled: true,
        power_profile_enabled: true,
        inhibit_sleep_enabled: true,
      },
      check_max_map_count: {
        current: 1048576,
        recommended: 1048576,
        sufficient: true,
        can_fix: true,
      },
      get_graphics_config: {
        gamescope: { enabled: false, width: 2560, height: 1440, fps_limit: 120, fullscreen: true },
        vkbasalt: { enabled: true, sharpen: true, sharpness: 0.4, smaa: false },
        launch_monitor: "primary",
      },
      get_mangohud_config: {
        enabled: true,
        preset: "standard",
        position: "top-left",
        theme_color: "ffffff",
        background_alpha: 0.4,
        round_corners: true,
        show_fps: true,
        show_frametime: true,
        show_cpu: true,
        show_gpu: true,
        show_ram: false,
        show_vram: false,
        show_temps: true,
        show_gamemode: false,
        show_vkbasalt: false,
        show_hdr: false,
        show_driver: false,
        show_engine_version: false,
        show_wine: false,
        show_gpu_name: false,
        show_resolution: false,
        horizontal: false,
      },
      get_proton_config: { options: { PROTON_ENABLE_WAYLAND: true } },
      get_backup_config: { automatic: true, interval_days: 1, keep: 7, dir: null },
      list_backups: [],
      set_ui_locale: null,
    },
  };
}

/**
 * Runs in the page before the app (Playwright's addInitScript): sets up what @tauri-apps/api reads
 * from `window.__TAURI_INTERNALS__`, as its own mocks module (`mockIPC`, `mockWindows`,
 * `mockConvertFileSrc`) does, and answers every command from `data`. A command without an answer
 * logs "demo: no answer for <command>" and gets an empty value, so a missing fixture shows up.
 */
export function installDemo(data) {
  localStorage.setItem("prefixr:locale", "en");

  const callbacks = new Map();
  let nextCallback = 1;
  const listeners = new Map();

  function transformCallback(callback, once = false) {
    const id = nextCallback++;
    callbacks.set(id, (payload) => {
      if (once) callbacks.delete(id);
      return callback && callback(payload);
    });
    return id;
  }

  const coverOf = (gameId) => {
    const game = data.games.find((g) => g.id === gameId);
    return game?.cover_url ? `/demo/artwork/${game.id}.svg` : null;
  };

  function answer(cmd, args = {}) {
    switch (cmd) {
      case "plugin:event|listen":
        if (!listeners.has(args.event)) listeners.set(args.event, []);
        listeners.get(args.event).push(args.handler);
        return args.handler;
      case "plugin:event|unlisten": {
        const handlers = listeners.get(args.event) ?? [];
        const index = handlers.indexOf(args.eventId);
        if (index !== -1) handlers.splice(index, 1);
        return null;
      }
      case "plugin:event|emit":
        for (const handler of listeners.get(args.event) ?? []) {
          callbacks.get(handler)?.({ event: args.event, id: handler, payload: args.payload });
        }
        return null;
      case "plugin:app|version":
        return data.version;
      case "get_game_cover":
        return coverOf(args.gameId);
      case "get_game_icon":
        return null;
      case "get_compatibility":
        return Object.fromEntries(
          (args.keys ?? [])
            .filter((k) => k in data.compatibility)
            .map((k) => [k, data.compatibility[k]]),
        );
      case "list_runner_releases":
        return data.releases[args.source] ?? [];
      case "get_store_install_defaults":
        return {
          install_dir: `/home/player/Games/${args.appName}`,
          new_prefix_path: `/home/player/Games/Prefixes/${args.appName}`,
          sizes: { download_size: 18_400_000_000, disk_size: 31_200_000_000 },
        };
    }
    if (cmd in data.answers) return structuredClone(data.answers[cmd]);
    console.warn(`demo: no answer for ${cmd}`);
    return cmd.startsWith("list_") ? [] : null;
  }

  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd, args) => answer(cmd, args),
    transformCallback,
    unregisterCallback: (id) => callbacks.delete(id),
    runCallback: (id, payload) => callbacks.get(id)?.(payload),
    callbacks,
    // The asset protocol: the screenshot server serves the demo artwork under /demo/artwork/.
    convertFileSrc: (path) =>
      path.startsWith("/demo/")
        ? path
        : `/demo/artwork/${encodeURIComponent(path.split("/").pop())}`,
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { windowLabel: "main", label: "main" },
    },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: (_event, id) => callbacks.delete(id),
  };
}

const escape = (text) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** Splits a title into lines of about `width` characters. */
function lines(title, width) {
  const out = [];
  for (const word of title.split(" ")) {
    const last = out.at(-1);
    if (last && (last + " " + word).length <= width) out[out.length - 1] = `${last} ${word}`;
    else out.push(word);
  }
  return out;
}

/**
 * A 600×900 cover (SteamGridDB's grid size) for a made-up game: a gradient, a few hills and a sun
 * in its colors, and its title. The same name always gives the same picture.
 */
export function coverSvg({ name, colors: [dark, light] }) {
  let seed = [...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);
  const rand = () => {
    seed = (seed * 1103515245 + 12345) >>> 0;
    return seed / 2 ** 32;
  };
  const sunX = 140 + Math.round(rand() * 320);
  const sunY = 400 + Math.round(rand() * 80);
  const hills = [0, 1, 2]
    .map((i) => {
      const base = 600 + i * 90;
      const peak = base - 80 - Math.round(rand() * 120);
      const mid = 100 + Math.round(rand() * 400);
      return `<path d="M0 ${base} Q${mid} ${peak} 600 ${base - 30 + i * 20} V900 H0 Z" fill="${dark}" opacity="${0.55 + i * 0.2}"/>`;
    })
    .join("");
  const titleLines = lines(name.toUpperCase(), 14);
  const text = titleLines
    .map(
      (line, i) =>
        `<text x="300" y="${270 + i * 58 - (titleLines.length - 1) * 29}" text-anchor="middle" font-family="DejaVu Sans, Liberation Sans, Arial, sans-serif" font-weight="700" font-size="48" letter-spacing="2" fill="#fff">${escape(line)}</text>`,
    )
    .join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" width="600" height="900" viewBox="0 0 600 900">
<defs><linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${dark}"/><stop offset="1" stop-color="${light}"/></linearGradient>
<radialGradient id="sun"><stop offset="0" stop-color="#fff" stop-opacity="0.95"/><stop offset="1" stop-color="${light}" stop-opacity="0"/></radialGradient></defs>
<rect width="600" height="900" fill="url(#sky)"/>
<circle cx="${sunX}" cy="${sunY}" r="150" fill="url(#sun)"/>
<circle cx="${sunX}" cy="${sunY}" r="60" fill="#fff" opacity="0.85"/>
${hills}
<rect width="600" height="900" fill="none" stroke="#000" stroke-opacity="0.25" stroke-width="8"/>
<g style="paint-order: stroke" stroke="#000" stroke-opacity="0.35" stroke-width="6">${text}</g>
</svg>`;
}
