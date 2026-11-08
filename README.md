# Clemtout Launcher v2.0

> Plateforme de lancement de jeux multijoueur, émulateurs Steam, intégration Photon/EOS et gestionnaire de manifests.

## 🚀 Fonctionnalités v2.0

- **Architecture Moderne** : Backend haute performance en Rust (Tauri v2) et interface réactive en React 19 + TypeScript.
- **Support Multijoueur Avancé** :
  - **Photon P2P Relay** : Injection automatique et redirection réseau.
  - **Epic Online Services (EOS)** : Détection et support des hooks OnlineFix.
  - **Steam Spacewar (AppID 480)** : Lancement émulé standard.
- **Mises à Jour Automatiques (GitHub Releases)** :
  - Détection automatique au démarrage des nouvelles versions publiées sur le dépôt `reduro/0xst4ck`.
  - Téléchargement et exécution de l'installeur en un clic.
- **Générateur Steam & Manifests** :
  - Génération de configurations Lua pour OpenSteamTool.
  - Vérification en un clic des dernières versions de manifests via l'API OpenSteamTool (`steamrun` / `wudrm`).
- **Support Multi-Langues (i18n)** :
  - 🇫🇷 Français
  - 🇬🇧 English
  - 🇪🇸 Español
  - 🇩🇪 Deutsch
  - 🇷🇺 Русский
- **Installeur Standard Windows** :
  - Assistant d'installation complet (`Clemtout-Launcher-Setup.exe`) avec sélection de dossier et création de vrais raccourcis natifs `.lnk` (via COM `IShellLinkW`).

## 🛠️ Compilation depuis les sources

### Prérequis
- Node.js 18+ (avec npm)
- Rust & Cargo (dernière version stable)
- Visual Studio C++ Build Tools (pour Windows)

### Installation et build
```bash
# Installer les dépendances frontend
npm install

# Build release
npm run build
npx tauri build --no-bundle
```
Le binaire compilé sera situé dans `src-tauri/target/release/clemtout-launcher.exe`.
