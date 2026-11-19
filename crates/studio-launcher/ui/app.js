// Roblox Studio Linux Launcher (RtRust) - Frontend Client Logic
(function () {
  'use strict';

  function sendIpc(action) {
    if (window.ipc && window.ipc.postMessage) {
      window.ipc.postMessage(JSON.stringify(action));
    } else {
      console.log('IPC Out:', action);
    }
  }

  // Fallback high-res thumbnails for default baseplate & places
  const DEFAULT_THUMBS = [
    "https://tr.rbxcdn.com/180DAY-cc821ae6198f8ae795ba7dfdf003b879/768/432/Image/Webp/noFilter",
    "https://tr.rbxcdn.com/180DAY-a5d625b060d40767cb47020bc8f707f1/768/432/Image/Webp/noFilter",
    "https://tr.rbxcdn.com/180DAY-26a971e4e61db1d3f9b233a0bcfdfc94/768/432/Image/Webp/noFilter",
    "https://tr.rbxcdn.com/180DAY-70732dfd5f661df6e100f9a2e6f404fa/768/432/Image/Webp/noFilter"
  ];

  const PRESET_WALLPAPERS = {
    'studio-dark': 'linear-gradient(135deg, #0d0f12 0%, #15181d 50%, #0d0f12 100%)',
    'neon-grid': 'radial-gradient(circle at 50% 50%, rgba(0, 245, 212, 0.15) 0%, #080a0e 75%)',
    'synthwave': 'linear-gradient(135deg, #1b0726 0%, #290832 45%, #ff4655 100%)',
    'cyber-void': 'radial-gradient(circle at 75% 25%, rgba(255, 70, 85, 0.25) 0%, #05070a 65%)'
  };

  let appState = {
    user: null,
    places: [
      { name: "Default Baseplate", place_id: 95206881, description: "Standard Template with Vulkan Caching", thumbnail_url: DEFAULT_THUMBS[0] },
      { name: "Testing Sandbox", place_id: 1818, description: "Quick Prototype & Script Testing", thumbnail_url: DEFAULT_THUMBS[1] }
    ],
    trending: [
      { name: "Blox Fruits", place_id: 2753915549, description: "Trending Anime Action RPG", playing: "450K+ Playing", thumbnail_url: DEFAULT_THUMBS[0] },
      { name: "Brookhaven RP", place_id: 4924922222, description: "Top Roleplay Community Hub", playing: "380K+ Playing", thumbnail_url: DEFAULT_THUMBS[1] },
      { name: "DOORS 👁️", place_id: 6516141723, description: "Atmospheric Horror & Puzzle", playing: "45K+ Playing", thumbnail_url: DEFAULT_THUMBS[2] },
      { name: "Rivals FPS", place_id: 17625359962, description: "High-Octane Competitive FPS", playing: "80K+ Playing", thumbnail_url: DEFAULT_THUMBS[3] }
    ],
    selectedPlaceId: 95206881,
    selectedPlaceName: "Default Baseplate",
    selectedPlaceDesc: "Standard Roblox Studio template with DXVK Vulkan pipeline caching enabled.",
    selectedThumb: DEFAULT_THUMBS[0],
    fps: 60,
    force_xwayland: true,
    hardware: null,
    config: {
      profile_mode: "auto",
      graphics_backend: "auto",
      dxvk_max_vram_mb: 0,
      editor_quality_level: 16,
      disable_shadows: false,
      block_webview2: false,
      force_xwayland_mouse: true,
      throttle_3d_inactive: true,
      disable_postfx: true,
      virtual_desktop: false,
      auto_hide_launcher: true,
      custom_runner_path: "",
      custom_env_vars: "",
      custom_cli_flags: "",
      wallpaper_mode: "auto",
      wallpaper_preset: "studio-dark",
      wallpaper_path: null,
      wallpaper_blur: 12,
      wallpaper_dim: 45,
      raw_custom_fflags: {},
      first_run_completed: false
    }
  };

  // --- DOM Elements ---
  const titlebar = document.getElementById('titlebar');
  const btnMinimize = document.getElementById('btn-minimize');
  const btnMaximize = document.getElementById('btn-maximize');
  const btnClose = document.getElementById('btn-close');

  const loginView = document.getElementById('login-view');
  const hubView = document.getElementById('hub-view');
  const btnLoginWebview = document.getElementById('btn-login-webview');
  const quickCodeDisplay = document.getElementById('quick-code-display');
  const quickTimer = document.getElementById('quick-timer');
  const quickStatusText = document.getElementById('quick-status-text');
  const btnRefreshQuickLogin = document.getElementById('btn-refresh-quick-login');
  const btnGuest = document.getElementById('btn-guest');
  const btnLoginSettings = document.getElementById('btn-login-settings');

  const navPills = document.querySelectorAll('.nav-pill');
  const tabPanes = document.querySelectorAll('.tab-pane');

  const hubHeroBg = document.getElementById('hub-hero-bg');
  const hubHeroVideo = document.getElementById('hub-hero-video');
  const hubHeroOverlay = document.getElementById('hub-hero-overlay');
  const heroGameTitle = document.getElementById('hero-game-title');
  const heroGameDesc = document.getElementById('hero-game-desc');

  const userAvatarBadge = document.getElementById('user-avatar-badge');
  const userAvatarBox = document.getElementById('user-avatar-box');
  const userDisplayName = document.getElementById('user-display-name');
  const userHandleName = document.getElementById('user-handle-name');
  const profileDropdown = document.getElementById('profile-dropdown-menu');
  const linkCreatorHub = document.getElementById('link-creator-hub');
  const linkRobloxHome = document.getElementById('link-roblox-home');
  const btnSignout = document.getElementById('btn-signout');

  const myPlacesCount = document.getElementById('my-places-count');
  const placesGrid = document.getElementById('places-grid-container');
  const trendingGrid = document.getElementById('trending-grid-container');

  // Bottom action dock elements
  const btnStartStudio = document.getElementById('btn-start-studio');
  const statusIndicatorDot = document.getElementById('status-indicator-dot');
  const statusDisplayText = document.getElementById('status-display-text');
  const dockFpsSelector = document.getElementById('dock-fps-selector');
  const btnQuickWaylandToggle = document.getElementById('btn-quick-wayland-toggle');
  const waylandStatusLabel = document.getElementById('wayland-status-label');
  const summaryThumbMini = document.getElementById('summary-thumb-mini');
  const summaryPlaceName = document.getElementById('summary-place-name');
  const summaryPlaceId = document.getElementById('summary-place-id');

  // Profile Selector & Reset
  const profilePills = document.querySelectorAll('.profile-pill-btn');
  const profileSubDesc = document.getElementById('profile-sub-desc');
  const btnResetProfile = document.getElementById('btn-reset-profile');

  // Granular Settings Elements
  const settingGraphicsBackend = document.getElementById('setting-graphics-backend');
  const settingVramLimit = document.getElementById('setting-vram-limit');
  const valVramLimit = document.getElementById('val-vram-limit');
  const settingQualityLevel = document.getElementById('setting-quality-level');
  const valQualityLevel = document.getElementById('val-quality-level');
  const settingWayland = document.getElementById('setting-wayland');
  const settingThrottle = document.getElementById('setting-throttle');
  const settingPostfx = document.getElementById('setting-postfx');
  const settingDisableShadows = document.getElementById('setting-disable-shadows');
  const settingBlockWebview2 = document.getElementById('setting-block-webview2');
  const settingVirtualDesktop = document.getElementById('setting-virtual-desktop');
  const settingAutohide = document.getElementById('setting-autohide');

  const settingCustomRunner = document.getElementById('setting-custom-runner');
  const settingCustomEnv = document.getElementById('setting-custom-env');
  const settingCustomFlags = document.getElementById('setting-custom-flags');

  // Rust DXGI Shim & Resource Guard Elements
  const settingUseDxgiShim = document.getElementById('setting-use-dxgi-shim');
  const settingAltTabGpuSleep = document.getElementById('setting-alt-tab-gpu-sleep');
  const settingVramCap = document.getElementById('setting-vram-cap');
  const settingCgroupRam = document.getElementById('setting-cgroup-ram');
  const settingDisableMsaa = document.getElementById('setting-disable-msaa');
  const settingForceVoxel = document.getElementById('setting-force-voxel');
  const settingDisableGrass = document.getElementById('setting-disable-grass');
  const telemetryBadge = document.getElementById('telemetry-badge');
  const telemetryText = document.getElementById('telemetry-text');

  // Directory & Plugin Buttons
  const btnOpenPlugins = document.getElementById('btn-open-plugins');
  const btnImportPlugins = document.getElementById('btn-import-plugins');
  const btnOpenStudioDir = document.getElementById('btn-open-studio-dir');
  const btnOpenPrefixDir = document.getElementById('btn-open-prefix-dir');
  const btnRestoreDll = document.getElementById('btn-restore-dll');
  const btnOpenRawFflags = document.getElementById('btn-open-raw-fflags');

  // Modals
  const modalAddPlace = document.getElementById('modal-add-place');
  const modalPlaceName = document.getElementById('modal-place-name');
  const modalPlaceId = document.getElementById('modal-place-id');
  const modalBtnCancel = document.getElementById('modal-btn-cancel');
  const modalBtnSave = document.getElementById('modal-btn-save');

  const modalWallpaper = document.getElementById('modal-wallpaper');
  const btnOpenWallpaper = document.getElementById('btn-open-wallpaper');
  const btnCloseWallpaper = document.getElementById('btn-close-wallpaper');
  const btnCancelWallpaper = document.getElementById('btn-cancel-wallpaper');
  const btnSaveWallpaper = document.getElementById('btn-save-wallpaper');
  const btnPickWallpaperFile = document.getElementById('btn-pick-wallpaper-file');
  const wpCurrentPathLabel = document.getElementById('wp-current-path-label');
  const wpBlurSlider = document.getElementById('wp-blur-slider');
  const valWpBlur = document.getElementById('val-wp-blur');
  const wpDimSlider = document.getElementById('wp-dim-slider');
  const valWpDim = document.getElementById('val-wp-dim');

  const modalRawFflags = document.getElementById('modal-raw-fflags');
  const btnCloseRawFflags = document.getElementById('btn-close-raw-fflags');
  const btnCancelRawFflags = document.getElementById('btn-cancel-raw-fflags');
  const btnSaveRawFflags = document.getElementById('btn-save-raw-fflags');
  const btnClearCustomFflags = document.getElementById('btn-clear-custom-fflags');
  const rawFflagsTextarea = document.getElementById('raw-fflags-textarea');
  const rawFflagsStatus = document.getElementById('raw-fflags-status');

  const modalOnboarding = document.getElementById('modal-onboarding');
  const btnSkipOnboarding = document.getElementById('btn-skip-onboarding');
  const btnApplyOnboarding = document.getElementById('btn-apply-onboarding');

  // --- Titlebar Drag & Window Controls ---
  titlebar.addEventListener('mousedown', (e) => {
    if (e.target.closest('.win-btn') || e.target.closest('button') || e.target.closest('input')) return;
    if (e.button === 0) {
      if (e.detail === 2) {
        sendIpc({ type: 'maximize' });
      } else {
        sendIpc({ type: 'drag_window' });
      }
    }
  });

  btnMinimize.addEventListener('click', () => sendIpc({ type: 'minimize' }));
  btnMaximize.addEventListener('click', () => sendIpc({ type: 'maximize' }));
  btnClose.addEventListener('click', () => sendIpc({ type: 'close' }));

  // --- Tab Navigation ---
  navPills.forEach(pill => {
    pill.addEventListener('click', () => {
      navPills.forEach(p => p.classList.remove('active'));
      tabPanes.forEach(t => t.classList.remove('active'));

      pill.classList.add('active');
      const targetTab = document.getElementById(pill.dataset.tab);
      if (targetTab) {
        targetTab.classList.remove('hidden');
        targetTab.classList.add('active');
      }
    });
  });

  // --- Profile Dropdown ---
  userAvatarBadge.addEventListener('click', (e) => {
    e.stopPropagation();
    profileDropdown.classList.toggle('hidden');
  });

  document.addEventListener('click', () => {
    profileDropdown.classList.add('hidden');
  });

  linkCreatorHub.addEventListener('click', () => sendIpc({ type: 'open_url', url: 'https://create.roblox.com/dashboard' }));
  linkRobloxHome.addEventListener('click', () => sendIpc({ type: 'open_url', url: 'https://www.roblox.com/home' }));
  btnSignout.addEventListener('click', () => sendIpc({ type: 'sign_out' }));

  // --- Login Screen Actions ---
  btnLoginWebview.addEventListener('click', () => sendIpc({ type: 'start_webview_login' }));
  btnRefreshQuickLogin.addEventListener('click', () => {
    quickCodeDisplay.textContent = '...';
    quickStatusText.textContent = 'Refreshing code...';
    sendIpc({ type: 'start_quick_login' });
  });

  btnGuest.addEventListener('click', showHub);
  btnLoginSettings.addEventListener('click', () => {
    showHub();
    const enginePill = document.querySelector('.nav-pill[data-tab="tab-engine"]');
    if (enginePill) enginePill.click();
  });

  function showHub() {
    loginView.classList.add('hidden');
    hubView.classList.remove('hidden');
  }

  function showLogin() {
    hubView.classList.add('hidden');
    loginView.classList.remove('hidden');
  }

  // --- Quick Folder Management Buttons ---
  if (btnOpenPlugins) btnOpenPlugins.addEventListener('click', () => sendIpc({ type: 'open_plugins_folder' }));
  if (btnImportPlugins) btnImportPlugins.addEventListener('click', () => sendIpc({ type: 'import_plugins' }));
  if (btnOpenStudioDir) btnOpenStudioDir.addEventListener('click', () => sendIpc({ type: 'open_studio_folder' }));
  if (btnOpenPrefixDir) btnOpenPrefixDir.addEventListener('click', () => sendIpc({ type: 'open_prefix_folder' }));
  if (btnRestoreDll) btnRestoreDll.addEventListener('click', () => sendIpc({ type: 'restore_original_dll' }));

  // --- Profile Selector Logic ---
  const PROFILE_DESCRIPTIONS = {
    'auto': 'Auto-tunes graphics, VRAM, and memory footprint based on detected hardware specs.',
    'potato': 'Aggressive Low-RAM mode: Disables PostFX/Shadows, blocks WebView2, caps DXVK VRAM to 1GB.',
    'balanced': 'Sweet spot: High-res Studio rendering with WebView2 blocked and 2GB DXVK memory budget.',
    'high': 'Max Fidelity: 144/240 FPS unlocked, sharp shadows, Future lighting, unlimited VRAM.',
    'custom': 'Custom manual configuration: Custom toggles and sliders are locked and never overwritten.'
  };

  profilePills.forEach(pill => {
    pill.addEventListener('click', () => {
      const mode = pill.dataset.profile;
      profilePills.forEach(p => p.classList.remove('active'));
      pill.classList.add('active');
      if (profileSubDesc) profileSubDesc.textContent = PROFILE_DESCRIPTIONS[mode] || '';
      sendIpc({ type: 'apply_profile', profile: mode });
    });
  });

  if (btnResetProfile) {
    btnResetProfile.addEventListener('click', () => {
      sendIpc({ type: 'reset_recommended_profile' });
    });
  }

  // --- Granular Settings Sync ---
  let syncTimeout = null;
  function triggerSettingsSync(markCustom = true) {
    if (markCustom && appState.config.profile_mode !== 'custom') {
      appState.config.profile_mode = 'custom';
      profilePills.forEach(p => p.classList.toggle('active', p.dataset.profile === 'custom'));
      if (profileSubDesc) profileSubDesc.textContent = PROFILE_DESCRIPTIONS['custom'];
    }

    clearTimeout(syncTimeout);
    syncTimeout = setTimeout(() => {
      const cfg = {
        target_fps: appState.fps,
        profile_mode: appState.config.profile_mode,
        graphics_backend: settingGraphicsBackend.value,
        dxvk_max_vram_mb: parseInt(settingVramLimit.value, 10) || 0,
        editor_quality_level: parseInt(settingQualityLevel.value, 10) || 16,
        disable_shadows: settingDisableShadows.checked,
        block_webview2: settingBlockWebview2.checked,
        force_xwayland_mouse: settingWayland.checked,
        throttle_3d_inactive: settingThrottle.checked,
        disable_postfx: settingPostfx.checked,
        virtual_desktop: settingVirtualDesktop.checked,
        auto_hide_launcher: settingAutohide.checked,
        use_dxgi_shim: settingUseDxgiShim ? settingUseDxgiShim.checked : true,
        alt_tab_gpu_sleep: settingAltTabGpuSleep ? settingAltTabGpuSleep.checked : true,
        vram_cap_mb: settingVramCap ? (parseInt(settingVramCap.value, 10) || 0) : 1024,
        cgroup_ram_limit_gb: settingCgroupRam ? (parseInt(settingCgroupRam.value, 10) || 0) : 6,
        disable_msaa: settingDisableMsaa ? settingDisableMsaa.checked : true,
        force_voxel_lighting: settingForceVoxel ? settingForceVoxel.checked : false,
        disable_grass: settingDisableGrass ? settingDisableGrass.checked : true,
        custom_runner_path: settingCustomRunner.value.trim() || null,
        custom_env_vars: settingCustomEnv.value.trim() || null,
        custom_cli_flags: settingCustomFlags.value.trim() || null,
        wallpaper_mode: appState.config.wallpaper_mode,
        wallpaper_preset: appState.config.wallpaper_preset,
        wallpaper_path: appState.config.wallpaper_path,
        wallpaper_blur: appState.config.wallpaper_blur,
        wallpaper_dim: appState.config.wallpaper_dim,
        raw_custom_fflags: appState.config.raw_custom_fflags || {},
        first_run_completed: appState.config.first_run_completed
      };
      sendIpc({ type: 'save_config', config: cfg });
    }, 200);
  }

  // Sliders UI update
  settingVramLimit.addEventListener('input', () => {
    const v = parseInt(settingVramLimit.value, 10);
    valVramLimit.textContent = v === 0 ? 'Unlimited' : `${v} MB`;
    triggerSettingsSync(true);
  });

  settingQualityLevel.addEventListener('input', () => {
    const q = parseInt(settingQualityLevel.value, 10);
    valQualityLevel.textContent = `Level ${q}`;
    triggerSettingsSync(true);
  });

  settingGraphicsBackend.addEventListener('change', () => triggerSettingsSync(true));

  [
    settingWayland,
    settingThrottle,
    settingPostfx,
    settingDisableShadows,
    settingBlockWebview2,
    settingVirtualDesktop,
    settingAutohide,
    settingUseDxgiShim,
    settingAltTabGpuSleep,
    settingDisableMsaa,
    settingForceVoxel,
    settingDisableGrass
  ].filter(Boolean).forEach(el => {
    el.addEventListener('change', () => {
      if (el === settingWayland) {
        appState.force_xwayland = settingWayland.checked;
        updateWaylandPillUI();
      }
      triggerSettingsSync(true);
    });
  });

  [settingVramCap, settingCgroupRam].filter(Boolean).forEach(el => {
    el.addEventListener('change', () => triggerSettingsSync(true));
  });

  [settingCustomRunner, settingCustomEnv, settingCustomFlags].forEach(input => {
    input.addEventListener('input', () => triggerSettingsSync(true));
  });

  // --- Places Rendering & Selection ---
  function selectPlace(placeId, name, desc, thumb) {
    appState.selectedPlaceId = placeId;
    appState.selectedPlaceName = name;
    appState.selectedPlaceDesc = desc;
    appState.selectedThumb = thumb;

    if (heroGameTitle) heroGameTitle.textContent = name.toUpperCase();
    if (heroGameDesc) heroGameDesc.textContent = desc || "Experience ready to launch.";
    if (summaryPlaceName) summaryPlaceName.textContent = name;
    if (summaryPlaceId) summaryPlaceId.textContent = `ID: ${placeId}`;
    if (summaryThumbMini && thumb) summaryThumbMini.style.backgroundImage = `url("${thumb}")`;

    applyWallpaper();
    renderPlaces();
  }

  function renderPlaces() {
    if (!placesGrid) return;
    placesGrid.innerHTML = '';
    if (myPlacesCount) myPlacesCount.textContent = `${appState.places.length} PLACES`;

    appState.places.forEach(p => {
      const isSelected = p.place_id === appState.selectedPlaceId;
      const thumb = p.thumbnail_url || DEFAULT_THUMBS[0];

      const card = document.createElement('div');
      card.className = `experience-card spotlight-card ${isSelected ? 'active' : ''}`;
      card.innerHTML = `
        <div class="card-thumb-wrap">
          <div class="card-thumb-img" style="background-image: url('${thumb}');"></div>
          <div class="card-status-badge ${isSelected ? 'active' : ''}">${isSelected ? 'SELECTED' : 'SAVED'}</div>
        </div>
        <div class="card-content">
          <div class="card-title">${escapeHtml(p.name)}</div>
          <div class="card-meta-row">
            <span class="card-id-label">ID: ${p.place_id}</span>
            <span style="font-size: 10px; color: var(--accent-cyan); font-weight:700;">DXVK Ready</span>
          </div>
        </div>
      `;

      card.addEventListener('click', () => {
        selectPlace(p.place_id, p.name, p.description, thumb);
      });

      placesGrid.appendChild(card);
    });

    // "Add Place" Action Card
    const addCard = document.createElement('div');
    addCard.className = 'card-add-action';
    addCard.innerHTML = `
      <div class="card-add-circle">+</div>
      <div style="font-size: 13px; font-weight: 700;">Add Experience</div>
      <div style="font-size: 10px; color: var(--text-dim);">Open via Place ID</div>
    `;
    addCard.addEventListener('click', () => {
      modalAddPlace.classList.remove('hidden');
      modalPlaceName.value = '';
      modalPlaceId.value = '';
      modalPlaceName.focus();
    });
    placesGrid.appendChild(addCard);
  }

  function renderTrending() {
    if (!trendingGrid) return;
    trendingGrid.innerHTML = '';
    appState.trending.forEach(p => {
      const isSelected = p.place_id === appState.selectedPlaceId;
      const thumb = p.thumbnail_url || DEFAULT_THUMBS[0];

      const card = document.createElement('div');
      card.className = `experience-card spotlight-card ${isSelected ? 'active' : ''}`;
      card.innerHTML = `
        <div class="card-thumb-wrap">
          <div class="card-thumb-img" style="background-image: url('${thumb}');"></div>
          <div class="card-status-badge ${isSelected ? 'active' : ''}">${isSelected ? 'SELECTED' : 'TRENDING'}</div>
        </div>
        <div class="card-content">
          <div class="card-title">${escapeHtml(p.name)}</div>
          <div class="card-meta-row">
            <span class="card-ccu-badge">
              <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="12" r="10"/></svg>
              ${p.playing || 'Active'}
            </span>
            <span class="card-id-label">ID: ${p.place_id}</span>
          </div>
        </div>
      `;

      card.addEventListener('click', () => {
        selectPlace(p.place_id, p.name, p.description, thumb);
      });

      trendingGrid.appendChild(card);
    });
  }

  // --- Add Place Modal Logic ---
  modalBtnCancel.addEventListener('click', () => modalAddPlace.classList.add('hidden'));
  modalBtnSave.addEventListener('click', () => {
    const name = modalPlaceName.value.trim() || 'Custom Place';
    const placeId = parseInt(modalPlaceId.value.trim(), 10);
    if (!placeId) {
      window.showToast('Please enter a valid numeric Place ID', false);
      return;
    }

    const newPlace = {
      name,
      place_id: placeId,
      description: 'Custom Place',
      thumbnail_url: DEFAULT_THUMBS[appState.places.length % DEFAULT_THUMBS.length]
    };

    appState.places.unshift(newPlace);
    modalAddPlace.classList.add('hidden');
    selectPlace(placeId, name, newPlace.description, newPlace.thumbnail_url);
    sendIpc({ type: 'add_place', name, place_id: placeId });
  });

  // --- Wallpaper Studio Logic (Hardware-accelerated) ---
  function applyWallpaper() {
    const mode = appState.config.wallpaper_mode || 'auto';
    const blur = appState.config.wallpaper_blur !== undefined ? appState.config.wallpaper_blur : 0;
    const dim = appState.config.wallpaper_dim !== undefined ? appState.config.wallpaper_dim : 45;

    if (blur > 0) {
      hubHeroBg.style.filter = `blur(${Math.min(blur, 8)}px)`;
    } else {
      hubHeroBg.style.filter = 'none';
    }

    if (hubHeroOverlay) {
      hubHeroOverlay.style.background = `rgba(15, 18, 24, ${dim / 100})`;
    }

    if (mode === 'local' && appState.config.wallpaper_path) {
      const p = appState.config.wallpaper_path;
      if (p.endsWith('.mp4') || p.endsWith('.webm')) {
        hubHeroBg.style.backgroundImage = 'none';
        hubHeroVideo.src = `file://${p}`;
        hubHeroVideo.classList.remove('hidden');
        hubHeroVideo.play().catch(() => {});
        return;
      } else {
        hubHeroVideo.classList.add('hidden');
        hubHeroBg.style.backgroundImage = `url("file://${p}")`;
        return;
      }
    }

    hubHeroVideo.classList.add('hidden');
    if (mode === 'preset') {
      const presetKey = appState.config.wallpaper_preset || 'studio-dark';
      hubHeroBg.style.backgroundImage = PRESET_WALLPAPERS[presetKey] || PRESET_WALLPAPERS['studio-dark'];
    } else {
      // Auto place sync
      hubHeroBg.style.backgroundImage = `url("${appState.selectedThumb || DEFAULT_THUMBS[0]}")`;
    }
  }

  if (btnOpenWallpaper) {
    btnOpenWallpaper.addEventListener('click', () => {
      modalWallpaper.classList.remove('hidden');
      updateWallpaperModalUI();
    });
  }

  if (btnCloseWallpaper) btnCloseWallpaper.addEventListener('click', () => modalWallpaper.classList.add('hidden'));
  if (btnCancelWallpaper) btnCancelWallpaper.addEventListener('click', () => modalWallpaper.classList.add('hidden'));

  // Wallpaper Mode Buttons
  const wpModeBtns = document.querySelectorAll('.wp-mode-btn');
  wpModeBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      wpModeBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      const mode = btn.dataset.mode;
      appState.config.wallpaper_mode = mode;

      document.getElementById('wp-pane-auto').classList.toggle('hidden', mode !== 'auto');
      document.getElementById('wp-pane-local').classList.toggle('hidden', mode !== 'local');
      document.getElementById('wp-pane-preset').classList.toggle('hidden', mode !== 'preset');
      applyWallpaper();
    });
  });

  if (btnPickWallpaperFile) {
    btnPickWallpaperFile.addEventListener('click', () => {
      sendIpc({ type: 'pick_wallpaper_file' });
    });
  }

  // Presets click
  const presetCards = document.querySelectorAll('.preset-card');
  presetCards.forEach(card => {
    card.addEventListener('click', () => {
      presetCards.forEach(c => c.classList.remove('active'));
      card.classList.add('active');
      appState.config.wallpaper_preset = card.dataset.preset;
      applyWallpaper();
    });
  });

  // Blur & Dim Sliders
  wpBlurSlider.addEventListener('input', () => {
    const val = parseInt(wpBlurSlider.value, 10);
    valWpBlur.textContent = `${val}px`;
    appState.config.wallpaper_blur = val;
    applyWallpaper();
  });

  wpDimSlider.addEventListener('input', () => {
    const val = parseInt(wpDimSlider.value, 10);
    valWpDim.textContent = `${val}%`;
    appState.config.wallpaper_dim = val;
    applyWallpaper();
  });

  if (btnSaveWallpaper) {
    btnSaveWallpaper.addEventListener('click', () => {
      modalWallpaper.classList.add('hidden');
      triggerSettingsSync(false);
      window.showToast('Wallpaper theme applied!', true);
    });
  }

  function updateWallpaperModalUI() {
    const mode = appState.config.wallpaper_mode || 'auto';
    wpModeBtns.forEach(b => b.classList.toggle('active', b.dataset.mode === mode));
    document.getElementById('wp-pane-auto').classList.toggle('hidden', mode !== 'auto');
    document.getElementById('wp-pane-local').classList.toggle('hidden', mode !== 'local');
    document.getElementById('wp-pane-preset').classList.toggle('hidden', mode !== 'preset');

    if (wpCurrentPathLabel) {
      wpCurrentPathLabel.textContent = appState.config.wallpaper_path || 'No local media selected';
    }

    const preset = appState.config.wallpaper_preset || 'studio-dark';
    presetCards.forEach(c => c.classList.toggle('active', c.dataset.preset === preset));

    wpBlurSlider.value = appState.config.wallpaper_blur !== undefined ? appState.config.wallpaper_blur : 12;
    valWpBlur.textContent = `${wpBlurSlider.value}px`;

    wpDimSlider.value = appState.config.wallpaper_dim !== undefined ? appState.config.wallpaper_dim : 45;
    valWpDim.textContent = `${wpDimSlider.value}%`;
  }

  // --- Raw FFlags JSON Editor Logic ---
  if (btnOpenRawFflags) {
    btnOpenRawFflags.addEventListener('click', () => {
      modalRawFflags.classList.remove('hidden');
      const currentFlags = appState.config.raw_custom_fflags || {};
      rawFflagsTextarea.value = JSON.stringify(currentFlags, null, 2);
      validateRawFflagsJson();
      rawFflagsTextarea.focus();
    });
  }

  if (btnCloseRawFflags) btnCloseRawFflags.addEventListener('click', () => modalRawFflags.classList.add('hidden'));
  if (btnCancelRawFflags) btnCancelRawFflags.addEventListener('click', () => modalRawFflags.classList.add('hidden'));

  function validateRawFflagsJson() {
    const text = rawFflagsTextarea.value.trim();
    if (!text) {
      rawFflagsStatus.innerHTML = '<span class="status-valid">✔ Empty (No custom overrides)</span>';
      return true;
    }
    try {
      const parsed = JSON.parse(text);
      if (typeof parsed !== 'object' || Array.isArray(parsed) || parsed === null) {
        rawFflagsStatus.innerHTML = '<span class="status-error">✖ JSON root must be a Key-Value Object</span>';
        return false;
      }
      const count = Object.keys(parsed).length;
      rawFflagsStatus.innerHTML = `<span class="status-valid">✔ Valid JSON syntax (${count} custom flags)</span>`;
      return true;
    } catch (e) {
      rawFflagsStatus.innerHTML = `<span class="status-error">✖ Syntax Error: ${escapeHtml(e.message)}</span>`;
      return false;
    }
  }

  rawFflagsTextarea.addEventListener('input', validateRawFflagsJson);

  // Quick chips insert
  document.querySelectorAll('.fflag-quick-chips .chip-item').forEach(chip => {
    chip.addEventListener('click', () => {
      const key = chip.dataset.key;
      let val = chip.dataset.val;
      if (val === 'true') val = "True";
      else if (!isNaN(val)) val = parseInt(val, 10);

      let current = {};
      try {
        current = JSON.parse(rawFflagsTextarea.value.trim() || '{}');
      } catch (e) {
        current = {};
      }
      current[key] = val;
      rawFflagsTextarea.value = JSON.stringify(current, null, 2);
      validateRawFflagsJson();
    });
  });

  if (btnClearCustomFflags) {
    btnClearCustomFflags.addEventListener('click', () => {
      rawFflagsTextarea.value = '{\n}';
      validateRawFflagsJson();
    });
  }

  if (btnSaveRawFflags) {
    btnSaveRawFflags.addEventListener('click', () => {
      if (!validateRawFflagsJson()) {
        window.showToast('Please fix JSON syntax errors before saving', false);
        return;
      }
      let flags = {};
      try {
        flags = JSON.parse(rawFflagsTextarea.value.trim() || '{}');
      } catch (e) {
        flags = {};
      }
      appState.config.raw_custom_fflags = flags;
      sendIpc({ type: 'save_raw_fflags', fflags: flags });
      modalRawFflags.classList.add('hidden');
    });
  }

  // --- First-Run Onboarding Wizard Logic ---
  if (btnSkipOnboarding) {
    btnSkipOnboarding.addEventListener('click', () => {
      modalOnboarding.classList.add('hidden');
      sendIpc({ type: 'complete_onboarding', profile: 'auto' });
    });
  }

  if (btnApplyOnboarding) {
    btnApplyOnboarding.addEventListener('click', () => {
      modalOnboarding.classList.add('hidden');
      const rec = (appState.hardware && appState.hardware.recommended_profile) || 'auto';
      sendIpc({ type: 'complete_onboarding', profile: rec });
    });
  }

  // --- Dock FPS Selector & Wayland Quick Toggle ---
  if (dockFpsSelector) {
    dockFpsSelector.querySelectorAll('.fps-pill-option').forEach(pill => {
      pill.addEventListener('click', () => {
        dockFpsSelector.querySelectorAll('.fps-pill-option').forEach(p => p.classList.remove('active'));
        pill.classList.add('active');
        appState.fps = parseInt(pill.dataset.fps, 10);
        triggerSettingsSync(true);
      });
    });
  }

  if (btnQuickWaylandToggle) {
    btnQuickWaylandToggle.addEventListener('click', () => {
      appState.force_xwayland = !appState.force_xwayland;
      if (settingWayland) settingWayland.checked = appState.force_xwayland;
      updateWaylandPillUI();
      triggerSettingsSync(true);
    });
  }

  function updateWaylandPillUI() {
    if (!btnQuickWaylandToggle || !waylandStatusLabel) return;
    if (appState.force_xwayland) {
      btnQuickWaylandToggle.classList.add('active');
      waylandStatusLabel.textContent = 'Anti-Spin: Active';
    } else {
      btnQuickWaylandToggle.classList.remove('active');
      waylandStatusLabel.textContent = 'Anti-Spin: Off';
    }
  }

  // --- START / UPDATE STUDIO BUTTON ---
  btnStartStudio.addEventListener('click', () => {
    if (appState.isUpdateAvailable) {
      statusIndicatorDot.className = 'status-indicator-dot busy';
      statusDisplayText.textContent = 'Downloading Studio packages & compiling...';
      const btnLabel = btnStartStudio.querySelector('.btn-label-text');
      if (btnLabel) btnLabel.textContent = 'DOWNLOADING PACKAGES...';
      const progressWrap = document.getElementById('dock-progress-bar-wrap');
      if (progressWrap) progressWrap.classList.remove('hidden');

      sendIpc({ type: 'perform_studio_update' });
      return;
    }

    statusIndicatorDot.className = 'status-indicator-dot busy';
    statusDisplayText.textContent = 'Launching Studio under Wine...';

    sendIpc({
      type: 'play',
      place_id: appState.selectedPlaceId,
      fps: appState.fps,
      force_xwayland: settingWayland.checked
    });
  });

  // Top nav update badge click: re-check updates
  const badgeUpdateStatus = document.getElementById('badge-update-status');
  if (badgeUpdateStatus) {
    badgeUpdateStatus.addEventListener('click', () => {
      window.showToast?.('Checking for Roblox Studio & GitHub updates...', true);
      sendIpc({ type: 'check_update' });
    });
  }

  window.setUpdateAvailable = function (info) {
    if (!info) return;
    const btnLabel = btnStartStudio.querySelector('.btn-label-text');
    const updateBadge = document.getElementById('badge-update-status');
    const updateDot = document.getElementById('update-dot');
    const updateStatusText = document.getElementById('update-status-text');

    if (info.has_update) {
      appState.isUpdateAvailable = true;
      appState.latestVersion = info.latest_version;

      btnStartStudio.classList.add('btn-update-mode');
      if (btnLabel) {
        btnLabel.textContent = info.current_version ? 'UPDATE ROBLOX STUDIO' : 'DOWNLOAD & INSTALL STUDIO';
      }
      statusDisplayText.textContent = `New Studio version available: ${info.latest_version}`;
      statusIndicatorDot.className = 'status-indicator-dot busy';

      if (updateBadge && updateStatusText && updateDot) {
        updateBadge.style.display = 'inline-flex';
        updateDot.style.background = '#00D2FF';
        updateDot.style.boxShadow = '0 0 8px #00D2FF';
        updateStatusText.textContent = 'Studio Update Available';
      }
    } else {
      appState.isUpdateAvailable = false;
      btnStartStudio.classList.remove('btn-update-mode');
      if (btnLabel) {
        btnLabel.textContent = 'START ROBLOX STUDIO';
      }
      statusDisplayText.textContent = 'System Ready';
      statusIndicatorDot.className = 'status-indicator-dot';

      if (updateBadge && updateStatusText && updateDot) {
        updateBadge.style.display = 'inline-flex';
        updateDot.style.background = '#10B981';
        updateDot.style.boxShadow = '0 0 8px #10B981';
        if (info.launcher_update) {
          updateStatusText.textContent = `Launcher ${info.launcher_update} on GitHub`;
          window.showToast?.(`New RtRust Launcher ${info.launcher_update} available on GitHub!`, true);
        } else {
          updateStatusText.textContent = 'v0.0.1 • Up to Date';
        }
      }
    }
  };

  // --- State Updates from Rust Backend ---
  window.updateState = function (state) {
    if (!state) return;

    if (state.user) {
      appState.user = state.user;
      userDisplayName.textContent = state.user.display_name || state.user.username;
      userHandleName.textContent = '@' + (state.user.username || 'developer');
      if (state.user.avatar_url) {
        userAvatarBox.innerHTML = `<img src="${state.user.avatar_url}" alt="avatar" style="width:100%;height:100%;border-radius:50%;object-fit:cover;">`;
      } else {
        userAvatarBox.innerHTML = `<span id="user-avatar-initial">${(state.user.display_name || state.user.username || 'R')[0].toUpperCase()}</span>`;
      }
      showHub();
    } else {
      if (appState.user) {
        appState.user = null;
        showLogin();
      }
    }

    if (state.places && state.places.length > 0) {
      appState.places = state.places;
      if (!appState.selectedPlaceId) {
        const first = appState.places[0];
        selectPlace(first.place_id, first.name, first.description, first.thumbnail_url);
      }
      renderPlaces();
    }

    if (state.trending && state.trending.length > 0) {
      appState.trending = state.trending;
      renderTrending();
    }

    if (state.hardware) {
      appState.hardware = state.hardware;

      const hwRam = document.getElementById('hw-ram-val');
      const hwGpu = document.getElementById('hw-gpu-val');
      const hwCpu = document.getElementById('hw-cpu-val');
      const hwProfile = document.getElementById('hw-profile-val');
      const hwWine = document.getElementById('hw-wine-val');

      if (hwRam) hwRam.textContent = `${state.hardware.total_ram_mib} MiB (${Math.round(state.hardware.total_ram_mib / 1024)} GB)`;
      if (hwGpu) hwGpu.textContent = `${state.hardware.gpu} (${state.hardware.is_igpu ? 'iGPU Vulkan' : 'Discrete GPU'})`;
      if (hwCpu) hwCpu.textContent = `${state.hardware.cpu_cores} Logical Cores (Vulkan Async)`;
      if (hwWine) hwWine.textContent = state.hardware.wine_version || 'Kombucha Patched';

      // Onboarding specs update
      const obRam = document.getElementById('ob-ram');
      const obGpu = document.getElementById('ob-gpu');
      const obWine = document.getElementById('ob-wine');
      const obRecTitle = document.getElementById('ob-rec-title');
      const obRecDesc = document.getElementById('ob-rec-desc');

      if (obRam) obRam.textContent = `${state.hardware.total_ram_mib} MiB (${Math.round(state.hardware.total_ram_mib / 1024)} GB)`;
      if (obGpu) obGpu.textContent = `${state.hardware.gpu} (${state.hardware.is_igpu ? 'Integrated' : 'Discrete'})`;
      if (obWine) obWine.textContent = state.hardware.wine_version || 'Wine / Kombucha Ready';

      const rec = state.hardware.recommended_profile || 'auto';
      if (obRecTitle) obRecTitle.textContent = `${rec.toUpperCase()} PROFILE`;
      if (obRecDesc) obRecDesc.textContent = PROFILE_DESCRIPTIONS[rec] || '';
    }

    if (state.config) {
      appState.config = Object.assign(appState.config, state.config);
      appState.fps = state.config.target_fps || 60;

      // Update Profile selection pills
      const currentProfile = state.config.profile_mode || 'auto';
      profilePills.forEach(p => p.classList.toggle('active', p.dataset.profile === currentProfile));
      if (profileSubDesc) profileSubDesc.textContent = PROFILE_DESCRIPTIONS[currentProfile] || '';
      const hwProfile = document.getElementById('hw-profile-val');
      if (hwProfile) hwProfile.textContent = currentProfile.toUpperCase();

      // Granular controls
      settingGraphicsBackend.value = state.config.graphics_backend || 'auto';
      settingVramLimit.value = state.config.dxvk_max_vram_mb || 0;
      valVramLimit.textContent = state.config.dxvk_max_vram_mb ? `${state.config.dxvk_max_vram_mb} MB` : 'Unlimited';

      settingQualityLevel.value = state.config.editor_quality_level || 16;
      valQualityLevel.textContent = `Level ${state.config.editor_quality_level || 16}`;

      settingDisableShadows.checked = !!state.config.disable_shadows;
      settingBlockWebview2.checked = !!state.config.block_webview2;

      settingWayland.checked = state.config.force_xwayland_mouse !== false;
      appState.force_xwayland = settingWayland.checked;
      updateWaylandPillUI();

      settingThrottle.checked = state.config.throttle_3d_inactive !== false;
      settingPostfx.checked = state.config.disable_postfx !== false;
      settingVirtualDesktop.checked = !!state.config.virtual_desktop;
      settingAutohide.checked = state.config.auto_hide_launcher !== false;

      // Rust DXGI Shim & Resource Guard Settings
      if (settingUseDxgiShim) settingUseDxgiShim.checked = state.config.use_dxgi_shim !== false;
      if (settingAltTabGpuSleep) settingAltTabGpuSleep.checked = state.config.alt_tab_gpu_sleep !== false;
      if (settingVramCap) settingVramCap.value = state.config.vram_cap_mb !== undefined ? state.config.vram_cap_mb : 1024;
      if (settingCgroupRam) settingCgroupRam.value = state.config.cgroup_ram_limit_gb !== undefined ? state.config.cgroup_ram_limit_gb : 6;
      if (settingDisableMsaa) settingDisableMsaa.checked = state.config.disable_msaa !== false;
      if (settingForceVoxel) settingForceVoxel.checked = !!state.config.force_voxel_lighting;
      if (settingDisableGrass) settingDisableGrass.checked = state.config.disable_grass !== false;

      if (settingCustomRunner) settingCustomRunner.value = state.config.custom_runner_path || '';
      if (settingCustomEnv) settingCustomEnv.value = state.config.custom_env_vars || '';
      if (settingCustomFlags) settingCustomFlags.value = state.config.custom_cli_flags || '';

      // Update FPS pills in dock
      if (dockFpsSelector) {
        dockFpsSelector.querySelectorAll('.fps-pill-option').forEach(p => {
          p.classList.toggle('active', parseInt(p.dataset.fps, 10) === appState.fps);
        });
      }

      applyWallpaper();

      // First-run onboarding check
      if (state.config.first_run_completed === false) {
        modalOnboarding.classList.remove('hidden');
      }
    }

    if (state.status_text) {
      statusDisplayText.textContent = state.status_text;
    }

    if (state.is_running !== undefined) {
      if (state.is_running) {
        statusIndicatorDot.className = 'status-indicator-dot busy';
        statusDisplayText.textContent = 'Roblox Studio Running';
        btnStartStudio.disabled = true;
        btnStartStudio.style.opacity = '0.7';
      } else {
        statusIndicatorDot.className = 'status-indicator-dot';
        statusDisplayText.textContent = 'System Ready';
        btnStartStudio.disabled = false;
        btnStartStudio.style.opacity = '1';
        window.finishInstallProgress();
      }
    }

    // Update Live GPU Telemetry if available
    if (state.gpu_telemetry && telemetryBadge && telemetryText) {
      telemetryBadge.style.display = 'inline-flex';
      telemetryText.textContent = `${state.gpu_telemetry.fps} FPS • VRAM ${state.gpu_telemetry.vram_cap_mb}M`;
    }
  };

  // --- Vinegar-Style Installation Progress Modal Handlers ---
  window.updateInstallProgress = function (current, total, pkg) {
    const modal = document.getElementById('modal-install-progress');
    const pkgLabel = document.getElementById('install-current-pkg');
    const barFill = document.getElementById('install-progress-bar-fill');
    const textInfo = document.getElementById('install-progress-text');
    const dockProgressWrap = document.getElementById('dock-progress-bar-wrap');
    const dockProgressBar = document.getElementById('dock-progress-bar');
    const percent = Math.min(100, Math.round((current / total) * 100));

    if (dockProgressWrap) dockProgressWrap.classList.remove('hidden');
    if (dockProgressBar) dockProgressBar.style.width = `${percent}%`;
    if (statusDisplayText) statusDisplayText.textContent = `Downloading ${pkg} (${percent}%)...`;

    if (modal && pkgLabel && barFill && textInfo) {
      modal.classList.remove('hidden');
      pkgLabel.textContent = `[${current}/${total}] ${pkg}`;
      barFill.style.width = `${percent}%`;
      textInfo.textContent = `${current} / ${total} packages (${percent}%)`;
    }
  };

  window.finishInstallProgress = function () {
    const modal = document.getElementById('modal-install-progress');
    if (modal) modal.classList.add('hidden');
    const dockProgressWrap = document.getElementById('dock-progress-bar-wrap');
    if (dockProgressWrap) dockProgressWrap.classList.add('hidden');
  };

  // --- Toast Notification Handler ---
  window.showToast = function (message, isSuccess = true) {
    const container = document.getElementById('toast-container');
    if (!container) return;

    const toast = document.createElement('div');
    toast.className = `toast-item ${isSuccess ? 'success' : 'error'}`;
    toast.innerHTML = `
      <div class="toast-icon">
        ${isSuccess
          ? '<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>'
          : '<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>'
        }
      </div>
      <div class="toast-msg">${escapeHtml(message)}</div>
    `;

    container.appendChild(toast);

    requestAnimationFrame(() => toast.classList.add('show'));

    setTimeout(() => {
      toast.classList.remove('show');
      setTimeout(() => toast.remove(), 400);
    }, 4000);
  };

  function escapeHtml(str) {
    if (!str) return '';
    return str.replace(/[&<>"']/g, function (m) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#039;' }[m];
    });
  }

  // Quick Login Event Listeners & Handlers
  let quickLoginTimerInterval = null;
  function startQuickLoginCountdown(seconds) {
    if (quickLoginTimerInterval) clearInterval(quickLoginTimerInterval);
    let remaining = seconds;
    const update = () => {
      if (remaining <= 0) {
        clearInterval(quickLoginTimerInterval);
        if (quickTimer) quickTimer.textContent = 'Expired';
        if (quickStatusText) quickStatusText.textContent = 'Code expired. Click Refresh.';
        return;
      }
      const m = Math.floor(remaining / 60);
      const s = remaining % 60;
      if (quickTimer) quickTimer.textContent = `Expires in: ${m}:${s < 10 ? '0' : ''}${s}`;
      remaining--;
    };
    update();
    quickLoginTimerInterval = setInterval(update, 1000);
  }

  window.onQuickLoginCode = function(code, qrUrl, timeoutSec) {
    if (quickCodeDisplay) {
      quickCodeDisplay.textContent = code;
      quickCodeDisplay.classList.remove('loading');
    }
    if (quickStatusText) {
      quickStatusText.textContent = 'Waiting for authorization...';
    }
    startQuickLoginCountdown(timeoutSec || 300);
  };

  window.onQuickLoginError = function(errMsg) {
    if (quickCodeDisplay) {
      quickCodeDisplay.textContent = 'RETRY';
      quickCodeDisplay.classList.remove('loading');
    }
    if (quickStatusText) {
      quickStatusText.textContent = 'Failed to fetch code. Click Refresh.';
    }
  };

  if (quickCodeDisplay) {
    quickCodeDisplay.addEventListener('click', () => {
      if (quickCodeDisplay.textContent === 'RETRY' || quickCodeDisplay.textContent === '------') {
        quickCodeDisplay.textContent = '...';
        if (quickStatusText) quickStatusText.textContent = 'Generating code...';
        sendIpc({ type: 'start_quick_login' });
      }
    });
  }

  // Request initial state from backend
  sendIpc({ type: 'init' });
  sendIpc({ type: 'start_quick_login' });
})();
