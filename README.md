# Clemtout Launcher v2.0 (Development Build)

[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011%20(x64)-blue.svg)](#requirements)
[![Backend](https://img.shields.io/badge/Backend-Rust%202024%20%7C%20Tauri%20v2-orange.svg)](https://tauri.app/)
[![Frontend](https://img.shields.io/badge/Frontend-React%2019%20%7C%20TypeScript%20%7C%20Tailwind-blue.svg)](https://react.dev/)
[![Release](https://img.shields.io/github/v/release/reduro/0xst4ck?label=Latest%20Release)](https://github.com/reduro/0xst4ck/releases)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

> A high-performance, lightweight Windows game management platform engineered in **Rust (Tauri v2)** and **React 19**, designed for automated network transport routing, Steam emulation, and manifest synchronization.

---

## ⚡ Key Capabilities

### 1. Multi-Engine Network Scanner & Intelligent Routing
The launcher inspects binaries, dynamic libraries, and engine metadata on disk to dynamically configure the target game's network stack:
- **Photon P2P Relay**: Native routing for Photon Unity Networking (PUN, Fusion, Quantum, and Bolt) via an isolated local bridge and relay socket layer.
- **Epic Online Services (EOS)**: Automated OnlineFix EOS integration for games relying on Epic account authentication and lobby matchmaking.
- **Steamworks Spacewar Emulation**: Seamless standard AppID 480 environment configuration with native overlay injection.
- **Strict Precedence Engine**: Eliminates DLL hook conflicts in hybrid environments (e.g. Photon multiplayer games utilizing Epic for cross-play authentication).

### 2. Native GitHub Auto-Updater
- Direct integration with GitHub Releases API (`reduro/0xst4ck`).
- Automatic background download and headless execution of `Clemtout-Launcher-Setup.exe`.
- Seamless in-place file replacement without orphaned locks or broken installations.

### 3. Native Windows Installer
- Standalone self-extracting installer (`Clemtout-Launcher-Setup.exe`).
- Native COM `IShellLinkW` desktop and start menu shortcut generation with proper icon resolution.
- Per-user installation in `%LOCALAPPDATA%\Programs\ClemtoutLauncher` with silent uninstallation support.

### 4. Internationalization (i18n)
Full native localization support across all interfaces:
- 🇬🇧 English
- 🇫🇷 French (Français)
- 🇪🇸 Spanish (Español)
- 🇩🇪 German (Deutsch)
- 🇷🇺 Russian (Русский)

### 5. DRM Handling & Manifest Sync
- **Steamless DRM Unpacker**: Automatic detection and extraction of Steam stub-protected executables.
- **OpenSteamTool Integration**: In-app manifest queries (`steamrun` / `wudrm`) and automated Lua manifest script generation.

---

## 📥 Installation

Download the latest packages from the [Official Releases Page](https://github.com/reduro/0xst4ck/releases/latest):

| Package | Format | Description |
|---|---|---|
| **Clemtout-Launcher-Setup.exe** | Executable Installer | **Recommended**. Complete setup assistant, registers desktop shortcuts and enables auto-updates. |
| **Clemtout-Launcher.exe** | Portable Executable | Standalone binary. No installation required. |

### Requirements
- **Operating System**: Windows 10 or Windows 11 (64-bit)
- **Web Runtime**: Microsoft Edge WebView2 Runtime (pre-installed on Windows 10/11)

---

## 🛠️ Building from Source

### Prerequisites
1. **Node.js**: `18.0.0` or newer (with `npm`)
2. **Rust & Cargo**: Stable toolchain (`1.80+`)
   ```bash
   rustup default stable
   ```
3. **C++ Build Tools**: Visual Studio Build Tools with the "Desktop development with C++" workload.

### Step-by-Step Compilation

1. **Clone the repository**:
   ```bash
   git clone https://github.com/reduro/0xst4ck.git
   cd 0xst4ck
   ```

2. **Install frontend dependencies**:
   ```bash
   npm install
   ```

3. **Build the production release**:
   ```bash
   npm run build
   npx tauri build --no-bundle
   ```

The compiled native executable will be located at:
```
src-tauri/target/release/clemtout-launcher.exe
```

---

## 🏗️ Architecture

```
0xst4ck/
├── src/                      # React 19 + TypeScript Frontend
│   ├── App.tsx               # Main application orchestration & views
│   ├── components/           # UI components (GameCard, Settings, Terminal, etc.)
│   ├── i18n/                 # Localization dictionaries
│   └── styles/               # Tailwind CSS theme configurations
├── src-tauri/                # Tauri v2 / Rust Core Engine
│   ├── src/
│   │   ├── lib.rs            # Tauri command router & IPC interface
│   │   ├── scanner.rs        # Binary & metadata network engine analyzer
│   │   ├── loader.rs         # Game process lifecycle & hook injector
│   │   ├── updater.rs        # GitHub Releases update orchestrator
│   │   └── models.rs         # Strongly-typed data models
│   └── Cargo.toml            # Rust dependencies & compilation profile
└── Installer.cs              # Native C# Windows setup assistant
```

---

## 📄 License

Distributed under the MIT License. See [LICENSE](LICENSE) for more details.
