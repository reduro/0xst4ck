import React, { useState, useEffect, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Home,
  Layers,
  Play,
  Plus,
  Settings as SettingsIcon,
  Search,
  Trash2,
  Clock,
  FolderOpen,
  X,
  FileCode,
  Download,
  UploadCloud,
  Check,
  RefreshCw,
  CheckSquare,
  Square,
  ArrowLeft,
  Gamepad2
} from "lucide-react";
import { Game, Settings, SteamSearchResult, DllSyncResult, NetworkRuntime, LuaConfigFile, UpdateInfo } from "./types";
import { translations, Language } from "./i18n";
import "./App.css";

export default function App() {
  const [games, setGames] = useState<Game[]>([]);
  const [settings, setSettings] = useState<Settings>({
    language: "fr",
    theme: "dark",
    steam_path: "",
    default_launch_mode: "spacewar",
    auto_unpack: true,
    auto_copy_hooks: true
  });

  const lang = (settings.language as Language) || "fr";
  const t = translations[lang] || translations.fr;
  const [activeTab, setActiveTab] = useState<"home" | "library" | "generate" | "settings">("home");
  const [searchQuery, setSearchQuery] = useState("");
  const [filterMode, setFilterMode] = useState<"all" | "recent" | "photon" | "eos">("all");
  const [bannerImages, setBannerImages] = useState<Record<string, string>>({});
  const [, setLoading] = useState(true);



  // DLL Verification & Sync State
  const [, setDllStatus] = useState<DllSyncResult | null>(null);
  const [, setIsSyncingDlls] = useState(false);

  // Multi-Selection State
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  // Auto-Updater State
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [isUpdating, setIsUpdating] = useState(false);
  const [updateBannerDismissed, setUpdateBannerDismissed] = useState(false);

  // Steam Auto-Import & Path State
  const [isImportingSteam, setIsImportingSteam] = useState(false);
  const [toastNotice, setToastNotice] = useState<string | null>(null);
  const [detectedSteamPath, setDetectedSteamPath] = useState("");
  const [manifestServer, setManifestServer] = useState("steamrun");

  // Inspect Game Details View
  const [inspectGame, setInspectGame] = useState<Game | null>(null);
  const [editingPathValue, setEditingPathValue] = useState("");
  const [pathSaveSuccess, setPathSaveSuccess] = useState(false);
  const [editNetworkMode, setEditNetworkMode] = useState(false);
  const [latestManifest, setLatestManifest] = useState<string | null>(null);
  const [checkingManifest, setCheckingManifest] = useState(false);

  // Lua / Manifest Config State
  const [luaConfig, setLuaConfig] = useState<LuaConfigFile | null>(null);
  const [luaDepotId, setLuaDepotId] = useState("");
  const [luaManifestId, setLuaManifestId] = useState("");
  const [luaSaving, setLuaSaving] = useState(false);
  const [luaEditMode, setLuaEditMode] = useState(false);
  const [luaRawContent, setLuaRawContent] = useState("");

  // Dynamic Launch State
  const [launchStep, setLaunchStep] = useState<string | null>(null);
  const [launchingGameId, setLaunchingGameId] = useState<string | null>(null);

  // Modal Add Game
  const [showModal, setShowModal] = useState(false);
  const [formName, setFormName] = useState("");
  const [formPath, setFormPath] = useState("");
  const [formArgs, setFormArgs] = useState("");
  const [formAppId, setFormAppId] = useState("");
  const [formBannerUrl, setFormBannerUrl] = useState("");
  const [steamResults, setSteamResults] = useState<SteamSearchResult[]>([]);
  const [, setSearchingSteam] = useState(false);

  // Steam Generator Page State
  const [genAppId, setGenAppId] = useState("");
  const [genSearchResults, setGenSearchResults] = useState<SteamSearchResult[]>([]);
  const [genSearching, setGenSearching] = useState(false);
  const [genCustomLua, setGenCustomLua] = useState<string | null>(null);
  const [genFileName, setGenFileName] = useState<string | null>(null);
  const [genGenerating, setGenGenerating] = useState(false);
  const [genOutput, setGenOutput] = useState<string | null>(null);
  const [genError, setGenError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    loadAll();
    loadSteamConfig();
    checkDllIntegrity();
  }, []);

  const checkDllIntegrity = async () => {
    try {
      setIsSyncingDlls(true);
      const res = await invoke<DllSyncResult>("verify_and_sync_dlls");
      setDllStatus(res);
      if (res.items.some((i) => i.status === "updated")) {
        setToastNotice("DLLs réinstallées et synchronisées avec succès.");
        setTimeout(() => setToastNotice(null), 3500);
      }
    } catch (err) {
      console.error("DLL verification error:", err);
    } finally {
      setIsSyncingDlls(false);
    }
  };

  // Keyboard Shortcuts: Ctrl+A to select all in Library, Esc to cancel/close
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
        if (activeTab === "library" && !showModal && !inspectGame) {
          e.preventDefault();
          const allIds = new Set(filteredGames.map((g) => g.id));
          setSelectedIds(allIds);
        }
      } else if (e.key === "Escape") {
        if (inspectGame) {
          setInspectGame(null);
        } else if (selectedIds.size > 0) {
          setSelectedIds(new Set());
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [activeTab, showModal, inspectGame, games, searchQuery]);

  const loadSteamConfig = async () => {
    try {
      const [path, server] = await Promise.all([
        invoke<string>("get_detected_steam_path"),
        invoke<string>("get_opensteamtool_server")
      ]);
      setDetectedSteamPath(path);
      setManifestServer(server);
    } catch {
      // ignore
    }
  };

  const loadAll = async () => {
    try {
      setLoading(true);
      const [gamesList, loadedSettings] = await Promise.all([
        invoke<Game[]>("get_games"),
        invoke<Settings>("get_settings")
      ]);
      setGames(gamesList);
      setSettings(loadedSettings);

      const banners: Record<string, string> = {};
      await Promise.all(
        gamesList.map(async (g) => {
          if (g.banner) {
            try {
              const b64 = await invoke<string>("get_banner_image", { filename: g.banner });
              banners[g.banner] = b64;
            } catch {
              // ignore
            }
          }
        })
      );
      setBannerImages(banners);
    } catch (err) {
      console.error("Failed to load launcher state:", err);
    } finally {
      setLoading(false);
    }
  };

  // AUTOMATIC STEAM IMPORT
  const handleImportSteam = async () => {
    try {
      setIsImportingSteam(true);
      setToastNotice("Analyse des bibliothèques Steam en cours...");
      const count = await invoke<number>("import_steam_games");
      if (count > 0) {
        setToastNotice(`${count} nouveau(x) jeu(x) importé(s) avec succès.`);
      } else {
        setToastNotice("Tous vos jeux Steam sont déjà synchronisés.");
      }
      await loadAll();
    } catch (err: any) {
      setToastNotice(`Erreur d'import : ${err}`);
    } finally {
      setIsImportingSteam(false);
      setTimeout(() => setToastNotice(null), 3500);
    }
  };

  // DYNAMIC LAUNCH
  const handleLaunchGame = async (game: Game, forcedMode?: string) => {
    const finalMode = forcedMode || settings.default_launch_mode;
    setLaunchingGameId(game.id);

    try {
      setLaunchStep("Analyse des dépendances réseau et runtime...");
      await new Promise((r) => setTimeout(r, 120));

      setLaunchStep("Injection Spacewar 480 & Steam Overlay...");

      const res = await invoke<string>("launch_game", { id: game.id, mode: finalMode });
      setLaunchStep(res);
      setTimeout(() => setLaunchStep(null), 4000);

      loadAll();
    } catch (err: any) {
      setLaunchStep(`Erreur lancement : ${err}`);
      setTimeout(() => setLaunchStep(null), 5000);
    } finally {
      setLaunchingGameId(null);
    }
  };

  const handleOpenFolder = async (path: string, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    try {
      await invoke("open_game_folder", { path });
    } catch (err) {
      console.error("Failed to open game folder:", err);
    }
  };

  // CHANGE GAME PATH
  
  const handleSetNetworkMode = async (mode: "photon" | "eos" | "spacewar") => {
    if (!inspectGame) return;
    const isPhoton = mode === "photon";
    const isEos = mode === "eos";
    const updated: Game = {
      ...inspectGame,
      requires_photon: isPhoton,
      network_runtime: {
        runtime: isPhoton ? "photon" : isEos ? "eos" : "steamworks",
        photon: isPhoton,
        eos: isEos,
        steamworks: true,
        requirements_summary: isPhoton ? "Photon P2P" : isEos ? "Epic Online Services" : "Steam 480",
        evidence: [],
        scanned: true,
      }
    };
    try {
      await invoke("update_game", { game: updated });
      setInspectGame(updated);
      setGames((prev) => prev.map((g) => (g.id === updated.id ? updated : g)));
      setToastNotice("Mode réseau mis à jour.");
      setTimeout(() => setToastNotice(null), 3000);
      setEditNetworkMode(false);
    } catch (err) {
      console.error("Failed to update network mode:", err);
    }
  };

  const handleSaveChangedPath = async () => {
    if (!inspectGame || !editingPathValue.trim()) return;
    try {
      const updated = await invoke<Game>("change_game_path", {
        id: inspectGame.id,
        newPath: editingPathValue.trim()
      });
      setInspectGame(updated);
      setPathSaveSuccess(true);
      setTimeout(() => setPathSaveSuccess(false), 2500);
      await loadAll();
    } catch (err) {
      console.error("Failed to update game path:", err);
    }
  };

  // SELECTION HANDLERS
  const handleToggleSelect = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    const next = new Set(selectedIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    setSelectedIds(next);
  };

  const handleSelectAll = () => {
    const all = new Set(filteredGames.map((g) => g.id));
    setSelectedIds(all);
  };

  const handleClearSelection = () => {
    setSelectedIds(new Set());
  };

  const handleDeleteSelected = async () => {
    if (selectedIds.size === 0) return;
    const confirm = window.confirm(`Voulez-vous supprimer ${selectedIds.size} jeu(x) de votre bibliothèque ?`);
    if (!confirm) return;

    try {
      await invoke("delete_games", { ids: Array.from(selectedIds) });
      setSelectedIds(new Set());
      await loadAll();
      setToastNotice("Jeux supprimés de la bibliothèque.");
      setTimeout(() => setToastNotice(null), 3000);
    } catch (err) {
      console.error("Delete error:", err);
    }
  };

  const handleDeleteSingle = async (id: string) => {
    if (!window.confirm("Supprimer ce jeu de la bibliothèque ?")) return;
    try {
      await invoke("delete_game", { id });
      setInspectGame(null);
      await loadAll();
    } catch (err) {
      console.error("Delete error:", err);
    }
  };

  // MODAL ADD
  const handleOpenAddModal = () => {
    setFormName("");
    setFormPath("");
    setFormArgs("");
    setFormAppId("");
    setFormBannerUrl("");
    setSteamResults([]);
    setShowModal(true);
  };

  const handleSearchSteam = async (val: string) => {
    setFormName(val);
    if (val.trim().length < 2) {
      setSteamResults([]);
      return;
    }
    try {
      setSearchingSteam(true);
      const res = await invoke<SteamSearchResult[]>("search_steam_games", { query: val });
      setSteamResults(res.slice(0, 5));
    } catch {
      // ignore
    } finally {
      setSearchingSteam(false);
    }
  };

  const handleSelectSteamResult = (sr: SteamSearchResult) => {
    setFormName(sr.name);
    setFormAppId(sr.appid.toString());
    setFormBannerUrl(sr.header_image);
    setSteamResults([]);
  };

  const handleSaveModal = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formName.trim() || !formPath.trim()) return;

    try {
      await invoke("add_game", {
        name: formName.trim(),
        path: formPath.trim(),
        arguments: formArgs.trim() || null,
        steamAppid: formAppId.trim() || null,
        bannerUrl: formBannerUrl.trim() || null
      });
      setShowModal(false);
      await loadAll();
    } catch (err: any) {
      alert(`Erreur : ${err}`);
    }
  };

  // SETTINGS HANDLERS
  const handleSaveSettings = async (newSettings: Settings) => {
    setSettings(newSettings);
    try {
      await invoke("save_settings", { settings: newSettings });
    } catch (err) {
      console.error("Failed to save settings:", err);
    }
  };

  const handleChangeManifestServer = async (server: string) => {
    try {
      setManifestServer(server);
      await invoke("set_opensteamtool_server", { server });
      setToastNotice(`Serveur de manifest basculé sur ${server}`);
      setTimeout(() => setToastNotice(null), 3000);
    } catch (err) {
      console.error("Failed to update manifest server:", err);
    }
  };

  // STEAM GENERATOR
  const extractSteamAppId = (val: string): string | null => {
    const trimmed = val.trim();
    const urlMatch = trimmed.match(/(?:\/app\/|\/apps\/|store\/)(\d+)/i);
    if (urlMatch && urlMatch[1]) return urlMatch[1];
    const parenMatch = trimmed.match(/\((\d+)\)$/);
    if (parenMatch && parenMatch[1]) return parenMatch[1];
    if (/^\d+$/.test(trimmed)) return trimmed;
    return null;
  };

  const runGenerateLua = async (overrideInput?: string) => {
    const raw = (overrideInput !== undefined ? overrideInput : genAppId).trim();
    if (!raw) return;
    const target = extractSteamAppId(raw) || raw;
    setGenGenerating(true);
    setGenOutput(null);
    setGenError(null);
    setGenSearchResults([]);

    try {
      const res = await invoke<string>("generate_steam_game", {
        appid: target,
        appidOrName: target,
        customLua: genCustomLua
      });
      setGenOutput(res);
    } catch (err: any) {
      setGenError(err.toString());
    } finally {
      setGenGenerating(false);
    }
  };

  const handleGenerateLua = () => {
    runGenerateLua();
  };

  const handleGenInputChange = async (val: string) => {
    setGenAppId(val);
    const trimmed = val.trim();
    if (trimmed.length < 2) {
      setGenSearchResults([]);
      return;
    }

    // If user pasted a direct Steam link, extract AppID and trigger generation + lookup
    const linkMatch = trimmed.match(/(?:\/app\/|\/apps\/|store\/)(\d+)/i);
    if (linkMatch && linkMatch[1]) {
      const extractedId = linkMatch[1];
      runGenerateLua(extractedId);
      return;
    }

    try {
      setGenSearching(true);
      const res = await invoke<SteamSearchResult[]>("search_steam_games", { query: trimmed });
      setGenSearchResults(res.slice(0, 6));
    } catch {
      // ignore
    } finally {
      setGenSearching(false);
    }
  };

  const handleSelectGenResult = (sr: SteamSearchResult) => {
    setGenAppId(`${sr.name} (${sr.appid})`);
    setGenSearchResults([]);
    runGenerateLua(sr.appid.toString());
  };

  const handleFileDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      processLuaFile(e.dataTransfer.files[0]);
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      processLuaFile(e.target.files[0]);
    }
  };

  const processLuaFile = (file: File) => {
    if (!file.name.endsWith(".lua")) {
      alert("Seuls les fichiers .lua sont acceptés");
      return;
    }
    setGenFileName(file.name);
    const id = file.name.replace(".lua", "");
    if (!genAppId) {
      setGenAppId(id);
    }
    const reader = new FileReader();
    reader.onload = (event) => {
      const content = event.target?.result as string;
      setGenCustomLua(content);
    };
    reader.readAsText(file);
  };

  
  const handleCheckLatestManifest = async (targetDepot?: string) => {
    const depot = targetDepot || luaDepotId || inspectGame?.steam_appid;
    if (!depot) return;
    try {
      setCheckingManifest(true);
      const res = await fetch(`https://manifest.steam.run/api/manifest/${depot.trim()}`);
      if (res.ok) {
        const data = await res.json();
        if (data && data.content) {
          setLatestManifest(data.content);
          setLuaManifestId(data.content);
        } else {
          setToastNotice("Aucune mise à jour trouvée sur SteamRun pour ce dépôt.");
          setTimeout(() => setToastNotice(null), 3000);
        }
      }
    } catch (err) {
      console.warn("Error fetching latest manifest:", err);
    } finally {
      setCheckingManifest(false);
    }
  };

  const handleOpenDrawer = async (game: Game) => {
    setInspectGame(game);
    setEditingPathValue(game.path);
    setPathSaveSuccess(false);
    setLuaConfig(null);
    setLuaDepotId("");
    setLuaManifestId("");
        setLuaEditMode(false);
    setLatestManifest(null);

    if (game.steam_appid) {
      try {
        const cfg = await invoke<LuaConfigFile>("get_game_lua", { appid: game.steam_appid });
        setLuaConfig(cfg);
        setLuaRawContent(cfg.content);
        // Pre-fill depot and manifest if present in content
        const m = cfg.content.match(/setManifestid\(\s*(\d+)\s*,\s*["'](\d+)["']/);
        if (m) {
          setLuaDepotId(m[1]);
          setLuaManifestId(m[2]);
        } else {
          // Check addappid depot
          const d = cfg.content.match(/addappid\(\s*(\d+)\s*,\s*1/);
          if (d) setLuaDepotId(d[1]);
        }
      } catch (err) {
        console.warn("Could not load lua config:", err);
      }
    }
  };

  const handleUpdateManifest = async () => {
    if (!inspectGame?.steam_appid || !luaDepotId.trim() || !luaManifestId.trim()) {
      setToastNotice("Veuillez renseigner le DepotID et le ManifestID.");
      setTimeout(() => setToastNotice(null), 3000);
      return;
    }
    try {
      setLuaSaving(true);
      const res = await invoke<string>("set_game_manifest", {
        appid: inspectGame.steam_appid,
        depotId: luaDepotId.trim(),
        manifestId: luaManifestId.trim()
      });
      setToastNotice(res);
      setTimeout(() => setToastNotice(null), 4000);
      // Reload lua config
      const cfg = await invoke<LuaConfigFile>("get_game_lua", { appid: inspectGame.steam_appid });
      setLuaConfig(cfg);
      setLuaRawContent(cfg.content);
    } catch (err) {
      console.error("Save manifest error:", err);
      setToastNotice(`Erreur : ${err}`);
      setTimeout(() => setToastNotice(null), 4000);
    } finally {
      setLuaSaving(false);
    }
  };

  const handleSaveRawLua = async () => {
    if (!luaConfig) return;
    try {
      setLuaSaving(true);
      await invoke("save_game_lua", {
        filePath: luaConfig.file_path,
        content: luaRawContent
      });
      setToastNotice("Fichier Lua enregistré avec succès.");
      setTimeout(() => setToastNotice(null), 3000);
          setLuaEditMode(false);
    setLatestManifest(null);
      const cfg = await invoke<LuaConfigFile>("get_game_lua", { appid: inspectGame!.steam_appid! });
      setLuaConfig(cfg);
    } catch (err) {
      console.error("Save lua error:", err);
      setToastNotice(`Erreur : ${err}`);
      setTimeout(() => setToastNotice(null), 4000);
    } finally {
      setLuaSaving(false);
    }
  };

  // Updater check on mount
  useEffect(() => {
    checkForUpdates(false);
  }, []);

  const checkForUpdates = async (showNotice = true) => {
    try {
      setIsCheckingUpdate(true);
      const info = await invoke<UpdateInfo>("check_launcher_update", {
        repo: settings.github_repo || "reduro/0xst4ck"
      });
      setUpdateInfo(info);
      if (info.update_available) {
        setUpdateBannerDismissed(false);
      } else if (showNotice) {
        setToastNotice(t.settings.upToDateMsg);
        setTimeout(() => setToastNotice(null), 3000);
      }
    } catch (err) {
      if (showNotice) {
        setToastNotice(`Erreur vérification MAJ : ${err}`);
        setTimeout(() => setToastNotice(null), 4000);
      }
    } finally {
      setIsCheckingUpdate(false);
    }
  };

  const handleInstallUpdate = async () => {
    if (!updateInfo?.download_url) return;
    try {
      setIsUpdating(true);
      setToastNotice(t.updater.downloadingUpdate);
      await invoke("download_launcher_update", {
        downloadUrl: updateInfo.download_url
      });
      setToastNotice(t.updater.updateLaunched);
      setTimeout(async () => {
        await invoke("exit_launcher");
      }, 1200);
    } catch (err) {
      setIsUpdating(false);
      setToastNotice(`Erreur installation MAJ : ${err}`);
      setTimeout(() => setToastNotice(null), 4000);
    }
  };

  const handleRescanRuntime = async (game: Game) => {
    try {
      setToastNotice("Analyse approfondie des dépendances en cours...");
      const runtime = await invoke<NetworkRuntime>("scan_game_runtime", { path: game.path });
      const updatedGame: Game = {
        ...game,
        requires_photon: runtime.photon,
        network_runtime: runtime
      };
      await invoke("update_game", { game: updatedGame });
      setInspectGame(updatedGame);
      await loadAll();
      setToastNotice(`Diagnostic mis à jour : ${runtime.requirements_summary}`);
      setTimeout(() => setToastNotice(null), 3500);
    } catch (err) {
      console.error("Rescan failed:", err);
    }
  };

  // Metrics
  const totalPlaytimeSeconds = games.reduce((acc, g) => acc + (g.playtime || 0), 0);
  const totalPlaytimeHours = (totalPlaytimeSeconds / 3600).toFixed(1);

  const recentGames = useMemo(() => {
    return [...games]
      .filter((g) => (g.last_played || 0) > 0)
      .sort((a, b) => b.last_played - a.last_played)
      .slice(0, 8);
  }, [games]);

  const photonGames = useMemo(() => {
    return games.filter((g) => g.requires_photon || g.network_runtime?.photon);
  }, [games]);

  const eosGames = useMemo(() => {
    return games.filter((g) => g.network_runtime?.eos);
  }, [games]);

  const filteredGames = useMemo(() => {
    return games
      .filter((g) => {
        const matchesSearch = g.name.toLowerCase().includes(searchQuery.toLowerCase());
        if (!matchesSearch) return false;
        if (filterMode === "recent") return (g.last_played || 0) > 0;
        if (filterMode === "photon") return g.requires_photon || g.network_runtime?.photon;
        if (filterMode === "eos") return g.network_runtime?.eos;
        return true;
      })
      .sort((a, b) => {
        if (filterMode === "recent") return (b.last_played || 0) - (a.last_played || 0);
        return a.name.localeCompare(b.name);
      });
  }, [games, searchQuery, filterMode]);

  const formatPlaytime = (seconds: number) => {
    if (!seconds) return "0 min";
    if (seconds < 3600) return `${Math.round(seconds / 60)} min`;
    const h = (seconds / 3600).toFixed(1);
    return `${h} h`;
  };

  return (
    <div className="launcher-root" style={{ flexDirection: "column" }}>
      {/* GITHUB AUTO-UPDATE BANNER */}
      {updateInfo?.update_available && !updateBannerDismissed && (
        <div className="update-banner">
          <div className="update-banner-text">
            <span>🚀</span>
            <strong>{t.updater.bannerNotice}</strong>
            <span className="update-version-tag">v{updateInfo.latest_version}</span>
            {updateInfo.release_name && <span className="update-rel-name">({updateInfo.release_name})</span>}
          </div>
          <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
            <button
              type="button"
              className="btn-update-action"
              onClick={handleInstallUpdate}
              disabled={isUpdating}
            >
              <Download size={14} />
              <span>{isUpdating ? t.updater.downloadingUpdate : t.updater.bannerBtn}</span>
            </button>
            <button
              type="button"
              className="btn-update-close"
              onClick={() => setUpdateBannerDismissed(true)}
            >
              <X size={15} />
            </button>
          </div>
        </div>
      )}

      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
      {/* SIDEBAR */}
      <aside className="sidebar">
        <div className="brand">
          <img src="/favicon.ico" alt="Logo" className="brand-logo" />
          <div>
            <div className="brand-name">Clemtout</div>
            <div className="brand-sub">Launcher v2.0</div>
          </div>
        </div>

        <nav className="nav">
          <button
            className={`nav-item ${activeTab === "home" && !inspectGame ? "active" : ""}`}
            onClick={() => {
              setInspectGame(null);
              setActiveTab("home");
            }}
          >
            <Home size={18} />
            <span>{t.nav.home}</span>
          </button>

          <button
            className={`nav-item ${activeTab === "library" && !inspectGame ? "active" : ""}`}
            onClick={() => {
              setInspectGame(null);
              setActiveTab("library");
            }}
          >
            <Layers size={18} />
            <span>{t.nav.library}</span>
            <span className="nav-badge">{games.length}</span>
          </button>



          <button
            className={`nav-item ${activeTab === "generate" && !inspectGame ? "active" : ""}`}
            onClick={() => {
              setInspectGame(null);
              setActiveTab("generate");
            }}
          >
            <FileCode size={18} />
            <span>{t.nav.generate}</span>
          </button>

          <button
            className={`nav-item ${activeTab === "settings" && !inspectGame ? "active" : ""}`}
            onClick={() => {
              setInspectGame(null);
              setActiveTab("settings");
            }}
          >
            <SettingsIcon size={18} />
            <span>{t.nav.settings}</span>
          </button>
        </nav>

        
      </aside>

      {/* MAIN VIEW */}
      <main className="main">
        {/* TOPBAR */}
        <header className="topbar">
          <div className="topbar-title">
            {inspectGame ? (
              <span className="details-nav-title">{t.topbar.gameDetails}</span>
            ) : (
              <>
                {activeTab === "home" && t.topbar.welcome}
                {activeTab === "library" && t.topbar.library}
                {activeTab === "generate" && t.topbar.generate}
                {activeTab === "settings" && t.topbar.settings}
              </>
            )}
          </div>

          </header>

        {/* TOAST NOTIFICATION */}
        {(launchStep || toastNotice) && (
          <div className="status-toast">
            {launchStep && <div className="toast-spinner"></div>}
            <span>{launchStep || toastNotice}</span>
          </div>
        )}

        {/* CONTENT PAGES */}
        <div className="content">
          {/* VIEW: GAME DETAILS (FULL CLEAN PAGE LIKE OLD LAUNCHER) */}
          {inspectGame ? (
            <div className="page active details-view">
              <button className="btn-back" onClick={() => setInspectGame(null)}>
                <ArrowLeft size={16} />
                <span>{t.actions.back}</span>
              </button>

              <h1 className="details-heading">{inspectGame.name}</h1>

              <div className="details-columns">
                {/* LEFT COLUMN: BANNER & PLAY */}
                <div className="details-col-left">
                  <div className="details-banner-box">
                    {inspectGame.banner && bannerImages[inspectGame.banner] ? (
                      <img src={bannerImages[inspectGame.banner]} alt={inspectGame.name} />
                    ) : (
                      <div className="details-banner-fallback">
                        <Gamepad2 size={48} />
                        <span>{inspectGame.name}</span>
                      </div>
                    )}
                  </div>

                  <div className="details-launch-buttons">
                    <button
                      className="btn-launch-primary"
                      onClick={() => handleLaunchGame(inspectGame, "spacewar")}
                      disabled={launchingGameId === inspectGame.id}
                      style={{ width: "100%", justifyContent: "center", padding: "14px 24px", fontSize: "16px" }}
                    >
                      <Play fill="white" size={20} />
                      <span>{t.actions.play}</span>
                    </button>
                  </div>

                  <div className="details-stat-card">
                    <Clock size={20} className="stat-clock-icon" />
                    <div>
                      <div className="stat-card-label">{t.gameDetails.totalPlaytime}</div>
                      <div className="stat-card-value">{formatPlaytime(inspectGame.playtime)}</div>
                    </div>
                  </div>
                </div>

                {/* RIGHT COLUMN: PATH & DIAGNOSTICS */}
                <div className="details-col-right">
                  <div className="details-card">
                    <h3>{t.gameDetails.location}</h3>
                    <div className="path-editor-box">
                      <input
                        type="text"
                        value={editingPathValue}
                        onChange={(e) => setEditingPathValue(e.target.value)}
                        placeholder="C:\Jeux\Game.exe"
                      />
                      <button
                        className="btn-save-path"
                        onClick={handleSaveChangedPath}
                      >
                        {pathSaveSuccess ? <Check size={16} /> : t.actions.save}
                      </button>
                    </div>

                    <div className="path-buttons-row">
                      <button
                        className="btn-toolbar"
                        onClick={(e) => handleOpenFolder(inspectGame.path, e)}
                      >
                        <FolderOpen size={15} />
                        <span>{t.actions.openFolder}</span>
                      </button>
                    </div>
                  </div>

                  <div className="details-card">
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <h3>{t.gameDetails.networkConfig}</h3>
                      <div style={{ display: "flex", gap: "8px" }}>
                        <button
                          className="btn-toolbar"
                          style={{ padding: "5px 10px", fontSize: "12px" }}
                          onClick={() => handleRescanRuntime(inspectGame)}
                          title={t.actions.rescan}
                        >
                          <RefreshCw size={12} />
                          <span>{t.actions.rescan}</span>
                        </button>
                        <button
                          className="btn-toolbar"
                          style={{ padding: "5px 12px", fontSize: "12px" }}
                          onClick={() => setEditNetworkMode(!editNetworkMode)}
                        >
                          <span>{editNetworkMode ? t.actions.close : t.actions.modify}</span>
                        </button>
                      </div>
                    </div>

                    <div style={{ marginTop: "12px", display: "flex", alignItems: "center", gap: "10px" }}>
                      <span style={{ fontSize: "13px", color: "var(--text-muted)" }}>{t.gameDetails.appliedNetwork}</span>
                      <span className={`runtime-tag ${inspectGame.requires_photon || inspectGame.network_runtime?.photon ? "photon-tag" : inspectGame.network_runtime?.eos ? "eos-tag" : "spacewar-tag"}`}>
                        {inspectGame.requires_photon || inspectGame.network_runtime?.photon
                          ? "Photon P2P"
                          : inspectGame.network_runtime?.eos
                          ? "Epic Games (EOS)"
                          : "Standard Steam 480"}
                      </span>
                    </div>

                    {editNetworkMode && (
                      <div style={{ marginTop: "14px", display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "10px" }}>
                        <button
                          className={`btn-network-choice ${inspectGame.requires_photon || inspectGame.network_runtime?.photon ? "active" : ""}`}
                          onClick={() => handleSetNetworkMode("photon")}
                        >
                          <strong>Photon P2P</strong>
                          <span>{t.gameDetails.photonDesc}</span>
                        </button>
                        <button
                          className={`btn-network-choice ${inspectGame.network_runtime?.eos ? "active" : ""}`}
                          onClick={() => handleSetNetworkMode("eos")}
                        >
                          <strong>Epic Games</strong>
                          <span>{t.gameDetails.eosDesc}</span>
                        </button>
                        <button
                          className={`btn-network-choice ${!inspectGame.requires_photon && !inspectGame.network_runtime?.photon && !inspectGame.network_runtime?.eos ? "active" : ""}`}
                          onClick={() => handleSetNetworkMode("spacewar")}
                        >
                          <strong>Standard 480</strong>
                          <span>{t.gameDetails.spacewarDesc}</span>
                        </button>
                      </div>
                    )}
                  </div>

                  {/* LUA & MANIFEST CONFIGURATION */}
                  <div className="details-card">
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <h3>{t.gameDetails.updatesTitle}</h3>
                      <button
                        className="btn-toolbar"
                        style={{ padding: "4px 10px", fontSize: "12px" }}
                        onClick={() => setLuaEditMode(!luaEditMode)}
                      >
                        <FileCode size={13} />
                        <span>{luaEditMode ? t.gameDetails.simpleMode : t.gameDetails.editRawLua}</span>
                      </button>
                    </div>

                    {luaConfig ? (
                      <div style={{ marginTop: "12px" }}>
                        <div style={{ fontSize: "12px", color: "var(--text-muted)", marginBottom: "12px", wordBreak: "break-all" }}>
                          Fichier : <code>{luaConfig.file_path}</code>
                        </div>

                        {!luaEditMode ? (
                          <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
                            <div style={{ display: "grid", gridTemplateColumns: "1fr 2fr", gap: "10px" }}>
                              <div>
                                <label style={{ fontSize: "12px", color: "var(--text-muted)", display: "block", marginBottom: "4px" }}>
                                  Depot ID
                                </label>
                                <input
                                  type="text"
                                  className="form-input"
                                  style={{ width: "100%", padding: "8px", background: "rgba(0,0,0,0.3)", border: "1px solid var(--border)", borderRadius: "6px", color: "white" }}
                                  placeholder="ex: 4954911"
                                  value={luaDepotId}
                                  onChange={(e) => setLuaDepotId(e.target.value)}
                                />
                              </div>
                              <div>
                                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "4px" }}>
                                  <label style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                                    Manifest ID (Version / MAJ)
                                  </label>
                                  <button
                                    type="button"
                                    onClick={() => handleCheckLatestManifest()}
                                    disabled={checkingManifest || !luaDepotId.trim()}
                                    style={{
                                      background: "transparent",
                                      border: "none",
                                      color: "#38bdf8",
                                      fontSize: "11px",
                                      cursor: "pointer",
                                      display: "flex",
                                      alignItems: "center",
                                      gap: "4px",
                                      padding: "0 2px"
                                    }}
                                  >
                                    <RefreshCw size={11} className={checkingManifest ? "animate-spin" : ""} />
                                    <span>{t.gameDetails.checkLatest}</span>
                                  </button>
                                </div>
                                <input
                                  type="text"
                                  className="form-input"
                                  style={{ width: "100%", padding: "8px", background: "rgba(0,0,0,0.3)", border: "1px solid var(--border)", borderRadius: "6px", color: "white" }}
                                  placeholder="ex: 7350154011825042241"
                                  value={luaManifestId}
                                  onChange={(e) => setLuaManifestId(e.target.value)}
                                />
                              </div>
                            </div>

                            {latestManifest && (
                              <div style={{ background: "rgba(56, 189, 248, 0.1)", border: "1px solid rgba(56, 189, 248, 0.3)", borderRadius: "6px", padding: "8px 12px", fontSize: "12px", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                                <span>{t.gameDetails.latestFound} <strong>{latestManifest}</strong></span>
                                {luaManifestId !== latestManifest ? (
                                  <button
                                    type="button"
                                    className="btn-toolbar"
                                    style={{ padding: "4px 10px", fontSize: "11px", background: "#10b981", color: "white" }}
                                    onClick={() => setLuaManifestId(latestManifest)}
                                  >
                                    {t.gameDetails.installVersion}
                                  </button>
                                ) : (
                                  <span style={{ color: "#10b981" }}>À jour</span>
                                )}
                              </div>
                            )}

                            <button
                              className="btn-toolbar"
                              style={{ alignSelf: "flex-start", padding: "8px 16px", background: "#3b82f6", color: "white" }}
                              onClick={handleUpdateManifest}
                              disabled={luaSaving}
                            >
                              <Check size={15} />
                              <span>{luaSaving ? t.gameDetails.saving : t.gameDetails.applyUpdate}</span>
                            </button>
                          </div>
                        ) : (
                          <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
                            <textarea
                              rows={10}
                              style={{
                                width: "100%",
                                padding: "10px",
                                background: "#0d1117",
                                color: "#58a6ff",
                                fontFamily: "monospace",
                                fontSize: "12px",
                                border: "1px solid var(--border)",
                                borderRadius: "6px",
                                resize: "vertical"
                              }}
                              value={luaRawContent}
                              onChange={(e) => setLuaRawContent(e.target.value)}
                            />
                            <button
                              className="btn-toolbar"
                              style={{ alignSelf: "flex-start", padding: "8px 16px", background: "#10b981", color: "white" }}
                              onClick={handleSaveRawLua}
                              disabled={luaSaving}
                            >
                              <Check size={15} />
                              <span>{luaSaving ? t.gameDetails.saving : t.actions.save}</span>
                            </button>
                          </div>
                        )}
                      </div>
                    ) : (
                      <div style={{ marginTop: "10px", fontSize: "13px", color: "var(--text-muted)" }}>
                        {inspectGame.steam_appid
                          ? "Chargement de la configuration Lua..."
                          : "Ce jeu n'a pas d'AppID Steam configuré."}
                      </div>
                    )}
                  </div>

                  <div className="details-danger-zone">
                    <button
                      className="btn-delete-game"
                      onClick={() => handleDeleteSingle(inspectGame.id)}
                    >
                      <Trash2 size={15} />
                      <span>{t.actions.deleteGame}</span>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <>
              {/* TAB 1: ACCUEIL (HOME) */}
              {activeTab === "home" && (
                <div className="page active">
                  <h1 className="page-heading">{t.home.welcomeTitle}</h1>

                  <div className="home-stats-grid">
                    <div className="stat-card">
                      <div className="stat-card-title">
                        <Gamepad2 size={20} className="icon-blue" />
                        <span>{t.home.statsTitle}</span>
                      </div>
                      <div className="stat-card-values">
                        <div className="stat-num-box">
                          <div className="stat-big-val">{games.length}</div>
                          <div className="stat-sub-lbl">{t.home.totalGames}</div>
                        </div>
                        <div className="stat-num-box">
                          <div className="stat-big-val">{totalPlaytimeHours}h</div>
                          <div className="stat-sub-lbl">{t.home.totalPlaytime}</div>
                        </div>
                      </div>
                    </div>

                    <div className="stat-card sync-box">
                      <div className="stat-card-title">
                        <Download size={20} className="icon-blue" />
                        <span>{t.home.importSteam}</span>
                      </div>
                      <p className="sync-desc">
                        {t.home.welcomeSubtitle}
                      </p>
                      <button
                        className="btn-toolbar"
                        onClick={handleImportSteam}
                        disabled={isImportingSteam}
                      >
                        <Download size={16} />
                        <span>{isImportingSteam ? t.home.importingSteam : t.home.importSteam}</span>
                      </button>
                    </div>
                  </div>

                  {recentGames.length > 0 && (
                    <div className="home-section">
                      <h2 className="section-title">
                        <Clock size={18} />
                        <span>{t.home.recentGames}</span>
                      </h2>
                      <div className="games-grid">
                        {recentGames.map((game) => {
                          const hasBanner = game.banner && bannerImages[game.banner];
                          return (
                            <div
                              key={`recent-${game.id}`}
                              className="game-card"
                              onClick={() => handleOpenDrawer(game)}
                            >
                              <div className="game-banner">
                                {hasBanner ? (
                                  <img src={bannerImages[game.banner!]} alt={game.name} loading="lazy" />
                                ) : (
                                  <span className="no-banner">{game.name.substring(0, 2).toUpperCase()}</span>
                                )}
                                <div className="play-circle-overlay">
                                  <div
                                    className="play-circle-btn"
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      handleLaunchGame(game);
                                    }}
                                    title={`Lancer ${game.name}`}
                                  >
                                    <Play fill="white" size={24} />
                                  </div>
                                </div>
                              </div>
                              <div className="game-info">
                                <h3 title={game.name}>{game.name}</h3>
                                <div className="playtime">
                                  <Clock size={13} />
                                  <span>{formatPlaytime(game.playtime)}</span>
                                  
                                </div>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  )}
                </div>
              )}

              {/* TAB 2: BIBLIOTHÈQUE */}
              {activeTab === "library" && (
                <div className="page active">
                  <div className="toolbar">
                    <div className="search-box">
                      <Search size={18} className="search-icon" />
                      <input
                        type="text"
                        placeholder={t.library.searchPlaceholder}
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                      />
                    </div>

                    {/* Filter Buttons */}
                    <div className="filter-buttons">
                      <button
                        className={`filter-btn ${filterMode === "all" ? "active" : ""}`}
                        onClick={() => setFilterMode("all")}
                      >
                        {t.library.filterAll} ({games.length})
                      </button>
                      <button
                        className={`filter-btn ${filterMode === "recent" ? "active" : ""}`}
                        onClick={() => setFilterMode("recent")}
                      >
                        {t.library.filterRecent} ({recentGames.length})
                      </button>
                      {photonGames.length > 0 && (
                        <button
                          className={`filter-btn ${filterMode === "photon" ? "active" : ""}`}
                          onClick={() => setFilterMode("photon")}
                        >
                          {t.library.filterPhoton} ({photonGames.length})
                        </button>
                      )}
                      {eosGames.length > 0 && (
                        <button
                          className={`filter-btn ${filterMode === "eos" ? "active" : ""}`}
                          onClick={() => setFilterMode("eos")}
                        >
                          {t.library.filterEos} ({eosGames.length})
                        </button>
                      )}
                    </div>

                    <div className="toolbar-actions">
                      <button
                        className="btn-toolbar"
                        onClick={handleImportSteam}
                        disabled={isImportingSteam}
                      >
                        <Download size={16} />
                        <span>{isImportingSteam ? t.home.importingSteam : t.home.importSteam}</span>
                      </button>

                      <button className="btn-toolbar btn-primary-add" onClick={handleOpenAddModal}>
                        <Plus size={16} />
                        <span>{t.actions.add}</span>
                      </button>
                    </div>
                  </div>

                  {/* SELECTION BAR */}
                  {selectedIds.size > 0 && (
                    <div className="selection-bar">
                      <span>{selectedIds.size} jeu(x) {t.library.selectedCount}</span>
                      <div style={{ display: "flex", gap: "8px" }}>
                        <button className="btn-sel-action" onClick={handleSelectAll}>
                          {t.actions.selectAll}
                        </button>
                        <button className="btn-sel-action" onClick={handleClearSelection}>
                          {t.actions.deselect}
                        </button>
                        <button className="btn-delete-selected" onClick={handleDeleteSelected}>
                          <Trash2 size={15} /> {t.actions.delete}
                        </button>
                      </div>
                    </div>
                  )}

                  {filteredGames.length === 0 ? (
                    <div className="empty-state">
                      <Layers size={56} />
                      <p>{t.library.noGamesFound}</p>
                      <span className="empty-hint">{t.library.noGamesSub}</span>
                    </div>
                  ) : (
                    <div className="games-grid">
                      {filteredGames.map((game) => {
                        const hasBanner = game.banner && bannerImages[game.banner];
                        const isSelected = selectedIds.has(game.id);

                        return (
                          <div
                            key={game.id}
                            className={`game-card ${isSelected ? "selected" : ""}`}
                            onClick={() => handleOpenDrawer(game)}
                          >
                            <div className="game-banner">
                              {hasBanner ? (
                                <img src={bannerImages[game.banner!]} alt={game.name} loading="lazy" />
                              ) : (
                                <span className="no-banner">{game.name.substring(0, 2).toUpperCase()}</span>
                              )}

                              {/* Multi-Selection Checkbox */}
                              <div
                                className="card-select-checkbox"
                                onClick={(e) => handleToggleSelect(game.id, e)}
                                title="Sélectionner"
                              >
                                {isSelected ? (
                                  <CheckSquare size={18} className="checkbox-icon checked" />
                                ) : (
                                  <Square size={18} className="checkbox-icon" />
                                )}
                              </div>

                              <div className="play-circle-overlay">
                                <div
                                  className="play-circle-btn"
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    handleLaunchGame(game);
                                  }}
                                  title={`Lancer ${game.name}`}
                                >
                                  <Play fill="white" size={24} />
                                </div>
                              </div>
                            </div>

                            <div className="game-info">
                              <h3 title={game.name}>{game.name}</h3>
                              <div className="playtime">
                                <Clock size={13} />
                                <span>{formatPlaytime(game.playtime)}</span>
                                
                              </div>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              )}

              {/* TAB 3: GÉNÉRER STEAM */}
              {activeTab === "generate" && (
                <div className="page active">
                  <h1 className="page-heading">{t.generator.title}</h1>

                  <div className="generate-wrapper">
                    <div className="generate-card">
                      <div className="form-group" style={{ position: "relative" }}>
                        <label>{t.generator.searchSteamPlaceholder}</label>
                        <input
                          type="text"
                          placeholder="Ex: Content Warning, https://store.steampowered.com/app/2881650/... ou 2881650"
                          value={genAppId}
                          onChange={(e) => handleGenInputChange(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") {
                              e.preventDefault();
                              handleGenerateLua();
                            }
                          }}
                        />
                        {genSearching && (
                          <div style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: "6px" }}>
                            Recherche sur Steam...
                          </div>
                        )}
                        {genSearchResults.length > 0 && (
                          <div className="autocomplete-dropdown" style={{ zIndex: 50, maxHeight: "280px", overflowY: "auto" }}>
                            {genSearchResults.map((sr) => (
                              <div
                                key={sr.appid}
                                className="autocomplete-item"
                                style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "12px", padding: "8px 12px", cursor: "pointer" }}
                                onClick={() => handleSelectGenResult(sr)}
                              >
                                <div style={{ display: "flex", alignItems: "center", gap: "10px", minWidth: 0 }}>
                                  <img
                                    src={sr.header_image}
                                    alt=""
                                    style={{ width: "48px", height: "24px", objectFit: "cover", borderRadius: "4px", flexShrink: 0 }}
                                  />
                                  <span style={{ fontWeight: 500, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                                    {sr.name}
                                  </span>
                                </div>
                                <span style={{ fontSize: "11px", color: "#58a6ff", background: "rgba(56, 139, 253, 0.15)", padding: "3px 8px", borderRadius: "4px", flexShrink: 0 }}>
                                  AppID {sr.appid} • Générer
                                </span>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>

                      <div
                        className="lua-drop-zone"
                        onDragOver={(e) => e.preventDefault()}
                        onDrop={handleFileDrop}
                        onClick={() => fileInputRef.current?.click()}
                      >
                        <input
                          type="file"
                          ref={fileInputRef}
                          style={{ display: "none" }}
                          accept=".lua"
                          onChange={handleFileChange}
                        />
                        <UploadCloud size={30} className="drop-icon" />
                        <div className="drop-title">
                          {genFileName ? genFileName : t.generator.dropLuaHint}
                        </div>
                        <div className="drop-hint">
                          {t.generator.help}
                        </div>
                      </div>

                      <button
                        className="btn-generate-main"
                        onClick={handleGenerateLua}
                        disabled={genGenerating || !genAppId.trim()}
                      >
                        <FileCode size={18} />
                        <span>{genGenerating ? t.generator.generating : t.generator.generateBtn}</span>
                      </button>

                      {genOutput && <div className="gen-alert-success">{genOutput}</div>}
                      {genError && <div className="gen-alert-error">{genError}</div>}
                    </div>
                  </div>
                </div>
              )}
              {/* TAB 4: PARAMÈTRES */}
              {activeTab === "settings" && (
                <div className="page active">
                  <h1 className="page-heading">{t.settings.title}</h1>

                  <div className="settings-wrapper">
                    {/* CHOIX DE LA LANGUE */}
                    <div className="setting-card">
                      <h3>{t.settings.languageTitle}</h3>
                      <p className="setting-desc">{t.settings.languageDesc}</p>
                      <div style={{ marginTop: "10px", maxWidth: "240px" }}>
                        <select
                          className="form-input"
                          style={{
                            width: "100%",
                            padding: "8px 12px",
                            background: "rgba(0,0,0,0.3)",
                            border: "1px solid var(--border)",
                            borderRadius: "6px",
                            color: "white",
                            fontSize: "13px",
                            cursor: "pointer",
                            outline: "none"
                          }}
                          value={settings.language}
                          onChange={(e) => handleSaveSettings({ ...settings, language: e.target.value })}
                        >
                          <option value="fr" style={{ background: "#1c2128", color: "white" }}>Français</option>
                          <option value="en" style={{ background: "#1c2128", color: "white" }}>English</option>
                          <option value="es" style={{ background: "#1c2128", color: "white" }}>Español</option>
                          <option value="de" style={{ background: "#1c2128", color: "white" }}>Deutsch</option>
                          <option value="ru" style={{ background: "#1c2128", color: "white" }}>Русский</option>
                        </select>
                      </div>
                    </div>
                    {/* MISES À JOUR DU LAUNCHER */}
                    <div className="setting-card">
                      <h3>{t.settings.updatesTitle}</h3>
                      <p className="setting-desc">{t.settings.updatesDesc}</p>
                      
                      <div style={{ display: "flex", alignItems: "center", gap: "16px", marginTop: "12px", flexWrap: "wrap" }}>
                        <div style={{ fontSize: "13px" }}>
                          {t.settings.currentVersion} <strong>v{updateInfo?.current_version || "2.0.0"}</strong>
                        </div>
                        <button
                          type="button"
                          className="btn-toolbar"
                          onClick={() => checkForUpdates(true)}
                          disabled={isCheckingUpdate || isUpdating}
                        >
                          <RefreshCw size={13} className={isCheckingUpdate ? "animate-spin" : ""} />
                          <span>{isCheckingUpdate ? t.settings.checkingUpdates : t.settings.checkUpdatesBtn}</span>
                        </button>
                      </div>

                      {updateInfo?.update_available ? (
                        <div style={{ marginTop: "14px", background: "rgba(37, 99, 235, 0.15)", border: "1px solid rgba(59, 130, 246, 0.4)", borderRadius: "8px", padding: "14px 16px" }}>
                          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "10px" }}>
                            <div>
                              <strong style={{ color: "#60a5fa", fontSize: "14px" }}>
                                {t.settings.updateAvailable} (v{updateInfo.latest_version})
                              </strong>
                              {updateInfo.release_name && (
                                <div style={{ fontSize: "12px", color: "var(--text-primary)", marginTop: "2px" }}>
                                  {updateInfo.release_name}
                                </div>
                              )}
                              {updateInfo.changelog && (
                                <div style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: "6px", maxHeight: "90px", overflowY: "auto", whiteSpace: "pre-line" }}>
                                  {updateInfo.changelog}
                                </div>
                              )}
                            </div>
                            <button
                              type="button"
                              className="btn-toolbar"
                              style={{ background: "#2563eb", color: "white", padding: "9px 18px", fontWeight: "bold" }}
                              onClick={handleInstallUpdate}
                              disabled={isUpdating}
                            >
                              <Download size={14} />
                              <span>{isUpdating ? t.updater.downloadingUpdate : t.settings.installUpdateBtn}</span>
                            </button>
                          </div>
                        </div>
                      ) : (
                        <div style={{ marginTop: "10px", fontSize: "12px", color: "#10b981" }}>
                          ✓ {t.settings.upToDateMsg}
                        </div>
                      )}
                    </div>

                    {/* SERVEUR DE MANIFESTS */}
                    <div className="setting-card">
                      <h3>{t.settings.manifestServerTitle}</h3>
                      <p className="setting-desc">
                        {t.settings.manifestServerDesc}
                      </p>
                      <div className="server-choice-grid">
                        <label
                          className={`server-option ${manifestServer === "steamrun" ? "selected" : ""}`}
                          onClick={() => handleChangeManifestServer("steamrun")}
                        >
                          <input
                            type="radio"
                            name="manifest_srv"
                            checked={manifestServer === "steamrun"}
                            onChange={() => {}}
                          />
                          <div>
                            <strong>{t.settings.serverMain}</strong>
                            <div className="server-url">https://manifest.steam.run/api/manifest/</div>
                          </div>
                        </label>

                        <label
                          className={`server-option ${manifestServer === "wudrm" ? "selected" : ""}`}
                          onClick={() => handleChangeManifestServer("wudrm")}
                        >
                          <input
                            type="radio"
                            name="manifest_srv"
                            checked={manifestServer === "wudrm"}
                            onChange={() => {}}
                          />
                          <div>
                            <strong>{t.settings.serverMirror}</strong>
                            <div className="server-url">http://gmrc.wudrm.com/manifest/</div>
                          </div>
                        </label>
                      </div>
                    </div>

                    {/* RÉPERTOIRE STEAM */}
                    <div className="setting-card">
                      <h3>{t.settings.steamPathTitle}</h3>
                      <p className="setting-desc">
                        {t.settings.steamPathDesc}
                      </p>
                      <div className="path-input-row">
                        <input
                          type="text"
                          value={settings.steam_path || detectedSteamPath}
                          placeholder="C:\Program Files (x86)\Steam"
                          onChange={(e) => handleSaveSettings({ ...settings, steam_path: e.target.value })}
                        />
                        <button
                          className="btn-toolbar"
                          onClick={async () => {
                            await loadSteamConfig();
                            if (detectedSteamPath) {
                              handleSaveSettings({ ...settings, steam_path: detectedSteamPath });
                            }
                          }}
                        >
                          <RefreshCw size={14} />
                          <span>{t.actions.detect}</span>
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </main>

      </div>

      {/* MODAL AJOUTER UN JEU */}
      {showModal && (
        <div className="modal-overlay" onClick={() => setShowModal(false)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>{t.addModal.title}</h2>
              <button className="modal-close" onClick={() => setShowModal(false)}>
                <X size={20} />
              </button>
            </div>

            <form onSubmit={handleSaveModal}>
              <div className="modal-body">
                <div className="form-group" style={{ position: "relative" }}>
                  <label>{t.addModal.name}</label>
                  <input
                    type="text"
                    required
                    value={formName}
                    onChange={(e) => handleSearchSteam(e.target.value)}
                    placeholder="Ex: Satisfactory..."
                  />
                  {steamResults.length > 0 && (
                    <div className="autocomplete-dropdown">
                      {steamResults.map((sr) => (
                        <div
                          key={sr.appid}
                          className="autocomplete-item"
                          onClick={() => handleSelectSteamResult(sr)}
                        >
                          <img src={sr.header_image} alt="" />
                          <span>{sr.name}</span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                <div className="form-group">
                  <label>{t.addModal.path}</label>
                  <input
                    type="text"
                    required
                    value={formPath}
                    onChange={(e) => setFormPath(e.target.value)}
                    placeholder="C:\Jeux\MonJeu\Game.exe"
                  />
                </div>

                <div className="form-group">
                  <label>{t.addModal.arguments}</label>
                  <input
                    type="text"
                    value={formArgs}
                    onChange={(e) => setFormArgs(e.target.value)}
                    placeholder="-windowed -high"
                  />
                </div>

                <div className="form-group">
                  <label>{t.addModal.appId}</label>
                  <input
                    type="text"
                    value={formAppId}
                    onChange={(e) => setFormAppId(e.target.value)}
                    placeholder="ex: 526870"
                  />
                </div>
              </div>

              <div className="modal-footer">
                <button
                  type="button"
                  className="btn-modal-cancel"
                  onClick={() => setShowModal(false)}
                >
                  Annuler
                </button>
                <button type="submit" className="btn-modal-save">
                  Ajouter le jeu
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}
