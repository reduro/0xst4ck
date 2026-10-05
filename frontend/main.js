// Made by the ClemtoutLauncher Team.
// You are allowed to use, modify, and redistribute this software for non-commercial purposes only.
// Sources:
// - Unpacker/Steamless: https://github.com/atom0s/Steamless
// - Steamtool : https://github.com/OpenSteam001/OpenSteamTool
const API_BASE = '';

window.showCustomModal = function (title, message, isConfirm = false) {
    return new Promise((resolve) => {
        const modal = document.getElementById('modal-alert');
        if (!modal) {
            if (isConfirm) resolve(confirm(message));
            else { alert(message); resolve(true); }
            return;
        }

        const titleEl = document.getElementById('modal-alert-title');
        const msgEl = document.getElementById('modal-alert-message');
        const okBtn = document.getElementById('modal-alert-ok');
        const cancelBtn = document.getElementById('modal-alert-cancel');

        const iconWrap = document.getElementById('modal-alert-icon');

        let cleanTitle = title;
        let iconType = 'info';

        if (title.includes('⚠️') || title.includes('\u26A0')) {
            iconType = 'warning';
            cleanTitle = title.replace(/[\u26A0\uFE0F⚠️]+/g, '').trim();
        } else if (title.includes('🎉')) {
            iconType = 'success';
            cleanTitle = title.replace(/🎉/g, '').trim();
        } else if (title.includes('🔍')) {
            iconType = 'search';
            cleanTitle = title.replace(/🔍/g, '').trim();
        }

        titleEl.textContent = cleanTitle;

        if (iconWrap) {
            if (iconType === 'warning') {
                iconWrap.style.background = 'rgba(243, 156, 18, 0.12)';
                iconWrap.style.color = '#f39c12';
                iconWrap.innerHTML = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>';
            } else if (iconType === 'success') {
                iconWrap.style.background = 'rgba(46, 204, 113, 0.12)';
                iconWrap.style.color = '#2ecc71';
                iconWrap.innerHTML = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>';
            } else if (iconType === 'search') {
                iconWrap.style.background = 'rgba(255, 255, 255, 0.05)';
                iconWrap.style.color = 'var(--text-primary)';
                iconWrap.innerHTML = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>';
            } else {
                iconWrap.style.background = 'rgba(31, 109, 243, 0.12)';
                iconWrap.style.color = 'var(--accent)';
                iconWrap.innerHTML = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>';
            }
        }
        if (msgEl) msgEl.innerHTML = message;

        if (isConfirm) {
            if (cancelBtn) cancelBtn.style.display = 'block';
            if (okBtn) okBtn.textContent = translate('modal.confirm');
            if (cancelBtn) cancelBtn.textContent = translate('modal.cancel');
        } else {
            if (cancelBtn) cancelBtn.style.display = 'none';
            if (okBtn) okBtn.textContent = translate('modal.ok');
        }

        modal.style.display = 'flex';
        modal.offsetHeight;
        modal.classList.add('show-anim');

        const closeMod = (val) => {
            modal.classList.remove('show-anim');
            setTimeout(() => {
                modal.style.display = 'none';
                resolve(val);
            }, 250);
        };

        if (okBtn) {
            okBtn.onclick = () => closeMod(true);
        }

        if (cancelBtn) {
            cancelBtn.onclick = () => closeMod(false);
        }
    });
};

let games = [];
let lastRenderedGames = [];
let selectedGames = new Set();
let currentGame = null;
let editingGame = null;
let currentLanguage = 'fr';
let selectedSteamID = null;

const i18n = {
    fr: {
        'nav.home': 'Accueil',
        'nav.library': 'Bibliothèque',
        "generate.ryuu_key": "Collez votre clé API Ryuu",
        "generate.ryuu_hint": "Vous devez vous connecter avec Discord sur",
        "generate.ryuu_hint_end": "pour obtenir votre clé.",
        'nav.generate': 'Générer Steam',
        'title.details': 'Détails du jeu',
        'nav.settings': 'Paramètres',
        'drop.zone': 'Glissez-déposez un fichier .lua ici ou cliquez',
        'generate.drop_zone_hint': 'Cette fonction est utile uniquement si l\'API ne possède pas votre jeu',
        'title.home': 'Bienvenue',
        'title.library': 'Bibliothèque',
        'title.generate': 'Générer un jeu Steam',
        'title.settings': 'Paramètres',
        'home.title': 'Bienvenue sur Clemtout Launcher',
        'home.stats': 'Statistiques',
        'home.games': 'Jeux',
        'home.playtime': 'Temps de jeu',
        'library.search': 'Rechercher un jeu...',
        'library.import': 'Importer Steam',
        'library.add': 'Ajouter',
        'library.delete': 'Supprimer',
        'library.deselect': 'Désélectionner',
        'library.empty': 'Aucun jeu trouvé',
        'library.empty_hint': 'Importe ta bibliothèque Steam ou ajoute des jeux manuellement',
        'generate.title': 'Générer un jeu Steam',
        'generate.appid': 'ID Steam (AppID)',
        'generate.search': 'Rechercher sur SteamDB',
        'generate.button': 'Générer',
        'generate.result': 'Résultat',
        'settings.title': 'Paramètres',
        'settings.language': 'Langue',
        'settings.steam_account': 'Compte Steam',
        'settings.paths': 'Chemins',
        'details.back': 'Retour',
        'details.launch_steam_fixed': 'Lancer avec Steam',
        'details.launch_steam_direct': 'Lancer sans Steam',
        'details.playtime': 'Temps total joué',
        'details.description': 'Description du jeu',
        'modal.add_game': 'Ajouter un jeu',
        'modal.edit_game': 'Modifier le jeu',
        'modal.name': 'Nom du jeu *',
        'modal.path': 'Chemin de l\'exécutable',
        'modal.args': 'Arguments (optionnel)',
        'modal.appid': 'Steam AppID',
        'modal.cancel': 'Annuler',
        'modal.save': 'Sauvegarder',
        'nav.legal_mentions': 'Mentions Légales',
        'modal.legal_title': 'Mentions Légales & Conditions d\'Utilisation',
        'modal.legal_body': 'Ce launcher est un utilitaire de configuration destiné aux détenteurs légitimes des jeux. Il permet l\'accès à des serveurs communautaires alternatifs. Ce logiciel ne cautionne pas le piratage. L\'utilisateur certifie posséder une licence valide pour chaque jeu lancé.',
        'nav.photon': 'Réseau',
        'photon.title': 'Configuration Photon Patch',
        'photon.subtitle': 'Accès aux Serveurs Multijoueurs',
        'photon.instructions1': 'Certains jeux requièrent une redirection DNS pour fonctionner avec les serveurs de la communauté.',
        'photon.status_configured': 'Configured',
        'photon.status_not_configured': 'Configuration Required',
        'photon.status_unknown': 'Vérification...',
        'photon.btn_copy': 'Copier les valeurs',
        'photon.btn_edit_admin': 'Modifier hosts',
        'details.launch_steam_spacewar': 'Jouer (Steam ID 480)',
        'details.launch_direct': 'Jouer (Sans Steam)',
        'modal.add': 'Ajouter Jeu',
        'modal.edit': 'Modifier Jeu',
        'title.photon': 'Photon Patch',
        'photon.explain_title': 'Gestion des Relais Réseau',
        'photon.explain_desc': 'Ce système redirige les serveurs Photon (Exit Games) vers des relais communautaires indépendants.',
        'photon.hint_patched': 'Patched: Serveurs Communautaires',
        'photon.hint_original': 'Originel: Serveurs Officiels',
        'photon.footer_text': 'Note: Redirection des services Exit Games (Photon) vers des relais communautaires.',
        'photon.btn_delete_official': 'Mode officiel',
        'legal.responsibility': "Toute utilisation détournée de cet outil reste sous l'entière responsabilité de l'utilisateur final. Clemtout Launcher n'est pas affilié à Valve Corporation, Steam, ou Exit Games.",
        'photon.btn_delete': 'Supprimer la configuration (Rejouer en officiel)',
        'photon.alert_title': 'Moteur Photon',
        'photon.alert_desc': 'Patch DNS requis pour le multijoueur.',
        'photon.alert_btn': 'Configurer',
        'connection.warning': 'Détection Photon indisponible (Pas de connexion)',
        'settings.steam_path': 'Chemin Steam',
        'settings.steam_path_hint': 'Dossier d\'installation (contenant steam.exe)',
        'settings.save': 'Sauvegarder',
        'settings.default': 'Défaut',
        'settings.no_steam_account': 'Aucun compte Steam trouvé',
        'settings.active': 'Actif',
        'photon.btn_clear_cache': 'Vider le cache',
        'photon.desc_clear_cache': 'Réinitialiser le cache DNS',
        'photon.desc_edit_admin': 'Éditer le fichier système',
        'photon.desc_copy': 'Copier dans le presse-papiers',
        'photon.desc_delete_official': 'Revenir aux serveurs d\'origine',
        'photon.current_status': 'Statut actuel du service',
        'modal.ok': 'OK',
        'modal.confirm': 'Confirmer',
        'modal.copied': 'Copié',
        'modal.copied_desc': 'La configuration DNS a été copiée dans votre presse-papiers.',
        'modal.delete_title': 'Suppression',
        'modal.delete_photon_desc': 'Voulez-vous vraiment supprimer la configuration DNS pour Photon ? Une élévation Admin sera demandée.',
        'modal.delete_game_desc': 'Êtes-vous sûr de vouloir supprimer ce jeu de votre bibliothèque ?',
        'modal.delete_multiple_title': 'Suppression Multiple',
        'modal.delete_multiple_desc': 'Êtes-vous sûr de vouloir supprimer <b>{count}</b> jeu(x) ?',
        'modal.import_steam_title': 'Importation Steam',
        'modal.import_steam_desc': 'Voulez-vous lancer le scan complet de vos disques pour trouver les jeux Steam installés ?',
        'modal.warning_title': 'Attention',
        'modal.appid_required': 'AppID requis (ou lien Steam valide)',
        'modal.appid_assistant': 'AppID Assistant',
        'modal.appid_updated': 'AppID mis à jour avec succès !',
        'modal.join_title': 'Rejoindre',
        'modal.join_desc': 'Voulez-vous rejoindre le serveur "<b style="color:var(--accent);">{server}</b>" ?',
        'modal.connecting_title': 'Connexion',
        'modal.connecting_desc': 'Lancement automatique de <b style="color:var(--accent);">{game}</b> et connexion au serveur en cours...',
        'modal.game_not_found': 'Le jeu "<b style="color:var(--accent);">{game}</b>" n\'a pas été trouvé dans votre bibliothèque pour le lancement automatique.',
        'generate.waiting': 'En attente...',
        'generate.search_placeholder': 'Nom du jeu ou AppID...',
        'generate.searching': 'Recherche en cours...',
        'generate.no_result': 'Aucun résultat trouvé',
        'generate.generating': 'Génération...',
        'generate.initializing': 'Initialisation en cours...',
        'generate.success': 'Génération Réussie !',
        'generate.success_desc': 'Le jeu devrait normalement être disponible dans votre bibliothèque Steam.',
        'generate.error': 'Erreur',
        'generate.network_error': 'Erreur Réseau',
        'generate.network_error_desc': 'Impossible de contacter le serveur.',
        'library.no_banner': 'Pas de bannière',
        'modal.success': 'Succès',
        'modal.error': 'Erreur',
        'api.success': 'Fichier Lua pour {appid} récupéré avec succès depuis l\'API.',
        'api.error_404': 'Erreur 404 : L\'API ne possède pas les fichiers pour ce jeu. Veuillez importer le jeu dans la zone prévue.',
        'api.error_403': 'Erreur 403 : Accès refusé (Cloudflare). Importation manuelle requise.',
        'api.error_generic': 'Erreur serveur ({code}). Importation manuelle requise.',
        'api.error_conn': 'Erreur de connexion à l\'API : {error}',
        'photon.cache_cleared_desc': 'Le cache de détection a été vidé. Les jeux seront rescannés lors de la prochaine visite.',
        'settings.launcher_edition': 'Édition du Launcher',
        'settings.launcher_mode': 'Mode actuel :',
        'settings.status_fix': 'Édition Fixe (Installé dans AppData avec Mises à jour Auto)',
        'settings.status_portable': 'Édition Portable (Pas d\'installation / Pas de Mises à jour)',
        'settings.status_not_configured': 'Non configuré',
        'settings.btn_convert_fix': 'Installer & Passer en Édition Fixe',
        'settings.btn_convert_portable': 'Passer en Mode Portable',
        'settings.btn_reinstall': 'Réinstaller le Launcher',
        'modal.reinstall_confirm': 'Voulez-vous vraiment réinstaller les composants du launcher ?',
    },
    en: {
        'nav.home': 'Home',
        'nav.library': 'Library',
        'nav.generate': 'Generate Steam',
        'title.details': 'Game details',
        'nav.settings': 'Settings',
        'drop.zone': 'Drag & drop a .lua file here or click',
        'generate.drop_zone_hint': 'This feature is only useful if the API does not have your game',
        'title.home': 'Welcome',
        'title.library': 'Library',
        'title.generate': 'Generate Steam Game',
        'title.settings': 'Settings',
        'home.title': 'Welcome to Clemtout Launcher',
        'home.stats': 'Statistics',
        'home.games': 'Games',
        'home.playtime': 'Playtime',
        'library.search': 'Search for a game...',
        'library.import': 'Import Steam',
        'library.add': 'Add',
        'library.delete': 'Delete',
        'library.deselect': 'Deselect',
        'library.empty': 'No games found',
        'library.empty_hint': 'Import your Steam library or add games manually',
        'generate.title': 'Generate Steam Game',
        'generate.appid': 'Steam ID (AppID)',
        'generate.search': 'Search on SteamDB',
        'generate.button': 'Generate',
        "generate.ryuu_key": "Paste your Ryuu API Key",
        "generate.ryuu_hint": "You must log in with Discord on",
        "generate.ryuu_hint_end": "to get your key.",
        'generate.result': 'Result',
        'settings.title': 'Settings',
        'settings.language': 'Language',
        'settings.steam_account': 'Steam Account',
        'settings.paths': 'Paths',
        'details.back': 'Back',
        'details.playtime': 'Total playtime',
        'details.launch_steam_generic': 'Launch via Steam',
        'details.launch_spacewar': 'Launch via Steam (Spacewar)',
        'details.launch_direct': 'Launch without Steam',
        'details.description': 'Game Description',
        'modal.add_game': 'Add Game',
        'modal.edit_game': 'Edit Game',
        'modal.name': 'Game name *',
        'modal.path': 'Executable path',
        'modal.args': 'Arguments (optional)',
        'modal.appid': 'Steam AppID',
        'modal.cancel': 'Cancel',
        'modal.save': 'Save',
        'nav.legal_mentions': 'Legal Mentions',
        'modal.legal_title': 'Legal Mentions & Terms of Use',
        'modal.legal_body': 'This launcher is a configuration utility intended for legitimate game owners. It enables access to alternative community servers. This software does not endorse piracy. The user certifies possessing a valid license for each game launched.',
        'nav.photon': 'Réseau',
        'photon.title': 'Photon Patch Setup',
        'photon.subtitle': 'Multiplayer Server Access',
        'photon.instructions1': 'Some games require a DNS redirection to work with community servers.',
        'photon.status_configured': 'Configured',
        'photon.status_not_configured': 'Configuration Required',
        'photon.status_unknown': 'Checking...',
        'details.launch_steam_spacewar': 'Play (Steam ID 480)',
        'details.launch_direct': 'Play (No Steam)',
        'modal.add': 'Add Game',
        'modal.edit': 'Edit Game',
        'title.photon': 'Photon Patch',
        'photon.explain_title': 'Network Relay Management',
        'photon.explain_desc': 'This system redirects Photon (Exit Games) servers to independent community relays.',
        'photon.hint_patched': 'Patched: Community Servers',
        'photon.hint_original': 'Original: Official Servers',
        'photon.footer_text': 'Note: Redirection of Exit Games (Photon) services to community relays.',
        'photon.btn_edit_admin': 'Edit Hosts',
        'photon.btn_copy': 'Copy Values',
        'photon.btn_delete_official': 'Official Mode',
        'legal.responsibility': 'Any misuse of this tool remains the sole responsibility of the end user. Clemtout Launcher is not affiliated with Valve Corporation, Steam, or Exit Games.',
        'photon.btn_delete': 'Delete configuration (Play on official)',
        'photon.alert_title': 'Photon Engine',
        'photon.alert_desc': 'DNS Patch required for multiplayer.',
        'photon.alert_btn': 'Configure',
        'connection.warning': 'Photon detection unavailable (No connection)',
        'settings.steam_path': 'Steam Path',
        'settings.steam_path_hint': 'Installation folder (containing steam.exe)',
        'settings.save': 'Save',
        'settings.default': 'Default',
        'settings.no_steam_account': 'No Steam account found',
        'settings.active': 'Active',
        'photon.btn_clear_cache': 'Clear Cache',
        'photon.desc_clear_cache': 'Reset DNS cache',
        'photon.desc_edit_admin': 'Edit the system file',
        'photon.desc_copy': 'Copy to clipboard',
        'photon.desc_delete_official': 'Revert to original servers',
        'photon.current_status': 'Current Service Status',
        'modal.ok': 'OK',
        'modal.confirm': 'Confirm',
        'modal.copied': 'Copied',
        'modal.copied_desc': 'The DNS configuration has been copied to your clipboard.',
        'modal.delete_title': 'Delete',
        'modal.delete_photon_desc': 'Are you sure you want to delete the Photon DNS configuration? Admin elevation will be requested.',
        'modal.delete_game_desc': 'Are you sure you want to remove this game from your library?',
        'modal.delete_multiple_title': 'Bulk Delete',
        'modal.delete_multiple_desc': 'Are you sure you want to delete <b>{count}</b> game(s)?',
        'modal.import_steam_title': 'Steam Import',
        'modal.import_steam_desc': 'Do you want to run a full scan of your drives to find installed Steam games?',
        'modal.warning_title': 'Warning',
        'modal.appid_required': 'AppID required (or valid Steam link)',
        'modal.appid_assistant': 'AppID Assistant',
        'modal.appid_updated': 'AppID successfully updated!',
        'modal.join_title': 'Join',
        'modal.join_desc': 'Do you want to join the server "<b style="color:var(--accent);">{server}</b>"?',
        'modal.connecting_title': 'Connecting',
        'modal.connecting_desc': 'Automatically launching <b style="color:var(--accent);">{game}</b> and connecting to the server...',
        'modal.game_not_found': 'The game "<b style="color:var(--accent);">{game}</b>" was not found in your library for automatic launch.',
        'generate.waiting': 'Waiting...',
        'generate.search_placeholder': 'Game name or AppID...',
        'generate.searching': 'Searching...',
        'generate.no_result': 'No result found',
        'generate.generating': 'Generating...',
        'generate.initializing': 'Initializing...',
        'generate.success': 'Generation Successful!',
        'generate.success_desc': 'The game should now be available in your Steam library.',
        'generate.error': 'Error',
        'generate.network_error': 'Network Error',
        'generate.network_error_desc': 'Cannot contact the server.',
        'library.no_banner': 'No banner',
        'modal.success': 'Success',
        'modal.error': 'Error',
        'api.success': 'Lua file for {appid} retrieved successfully from the API.',
        'api.error_404': 'Error 404: The API does not have the files for the desired game; please import the game into the import area.',
        'api.error_403': 'Error 403: Access denied (Cloudflare). Manual import required.',
        'api.error_generic': 'Server error ({code}). Manual import required.',
        'api.error_conn': 'API connection error: {error}',
        'photon.cache_cleared_desc': 'The detection cache has been cleared. Games will be rescanned on your next visit.',
        'settings.launcher_edition': 'Launcher Edition',
        'settings.launcher_mode': 'Current Mode:',
        'settings.status_fix': 'Fix Edition (Installed in AppData with Auto-Updates)',
        'settings.status_portable': 'Portable Edition (No Installation / No Auto-Updates)',
        'settings.status_not_configured': 'Not configured',
        'settings.btn_convert_fix': 'Install & Switch to Fix Edition',
        'settings.btn_convert_portable': 'Switch to Portable Mode',
        'settings.btn_reinstall': 'Reinstall Launcher',
        'modal.reinstall_confirm': 'Are you sure you want to reinstall the launcher components?',
    }
};

function translate(key) {
    return i18n[currentLanguage][key] || key;
}

function updateTranslations() {
    document.querySelectorAll('[data-i18n]').forEach(el => {
        const key = el.getAttribute('data-i18n');
        const translation = translate(key);
        
        // If the element has an SVG (icon), only update the text portion
        const textNode = Array.from(el.childNodes).find(node => node.nodeType === Node.TEXT_NODE && node.textContent.trim().length > 0);
        if (textNode) {
            textNode.textContent = translation;
        } else {
            el.textContent = translation;
        }
    });
    document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
        el.placeholder = translate(el.getAttribute('data-i18n-placeholder'));
    });
}

async function checkPhotonDbSync(retry = true) {
    try {
        const res = await fetch(`${API_BASE}/api/photon/db-status`);
        const data = await res.json();
        const warning = document.getElementById('connection-warning');
        
        if (data.syncing && retry) {
            // Wait for background sync to finish (retry once after 2.5s)
            setTimeout(() => checkPhotonDbSync(false), 2500);
            return;
        }

        if (warning) {
            warning.style.display = data.loaded ? 'none' : 'flex';
            const tooltip = document.getElementById('connection-tooltip');
            if (tooltip) tooltip.textContent = translate('connection.warning');
        }
    } catch (e) {
        console.error("Error checking Photon DB sync:", e);
        const warning = document.getElementById('connection-warning');
        if (warning) warning.style.display = 'flex';
    }
}

// Periodically check connection if DB isn't loaded
setInterval(() => {
    const warning = document.getElementById('connection-warning');
    if (warning && warning.style.display === 'flex') {
        console.log("[NETWORK] Periodic check: Trying to sync database...");
        checkPhotonDbSync();
    }
}, 60000);

document.addEventListener('DOMContentLoaded', () => {
    try {
        initNavigation();
        initToolbar();
        initModal();
        loadSettings();
        loadUserInfo();
        loadGames();
        initSettings();
        initPhotonPage();
        initLegalLink();
        checkPhotonDbSync();
        
        // Network Listeners for Auto-Reconnect
        window.addEventListener('online', () => {
            console.log("[NETWORK] Connection restored. Retrying Photon sync...");
            checkPhotonDbSync();
        });
        window.addEventListener('offline', () => {
            console.log("[NETWORK] Connection lost.");
            const warning = document.getElementById('connection-warning');
            if (warning) warning.style.display = 'flex';
        });
    } catch (e) {
        console.error("Critical error during initialization:", e);
    }
});



function extractAppID(val) {
    if (!val) return '';
    const match = val.match(/\/app\/(\d+)/) || val.match(/steam:\/\/run\/(\d+)/);
    if (match) return match[1];
    return /^\d+$/.test(val) ? val : '';
}

function initSettings() {
    const langSelect = document.getElementById('language-select');
    if (langSelect) {
        langSelect.addEventListener('change', async (e) => {
            currentLanguage = e.target.value;
            updateTranslations();

            try {
                await fetch(`${API_BASE}/api/settings`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ language: currentLanguage })
                });
            } catch (err) {
                console.error('Language save error:', err);
            }
        });
    }
}

const btnSettingsReinstall = document.getElementById('btn-settings-reinstall');
if (btnSettingsReinstall) {
    btnSettingsReinstall.addEventListener('click', async () => {
        const confirmed = await window.showCustomModal("⚠️ Réinstallation", "Voulez-vous vraiment réinstaller les composants du launcher ?", true);

        if (confirmed) {
            try {
                const res = await fetch('/api/install/reinstall', { method: 'POST' });
                const data = await res.json();

                if (data.success) {
                    window.showCustomModal("🎉 Succès", "Réinitialisation effectuée. Le launcher va se relancer...");
                    document.getElementById('install-overlay').style.display = 'flex';
                } else {
                    alert("Erreur lors de la réinstallation : " + data.error);
                }
            } catch (err) {
                console.error("Erreur réseau :", err);
            }
        }
    });
}

async function loadSettings() {
    try {
        const res = await fetch(`${API_BASE}/api/settings`);
        const settings = await res.json();
        selectedSteamID = settings.selected_steam_account || null;
        console.log("Settings chargés:", settings);

        if (document.getElementById('language-select')) {
            document.getElementById('language-select').value = settings.language || 'fr';
            currentLanguage = settings.language || 'fr';
        }

        const pathInput = document.getElementById('setting-steam-path');
        if (pathInput) {
            pathInput.value = settings.steam_path || 'C:\\Program Files (x86)\\Steam';
        }

        const saveBtn = document.getElementById('save-steam-path-btn');
        if (saveBtn) {
            const newBtn = saveBtn.cloneNode(true);
            saveBtn.parentNode.replaceChild(newBtn, saveBtn);

            newBtn.addEventListener('click', async () => {
                const pathInput = document.getElementById('setting-steam-path');
                if (!pathInput) return;

                const newPath = pathInput.value.trim();
                try {
                    const r = await fetch(`${API_BASE}/api/settings`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ steam_path: newPath })
                    });

                    if (r.ok) {
                        const d = await r.json();
                        alert(`Chemin sauvegardé : ${d.steam_path}`);
                        loadSteamAccounts();
                    } else {
                        alert("Server error during backup");
                    }
                } catch (e) {
                    console.error(e);
                    alert('Network error');
                }
            });
        }

        const resetBtn = document.getElementById('reset-steam-path-btn');
        if (resetBtn) {
            const newResetBtn = resetBtn.cloneNode(true);
            resetBtn.parentNode.replaceChild(newResetBtn, resetBtn);

            newResetBtn.addEventListener('click', async () => {
                try {
                    const r = await fetch(`${API_BASE}/api/settings`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ steam_path: "" })
                    });
                    if (r.ok) {
                        const d = await r.json();
                        document.getElementById('setting-steam-path').value = d.steam_path;
                        alert(`Chemin Steam réinitialisé à la détection automatique :\n${d.steam_path}`);
                        loadSteamAccounts();
                    }
                } catch(e) {
                    console.error(e);
                }
            });
        }

        updateTranslations();
    } catch (err) {
        console.error('Error loading settings:', err);
    }
}



function initNavigation() {
    document.querySelectorAll('.nav-item').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('.nav-item').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');

            const page = btn.dataset.page;
            showPage(page);
        });
    });
}

function showPage(page) {
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
    const pageTarget = document.getElementById(`${page}-page`);
    if (pageTarget) pageTarget.classList.add('active');

    const titleKey = `title.${page}`;
    document.querySelector('.title').textContent = translate(titleKey);

    if (page === 'photon') {
        checkPhotonStatus();
    } else if (page === 'settings') {
        loadSteamAccounts();
    } else if (page === 'home') {
        updateHomeStats();
    } else if (page === 'legal') {
        document.querySelector('.title').textContent = translate('nav.legal_mentions');
    } else if (page === 'admin') {
        document.querySelector('.title').textContent = 'Admin';
    }
}


function updateHomeStats() {
    const totalGames = games.length;
    const totalPlaytime = games.reduce((sum, g) => sum + (g.playtime || 0), 0);
    const hours = Math.floor(totalPlaytime / 3600);
    let totalSeconds = 0;
    document.getElementById('stat-games').textContent = totalGames;
    document.getElementById('stat-playtime').textContent = `${hours}h`;
    for (const g of games) {
        if (!g.playtime) continue;

        if (typeof g.playtime === "object") {
            totalSeconds += g.playtime[selectedSteamID] || 0;
        } else {
            totalSeconds += g.playtime || 0;
        }
    }

    const h = Math.floor(totalSeconds / 3600);
    const m = Math.floor((totalSeconds % 3600) / 60);

    document.getElementById('stat-games').textContent = totalGames;
    document.getElementById('stat-playtime').textContent = `${h}h${m.toString().padStart(2, '0')}`;
}
async function loadSteamAccounts() {
    try {
        const res = await fetch(`${API_BASE}/api/steam/accounts`);
        const data = await res.json();
        const container = document.getElementById('steam-accounts-list');

        if (data.accounts && data.accounts.length > 0) {
            container.innerHTML = data.accounts.map(acc => `
                <div class="account-item ${acc.selected ? 'selected' : ''}" data-steamid="${acc.steamid}">
                    <img src="${API_BASE}/api/steam/avatar/${acc.steamid}"
                         onerror="this.style.display='none'"
                         class="account-avatar">
                    <div class="account-info">
                        <div class="account-name">${acc.personaname}</div>
                        <div class="account-id">${acc.steamid}</div>
                    </div>
                    ${acc.selected ? `<span class="account-badge">${translate('settings.active')}</span>` : ''}
                </div>
            `).join('');

            container.querySelectorAll('.account-item').forEach(item => {
                item.addEventListener('click', () => selectSteamAccount(item.dataset.steamid));
            });
        } else {
            container.innerHTML = `<p>${translate('settings.no_steam_account')}</p>`;
        }
    } catch (err) {
        console.error('Steam account error:', err);
    }
}

async function selectSteamAccount(steamid) {
    try {
        await fetch(`${API_BASE}/api/steam/select`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ steamid })
        });

        selectedSteamID = steamid;

        fetch(`${API_BASE}/api/steam/update-playtime`, { method: "POST" }).then(() => {
            loadGames();
        });

        await Promise.all([
            loadSteamAccounts(),
            loadUserInfo(),
            loadGames()
        ]);

    } catch (err) {
        console.error('Error selecting account or Playtime update:', err);
    }
}

function initToolbar() {
    const searchInput = document.getElementById('search-input');
    if (searchInput) {
        searchInput.addEventListener('input', (e) => {
            const query = e.target.value.toLowerCase();
            renderGames(games.filter(g => g.name.toLowerCase().includes(query)));
        });
    }


    const genAppIdInput = document.getElementById('gen-appid');
    const autocompleteDiv = document.getElementById('steam-autocomplete-results');
    let searchDebounce = null;

    if (genAppIdInput && autocompleteDiv) {
        genAppIdInput.addEventListener('input', (e) => {
            const val = e.target.value.trim();
            clearTimeout(searchDebounce);

            if (/^\d+$/.test(val) || val.length < 2 || val.includes('http') || val.includes('steam://')) {
                autocompleteDiv.style.display = 'none';
                return;
            }

            searchDebounce = setTimeout(() => searchSteamGames(val), 350);
        });

        document.addEventListener('click', (e) => {
            if (!genAppIdInput.contains(e.target) && !autocompleteDiv.contains(e.target)) {
                autocompleteDiv.style.display = 'none';
            }
        });
    }



    const importBtn = document.getElementById('import-steam-btn');
    if (importBtn) {
        importBtn.addEventListener('click', importSteam);
    }

    const addBtn = document.getElementById('add-game-btn');
    if (addBtn) {
        addBtn.addEventListener('click', () => {
            editingGame = null;
            openModal();
        });
    }

    const deleteSelectedBtn = document.getElementById('delete-selected-btn');
    if (deleteSelectedBtn) {
        deleteSelectedBtn.addEventListener('click', deleteSelected);
    }

    const generateBtn = document.getElementById('generate-btn');
    if (generateBtn) {
        generateBtn.addEventListener('click', generateGame);
    }

    const backBtn = document.getElementById('back-btn');
    if (backBtn) {
        backBtn.addEventListener('click', () => {
            showPage('library');
            document.querySelectorAll('.nav-item').forEach(b => b.classList.remove('active'));
            const libBtn = document.querySelector('.nav-item[data-page="library"]');
            if(libBtn) libBtn.classList.add('active');
        });
    }

    const dropZone = document.getElementById('lua-drop-zone');
    const luaFileInput = document.getElementById('lua-file-input');

    if (dropZone && luaFileInput) {
        dropZone.addEventListener('click', () => {
            luaFileInput.click();
        });

        luaFileInput.addEventListener('change', (e) => {
            if (e.target.files.length > 0) {
                handleLuaFile(e.target.files[0]);
            }
        });

        dropZone.addEventListener('dragover', (e) => {
            e.preventDefault();
            dropZone.style.borderColor = 'var(--border-hover)';
            dropZone.style.background = 'rgba(102, 192, 244, 0.05)';
        });

        dropZone.addEventListener('dragleave', () => {
            dropZone.style.borderColor = 'var(--border)';
            dropZone.style.background = 'var(--bg-input)';
        });

        dropZone.addEventListener('drop', (e) => {
            e.preventDefault();
            dropZone.style.borderColor = 'var(--border)';
            dropZone.style.background = 'var(--bg-input)';

            if (e.dataTransfer.files.length > 0) {
                const file = e.dataTransfer.files[0];
                handleLuaFile(file);
            }
        });
    }
}

function handleLuaFile(file) {
    console.log("Fichier Lua détecté :", file.name);

    const reader = new FileReader();
    reader.onload = async (event) => {
        const rawContent = event.target.result;

        try {
            const res = await fetch(`${API_BASE}/api/lua/upload`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    filename: file.name,
                    content: rawContent
                })
            });

            const data = await res.json();
            if (data.success) {
                window.showCustomModal(`🎉 ${translate('modal.success')}`, `${translate('generate.lua_success_prefix')} ${file.name} ${translate('generate.lua_success_suffix')}`);
            } else {
                window.showCustomModal(`⚠️ ${translate('modal.error')}`, translate('generate.lua_error_process') + (data.error || translate('generate.lua_error_unknown')));
            }
        } catch (err) {
            console.error(err);
            window.showCustomModal(`⚠️ ${translate('modal.error')}`, translate('generate.lua_error_network'));
        }
    };

    reader.readAsText(file);
}

function initModal() {
    const modal = document.getElementById('modal-game');

    document.getElementById('modal-close').addEventListener('click', closeModal);
    document.getElementById('modal-cancel').addEventListener('click', closeModal);
    document.getElementById('modal-save').addEventListener('click', saveGame);

    modal.addEventListener('click', (e) => {
        if (e.target === modal) {
            e.stopPropagation();
            closeModal();
        }
    });

    const modalAppIdInput = document.getElementById('modal-appid');
    if (modalAppIdInput) {
        modalAppIdInput.addEventListener('input', (e) => {
            const extracted = extractAppID(e.target.value.trim());
            if (extracted && extracted !== e.target.value) {
                e.target.value = extracted;
            }
        });
    }
}

document.addEventListener('DOMContentLoaded', async () => {
    const generateBtn = document.getElementById('generate-btn');
    if (generateBtn) {
        generateBtn.addEventListener('click', generateGame);
    }
});

async function generateGame() {
    const rawInput = document.getElementById('gen-appid').value.trim();
    const appid = extractAppID(rawInput);

    if (!appid) {
        await window.showCustomModal(`⚠️ ${translate('modal.warning_title')}`, translate('modal.appid_required'));
        return;
    }

    const btn = document.getElementById('generate-btn');
    const output = document.getElementById('generate-output');

    btn.disabled = true;
    btn.innerHTML = `<svg class="spinner-icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: middle; margin-right: 6px; margin-bottom: 0;"><circle cx="12" cy="12" r="10"></circle><path d="M12 2v4"></path></svg> ${translate('generate.generating')}`;
    output.className = 'result-box';
    output.innerHTML = `<svg class="spinner-icon" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--text-secondary)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><path d="M12 2v4"></path></svg><div>${translate('generate.initializing')}</div>`;

    try {
        const res = await fetch(`${API_BASE}/api/generate`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ appid: appid }) 
        });
        
        const data = await res.json();        

        if (data.success) {
            output.className = 'result-box success';
            output.innerHTML = `<svg class="check-icon" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>
<div><b style="font-size: 16px;">${translate('generate.success')}</b><br><span style="color:var(--text-secondary); font-size: 13.5px; margin-top: 5px; display:inline-block;">${data.game.name}<br>${translate('generate.success_desc')}</span></div>`;
            await loadGames();
        } else {
            // Logique de traduction inline sans nouvelle fonction
            let errorMsg = data.error;
            if (errorMsg.includes("Error 404")) errorMsg = translate('api.error_404');
            else if (errorMsg.includes("Error 403") || errorMsg.includes("Blocked by Cloudflare")) errorMsg = translate('api.error_403');
            else if (errorMsg.includes("server returned an error")) errorMsg = translate('api.error_generic').replace('{code}', errorMsg.match(/\d+/) || '500');
            else if (errorMsg.includes("API connection error")) errorMsg = translate('api.error_conn').replace('{error}', '');

            output.className = 'result-box error';
            output.innerHTML = `<svg class="err-icon" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
<div><b>${translate('generate.error')}</b><br><span style="color:var(--text-secondary); font-size: 13.5px;">${errorMsg}</span></div>`;
        }
    } catch (err) {
        console.error(err);
        output.className = 'result-box error';
        output.innerHTML = `<svg class="err-icon" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
<div><b>${translate('generate.network_error')}</b><br><span style="color:var(--text-secondary); font-size: 13.5px;">${translate('generate.network_error_desc')}</span></div>`;
    } finally {
        btn.disabled = false;
        btn.innerHTML = `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:middle; margin-right:6px;"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"></path><polyline points="14 2 14 8 20 8"></polyline></svg> ${translate('generate.button')}`;
    }
}

function openModal(game = null) {
    const modal = document.getElementById('modal-game');
    editingGame = game;

    document.getElementById('modal-title').textContent = game ? translate('modal.edit_game') : translate('modal.add_game');
    document.getElementById('modal-name').value = game?.name || '';
    document.getElementById('modal-path').value = game?.path || '';
    document.getElementById('modal-args').value = game?.arguments || '';
    document.getElementById('modal-appid').value = game?.steam_appid || '';

    modal.style.display = 'flex';
}

function closeModal() {
    document.getElementById('modal-game').style.display = 'none';
}

async function saveGame() {
    const name = document.getElementById('modal-name').value.trim();
    if (!name) {
        await window.showCustomModal('Attention', 'Le nom du jeu est requis.');
        return;
    }

    const gameData = {
        name,
        path: document.getElementById('modal-path').value.trim(),
        arguments: document.getElementById('modal-args').value.trim(),
        steam_appid: document.getElementById('modal-appid').value.trim()
    };

    try {
        let response;
        if (editingGame) {
            response = await fetch(`${API_BASE}/api/games/${editingGame.id}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(gameData)
            });
        } else {
            response = await fetch(`${API_BASE}/api/games`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(gameData)
            });
        }

        if (response.ok) {
            closeModal();
            await loadGames();
        } else {
            await window.showCustomModal('Erreur', 'Erreur lors de la sauvegarde.');
        }
    } catch (err) {
        console.error(err);
        await window.showCustomModal('Erreur', 'Erreur réseau lors de la sauvegarde.');
    }
}

async function loadUserInfo() {
    try {
        const res = await fetch(`${API_BASE}/api/steam/accounts`);
        const data = await res.json();

        if (data.accounts && data.accounts.length > 0) {
            const selected = data.accounts.find(a => a.selected) || data.accounts[0];
            document.getElementById('user-name').textContent = selected.personaname;

            try {
                const avatarRes = await fetch(`${API_BASE}/api/steam/avatar/${selected.steamid}`);
                if (avatarRes.ok) {
                    const blob = await avatarRes.blob();
                    const url = URL.createObjectURL(blob);
                    const img = document.getElementById('user-avatar');
                    img.src = url;
                    img.style.display = 'block';
                }
            } catch (e) {
                console.log('Avatar non trouvé');
            }
        }
    } catch (err) {
        console.error('Erreur chargement user:', err);
    }
}

async function loadGames() {
    try {
        const res = await fetch(`${API_BASE}/api/games?t=${Date.now()}`);
        const data = await res.json();
        games = data.games || [];
        renderGames(games);
        updateHomeStats();
    } catch (err) {
        console.error('Erreur chargement games:', err);
        document.getElementById('games-grid').innerHTML =
            '<div class="loading">Erreur de chargement</div>';
    }
}

function renderGames(gamesToRender) {
    lastRenderedGames = gamesToRender;
    const grid = document.getElementById('games-grid');
    const emptyState = document.getElementById('empty-state');

    if (gamesToRender.length === 0) {
        grid.innerHTML = '';
        emptyState.style.display = 'block';
        return;
    }

    emptyState.style.display = 'none';
    grid.innerHTML = gamesToRender.map(game => createGameCard(game)).join('');

    document.querySelectorAll('.game-card').forEach(card => {
        const gameId = card.dataset.id;
        const game = games.find(g => g.id === gameId);

        card.addEventListener('click', (e) => {
            if (e.target.closest('.btn-edit') || e.target.closest('.btn-delete')) return;
            if (e.ctrlKey || e.metaKey) {
                toggleSelection(gameId);
            } else if (selectedGames.size > 0) {
                toggleSelection(gameId);
            } else {
                showGameDetails(game);
            }
        });

        card.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            toggleSelection(gameId);
        });

        card.querySelector('.btn-edit').addEventListener('click', (e) => {
            e.stopPropagation();
            openModal(game);
        });

        card.querySelector('.btn-delete').addEventListener('click', (e) => {
            e.stopPropagation();
            deleteGame(gameId);
        });
    });

    updateSelectionBar();
}

function createGameCard(game) {
    const playtime = game.playtime ? formatPlaytime(game.playtime) : null;
    const bannerUrl = game.banner ? `${API_BASE}/api/banners/${game.banner}` : '';
    const selected = selectedGames.has(game.id) ? 'selected' : '';

    return `
        <div class="game-card ${selected}" data-id="${game.id}">
            <div class="game-banner">
                ${bannerUrl ?
            `<img src="${bannerUrl}" alt="${game.name}">` :
            `<div class="no-banner">${translate('library.no_banner')}</div>`
        }
            </div>
            <div class="game-info">
                <h3>${escapeHtml(game.name)}</h3>
                ${playtime ? `
                    <div class="playtime">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                            <circle cx="12" cy="12" r="10"></circle>
                            <polyline points="12 6 12 12 16 14"></polyline>
                        </svg>
                        <span>${playtime}</span>
                    </div>
                ` : ''}
            </div>
            <div class="game-actions">
                <button class="btn-icon btn-edit" title="Modifier">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
                        <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
                    </svg>
                </button>
                <button class="btn-icon btn-delete" title="Supprimer">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="3 6 5 6 21 6"></polyline>
                        <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                    </svg>
                </button>
            </div>
        </div>
    `;
}

function toggleSelection(gameId) {
    if (selectedGames.has(gameId)) {
        selectedGames.delete(gameId);
    } else {
        selectedGames.add(gameId);
    }

    const card = document.querySelector(`.game-card[data-id="${gameId}"]`);
    if (card) {
        card.classList.toggle('selected');
    }

    updateSelectionBar();
}

function updateSelectionBar() {
    const bar = document.getElementById('selection-bar');
    const count = document.getElementById('selection-count');

    if (selectedGames.size > 0) {
        bar.style.display = 'flex';
        count.textContent = `${selectedGames.size} jeu(x) sélectionné(s)`;
    } else {
        bar.style.display = 'none';
    }
}

async function deleteGame(gameId) {
    const confirmed = await window.showCustomModal(`⚠️ ${translate('modal.delete_title')}`, translate('modal.delete_game_desc'), true);
    if (!confirmed) return;

    try {
        const res = await fetch(`${API_BASE}/api/games/${gameId}`, {
            method: 'DELETE'
        });

        if (res.ok) {
            await loadGames();
        }
    } catch (err) {
        console.error(err);
        await window.showCustomModal('Erreur', 'Erreur lors de la suppression.');
    }
}

async function deleteSelected() {
    if (selectedGames.size === 0) return;
    const confirmed = await window.showCustomModal(`⚠️ ${translate('modal.delete_multiple_title')}`, translate('modal.delete_multiple_desc').replace('{count}', selectedGames.size), true);
    if (!confirmed) return;

    try {
        const res = await fetch(`${API_BASE}/api/games/bulk-delete`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ ids: Array.from(selectedGames) })
        });

        if (res.ok) {
            selectedGames.clear();
            updateSelectionBar();
            await loadGames();
        }
    } catch (err) {
        console.error(err);
        await window.showCustomModal('Erreur', 'Erreur lors de la suppression multiple.');
    }
}

async function showGameDetails(game) {
    // Fetch latest game data to refresh Photon status
    try {
        const gameRes = await fetch(`${API_BASE}/api/games/${game.id}`);
        if (gameRes.ok) {
            game = await gameRes.json();
            currentGame = game;
        }
    } catch (e) { console.error("Error refreshing game data:", e); }

    // A full library load never scans game folders. Scan only the game whose
    // details were opened, then persist and display the concrete result.
    if (!game.network_runtime || !game.network_runtime.scanned) {
        try {
            const runtimeRes = await fetch(`${API_BASE}/api/games/${game.id}/network-runtime`, { method: 'POST' });
            if (runtimeRes.ok) {
                game.network_runtime = await runtimeRes.json();
                game.requires_photon = Boolean(game.network_runtime.photon);
                game.requires_eos = Boolean(game.network_runtime.eos);
            }
        } catch (e) {
            console.error('Network runtime scan error:', e);
        }
    }

    // Fetch local detector status
    let photonStatus = { active: false };
    try {
        const statusRes = await fetch(`${API_BASE}/api/photon/status`);
        photonStatus = await statusRes.json();
    } catch (e) { console.error("Error fetching photon status:", e); }

    document.getElementById('details-title').textContent = game.name;
    document.getElementById('details-playtime').textContent = formatPlaytime(game.playtime || 0);

    const runtimeContainer = document.getElementById('network-runtime-container');
    if (runtimeContainer) {
        const runtime = game.network_runtime || { runtime: 'unknown', evidence: [] };
        const labels = { photon: 'Photon', eos: 'EOS', photon_eos: 'Photon + EOS', none: 'Aucun runtime détecté', unknown: 'Analyse indisponible' };
        const evidence = (runtime.evidence || []).slice(0, 3).map(item => item.path).join(' · ');
        runtimeContainer.innerHTML = `<div class="source-info" style="padding:15px;background:rgba(255,255,255,.02);border-radius:8px;border:1px solid var(--border);font-size:12px;color:var(--text-muted);display:grid;gap:5px;">
            <strong style="color:var(--text)">Réseau détecté : ${labels[runtime.runtime] || runtime.runtime}</strong>
            <span>${evidence || 'Aucun marqueur local trouvé dans cette installation.'}</span>
        </div>`;
    }

    const bannerImg = document.getElementById('details-banner-img');

    if (game.steam_appid) {
        const hqUrl = `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_appid}/capsule_616x353.jpg`;
        const fallbackUrl = `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_appid}/header.jpg`;
        bannerImg.src = hqUrl;
        bannerImg.onerror = () => {
            if (game.banner) { bannerImg.src = `${API_BASE}/api/banners/${game.banner}`; }
            else { bannerImg.src = fallbackUrl; }
            bannerImg.onerror = null;
        };
        bannerImg.style.display = 'block';
    } else if (game.banner) {
        bannerImg.src = `${API_BASE}/api/banners/${game.banner}`;
        bannerImg.style.display = 'block';
    } else {
        bannerImg.style.display = 'none';
    }

    if (document.getElementById('launch-steam-btn')) {
        document.getElementById('launch-steam-btn').style.display = game.path ? 'block' : 'none';
        document.getElementById('launch-steam-btn').onclick = () => launchGame(game.id, 'crack_fix');
    }
    if (document.getElementById('launch-direct-btn')) {
        document.getElementById('launch-direct-btn').style.display = game.path ? 'block' : 'none';
        document.getElementById('launch-direct-btn').onclick = () => launchGame(game.id, 'direct');
    }

    if (game.steam_appid) {
        loadGameDescription(game.id);
    } else {
        document.getElementById('details-description').innerHTML = '<p>Aucune description disponible</p>';
    }

    // Elegant Glassmorphism Photon Banner Logic
    const existingBanner = document.getElementById('photon-game-alert');
    if (existingBanner) existingBanner.remove();

    if (game.requires_photon && !photonStatus.active) {
        console.log(`[DEBUG] Showing elegant Photon banner for ${game.name}`);
        const banner = document.createElement('div');
        banner.id = 'photon-game-alert';
        banner.className = 'photon-banner';
        banner.innerHTML = `
            <div class="photon-banner-icon">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                    <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
                    <line x1="12" y1="9" x2="12" y2="13"></line>
                    <line x1="12" y1="17" x2="12.01" y2="17"></line>
                </svg>
            </div>
            <div class="photon-banner-body">
                <div class="photon-banner-info">
                    <div class="photon-banner-title">${translate('photon.alert_title')}</div>
                    <div class="photon-banner-text">${translate('photon.alert_desc')}</div>
                </div>
                <button class="btn-photon-action" id="photon-goto-btn">
                    ${translate('photon.alert_btn')}
                </button>
            </div>
        `;

        const launchGrid = document.querySelector('.launch-actions-grid');
        if (launchGrid) {
            launchGrid.parentNode.insertBefore(banner, launchGrid);
        }

        document.getElementById('photon-goto-btn').addEventListener('click', () => {
            showPage('photon');
            document.querySelectorAll('.nav-item').forEach(b => b.classList.remove('active'));
            const photonNav = document.querySelector('.nav-item[data-page="photon"]');
            if (photonNav) photonNav.classList.add('active');
        });
    }

    showPage('details');
}

async function loadGameDescription(gameId) {
    const descDiv = document.getElementById('details-description');
    descDiv.innerHTML = '<p>Chargement...</p>';

    const langMap = {
        'fr': 'french',
        'en': 'english'
    };
    const steamLang = langMap[currentLanguage] || 'english';

    try {
        const res = await fetch(`${API_BASE}/api/games/${gameId}/details?lang=${steamLang}`);
        const data = await res.json();

        let html = '';
        if (data.description) {
            html += `<div>${data.description}</div>`;
        }
        if (data.requirements) {
            html += `<div class="requirements" style="margin-top: 20px; padding-top: 20px; border-top: 1px solid #333;">
                        <h3>⚙️ Configuration</h3>
                        ${data.requirements}
                     </div>`;
        }

        descDiv.innerHTML = html || '<p>Description non disponible</p>';
    } catch (err) {
        console.error(err);
        descDiv.innerHTML = '<p>Erreur de chargement</p>';
    }
}


async function launchGame(gameId, mode) {
    const steamBtn = document.getElementById('launch-steam-btn');
    const directBtn = document.getElementById('launch-direct-btn');

    [steamBtn, directBtn].forEach(btn => {
        if(btn) {
            btn.dataset.originalHtml = btn.innerHTML;
            btn.disabled = true;
            btn.style.opacity = '0.5';
            btn.style.cursor = 'wait';
            btn.innerHTML = `<span class="spinner" style="display:inline-block; margin-right:8px; width:12px; height:12px; border:2px solid; border-radius:50%; border-top-color:transparent; animation: spin 1s linear infinite;"></span> Lancement en cours...`;
        }
    });

    try {
        const payload = { mode: mode };

        const response = await fetch(`/api/games/${gameId}/launch`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        });

        const result = await response.json();
        if (result.success) {
            console.log("Jeu lancé avec succès en mode : " + mode);
        } else {
            await window.showCustomModal("Erreur de lancement", result.error);
        }
    } catch (err) {
        console.error("Erreur de lancement:", err);
    } finally {
        [steamBtn, directBtn].forEach(btn => {
            if(btn) {
                btn.disabled = false;
                btn.style.opacity = '1';
                btn.style.cursor = '';
                if(btn.dataset.originalHtml) {
                    btn.innerHTML = btn.dataset.originalHtml;
                }
            }
        });
    }
}

async function importSteam() {
    const confirmed = await window.showCustomModal(`🔍 ${translate('modal.import_steam_title')}`, translate('modal.import_steam_desc'), true);
    if (!confirmed) return;

    const btn = document.getElementById('import-steam-btn');
    const originalText = btn.innerHTML;

    btn.disabled = true;
    btn.innerHTML = '<span>🔍 Scan des disques en cours...</span>';

    try {
        console.log("Lancement de la requête d'import...");
        const res = await fetch(`${API_BASE}/api/import/steam`, {
            method: 'POST'
        });

        console.log("Réponse reçue, analyse...");
        const data = await res.json();

        if (data.success) {
            await window.showCustomModal('🎉 Import Terminé', `L'analyse est terminée !<br><br><span style="color:var(--accent); font-size:24px; font-weight:bold;">${data.imported}</span> nouveaux jeux Steam ont été trouvés et ajoutés à votre bibliothèque.`);
            await loadGames();
        } else {
            await window.showCustomModal('Erreur', 'Erreur signalée par le serveur: ' + data.error);
        }
    } catch (err) {
        console.error("Erreur JS Import:", err);
        await window.showCustomModal('Erreur Réseau', 'Erreur de connexion au backend. Vérifiez que l\'application tourne correctement.');
    } finally {
        btn.disabled = false;
        btn.innerHTML = originalText;
    }
}

async function searchSteamGames(query) {
    const autocompleteDiv = document.getElementById('steam-autocomplete-results');
    if (!autocompleteDiv) return;

    autocompleteDiv.innerHTML = `<div style="padding: 12px; color: var(--text-muted); font-size: 13px; display:flex; align-items:center; gap:8px;"><svg class="spinner-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><path d="M12 2v4"></path></svg> ${translate('generate.searching')}</div>`;
    autocompleteDiv.style.display = 'block';

    try {
        const res = await fetch(`${API_BASE}/api/steam/search?q=${encodeURIComponent(query)}`);
        if (!res.ok) throw new Error('API error');
        const data = await res.json();

        if (!data || data.length === 0) {
            autocompleteDiv.innerHTML = `<div style="padding: 12px; color: var(--text-muted); font-size: 13px;">${translate('generate.no_result')}</div>`;
            return;
        }

        autocompleteDiv.innerHTML = data.slice(0, 10).map(app => `
            <div class="steam-suggest-item" data-appid="${app.appid}" data-name="${app.name.replace(/"/g, '&quot;')}" style="display:flex; align-items:center; gap:12px; padding:10px 14px; cursor:pointer; transition: background 0.15s; border-bottom: 1px solid var(--border);">
                <img src="https://cdn.cloudflare.steamstatic.com/steam/apps/${app.appid}/capsule_231x87.jpg"
                     onerror="this.src='https://cdn.cloudflare.steamstatic.com/steam/apps/${app.appid}/header.jpg'; this.onerror=null;"
                     style="width:77px; height:29px; object-fit:cover; border-radius:4px; flex-shrink:0;">
                <div style="flex:1; min-width:0;">
                    <div style="font-size:13.5px; color:var(--text-primary); white-space:nowrap; overflow:hidden; text-overflow:ellipsis; font-weight:500;">${app.name}</div>
                    <div style="font-size:11px; color:var(--text-muted); margin-top:2px;">AppID: ${app.appid}</div>
                </div>
            </div>
        `).join('');

        autocompleteDiv.querySelectorAll('.steam-suggest-item').forEach(item => {
            item.addEventListener('mouseenter', () => { item.style.background = 'var(--bg-input)'; });
            item.addEventListener('mouseleave', () => { item.style.background = ''; });
            item.addEventListener('click', () => {
                const appid = item.dataset.appid;
                const genInput = document.getElementById('gen-appid');
                if (genInput) genInput.value = appid;
                autocompleteDiv.style.display = 'none';
            });
        });
    } catch (err) {
        console.error('Erreur recherche Steam:', err);
        autocompleteDiv.innerHTML = '<div style="padding: 12px; color: #e74c3c; font-size: 13px;">Erreur lors de la recherche. Vérifiez votre connexion.</div>';
    }
}

function formatPlaytime(playtime) {
    if (!playtime) return "0h00";

    if (typeof playtime === "object") {
        if (!selectedSteamID) return "0h00";

        const seconds = playtime[selectedSteamID] || 0;
        const h = Math.floor(seconds / 3600);
        const m = Math.floor((seconds % 3600) / 60);
        return `${h}h${m.toString().padStart(2, "0")}`;
    }

    const seconds = playtime || 0;
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    return `${h}h${m.toString().padStart(2, "0")}`;
}


function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

document.addEventListener('keydown', (e) => {
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.isContentEditable) {
        return;
    }

    if ((e.ctrlKey || e.metaKey) && (e.key === 'a' || e.key === 'A') &&
        document.getElementById('library-page').classList.contains('active')) {
        e.preventDefault();
        lastRenderedGames.forEach(g => selectedGames.add(g.id));
        renderGames(lastRenderedGames);
    }

    if (e.key === 'Delete' && selectedGames.size > 0) {
        deleteSelected();
    }

    if (e.key === 'Escape' && selectedGames.size > 0) {
        deselectAll();
    }
});

window.deselectAll = function () {
    selectedGames.clear();
    renderGames(games);
};



async function loadServers() {
    const tbody = document.getElementById('servers-tbody');
    if (!tbody) return;

    tbody.innerHTML = '<tr><td colspan="4" style="padding: 20px; text-align: center; color: var(--text-muted);">Chargement...</td></tr>';

    try {
        const res = await fetch(`${API_BASE}/api/servers`);
        const servers = await res.json();

        if (!servers || servers.length === 0) {
            tbody.innerHTML = '<tr><td colspan="4" style="padding: 20px; text-align: center; color: var(--text-muted);">Aucun serveur disponible</td></tr>';
            return;
        }

        tbody.innerHTML = servers.map(s => `
            <tr style="border-bottom: 1px solid var(--bg-card);">
                <td style="padding: 10px;">${escapeHtml(s.ServerName)}</td>
                <td style="padding: 10px;">${s.ServerCapacity} / ${s.MaxCapacity}</td>
                <td style="padding: 10px;"><span class="account-badge" style="background: var(--bg-input);">${escapeHtml(s.GameName)}</span></td>
                <td style="padding: 10px;">
                    <button class="btn-toolbar" onclick="reserveServer('${s.ServerId}', '${escapeHtml(s.ServerName).replace(/'/g, "\\'")}', '${escapeHtml(s.GameName).replace(/'/g, "\\'")}')">Rejoindre</button>
                </td>
            </tr>
        `).join('');
    } catch (err) {
        console.error('Erreur chargement serveurs:', err);
        tbody.innerHTML = '<tr><td colspan="4" style="padding: 20px; text-align: center; color: #ff4444;">Erreur de chargement</td></tr>';
    }
}

window.reserveServer = async function (serverId, serverName, gameName) {
    const confirmed = await window.showCustomModal(translate('modal.join_title'), translate('modal.join_desc').replace('{server}', serverName), true);
    if (!confirmed) return;

    let gameToLaunch = null;
    if (gameName) {
        const lowerGameName = gameName.toLowerCase();
        gameToLaunch = games.find(g =>
            g.name.toLowerCase().includes(lowerGameName) ||
            lowerGameName.includes(g.name.toLowerCase())
        );
    }

    if (gameToLaunch) {
        await window.showCustomModal(translate('modal.connecting_title'), translate('modal.connecting_desc').replace('{game}', gameToLaunch.name));
        try {
            const res = await fetch(`${API_BASE}/api/games/${gameToLaunch.id}/launch`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ mode: 'crack_fix', server_id: serverId })
            });

            const result = await res.json();
            if (result.success) {
                showServerSession(serverName);
            } else {
                await window.showCustomModal("Erreur de lancement", result.error);
            }
        } catch (err) {
            console.error(err);
            await window.showCustomModal("Erreur", "Erreur réseau impossible de joindre le launcher backend.");
        }
    } else {
        await window.showCustomModal(`⚠️ ${translate('modal.warning_title')}`, translate('modal.game_not_found').replace('{game}', gameName));
    }
};

function showServerSession(serverName) {
    const banner = document.getElementById('server-session-banner');
    const nameSpan = document.getElementById('active-server-name');
    if (banner && nameSpan) {
        banner.style.display = 'block';
        nameSpan.textContent = serverName;
    }
}


function initPhotonPage() {
    const scanButton = document.getElementById('network-rescan-btn');
    if (scanButton) scanButton.addEventListener('click', scanNetworkRuntimes);
}

async function checkPhotonStatus() {
    const statusEl = document.getElementById('photon-status-indicator');
    if (!statusEl) return;
    try {
        const res = await fetch(`${API_BASE}/api/photon/status`);
        const data = await res.json();
        const statusText = statusEl.querySelector('.status-text');
        statusEl.className = 'photon-status-pill configured';
        if (statusText) statusText.textContent = data.active ? 'Prêt — scan local' : 'Indisponible';
    } catch (e) {
        console.error('Network detector status error:', e);
        statusEl.className = 'photon-status-pill not_configured';
        const statusText = statusEl.querySelector('.status-text');
        if (statusText) statusText.textContent = 'Erreur';
    }
}

async function scanNetworkRuntimes() {
    const button = document.getElementById('network-rescan-btn');
    if (button) button.disabled = true;
    try {
        const res = await fetch(`${API_BASE}/api/network-runtime/scan`, { method: 'POST' });
        const data = await res.json();
        if (!res.ok || !data.success) throw new Error(data.error || 'Scan failed');
        await loadGames();
        const photon = data.games.filter(game => game.photon).length;
        const eos = data.games.filter(game => game.eos).length;
        await window.showCustomModal('Analyse terminée', `Photon : ${photon} jeu(x) · EOS : ${eos} jeu(x)`);
    } catch (e) {
        console.error('Network runtime scan error:', e);
        await window.showCustomModal('Erreur', 'Impossible d’analyser les installations sélectionnées.');
    } finally {
        if (button) button.disabled = false;
        checkPhotonStatus();
    }
}

async function updateGameAppId(gameId, newAppId) {
    try {
        const res = await fetch(`${API_BASE}/api/games/${gameId}`);
        const game = await res.json();
        
        game.steam_appid = newAppId;
        
        await fetch(`${API_BASE}/api/games/${gameId}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(game)
        });
        
        await window.showCustomModal(`🎉 ${translate('modal.appid_assistant')}`, translate('modal.appid_updated'));
        loadGames();
        loadGameDetails(gameId);
    } catch (err) {
        console.error('Erreur mise à jour AppID:', err);
    }
}

function initLegalLink() {
    console.log("Legal link initialized via standard navigation.");
}

async function verifyUpdaterUpdate() {
    try {
        const response = await fetch('/api/updater/check-self');
        const data = await response.json();

        if (data.update_started) {
            window.showCustomModal(
                "🔧 Updater Update",
                "An update for the updater is installing in the background. Please do not close the launcher, but you can still use it normally."
            );
        }
    } catch (err) {
        console.error('Error during updater auto-check:', err);
    }
}
async function checkInstallationFlow() {
    try {
        const res = await fetch('/api/install/status');
        const data = await res.json();

        updateSettingsInstallationUI(data.status);

        if (data.status === 'ask') {
            document.getElementById('install-overlay').style.display = 'flex';
        } else if (data.status === 'fix' || data.status === 'installed') {
            verifyUpdaterUpdate();
        }
    } catch (err) {
        console.error("Installation flow check failed:", err);
    }
}

function forceShowSetupOverlay() {
    const overlay = document.getElementById('install-overlay');
    if (overlay) {
        overlay.style.display = 'flex';
        document.getElementById('setup-buttons').style.display = 'flex';
        document.getElementById('setup-loading').style.display = 'none';
    }
}

function updateSettingsInstallationUI(status) {
    const textEl = document.getElementById('settings-launcher-mode-text');
    const btnReinstall = document.getElementById('btn-settings-reinstall');

    if (!textEl) return;

    if (status === 'fix') {
        textEl.textContent = translate('settings.status_fix');
        textEl.style.color = "var(--border-hover)";
    } else {
        textEl.textContent = translate('settings.status_not_configured');
        textEl.style.color = "var(--text-secondary)";
    }

    if(btnReinstall) btnReinstall.style.display = (status === 'fix') ? "inline-block" : "none";
}

function updateSettingsInstallationUI(status) {
    const textEl = document.getElementById('settings-launcher-mode-text');
    const btnReinstall = document.getElementById('btn-settings-reinstall');

    if (!textEl) return;

    if (status === 'fix') {
        textEl.textContent = translate('settings.status_fix');
        textEl.style.color = "var(--border-hover)";
    } else if (status === 'portable') {
        textEl.textContent = translate('settings.status_portable');
        textEl.style.color = "var(--text-primary)";
    } else {
        textEl.textContent = translate('settings.status_not_configured');
    }

    if(btnReinstall) btnReinstall.style.display = "inline-block";
}

document.addEventListener('DOMContentLoaded', () => {

    const btnFix = document.getElementById('btn-setup-fix');
    const btnReinstall = document.getElementById('btn-settings-reinstall');

    if (btnFix) {
        btnFix.addEventListener('click', async () => {
            await fetch('/api/install/fix', { method: 'POST' });
            document.getElementById('install-overlay').style.display = 'none';
            checkInstallationFlow();
        });
    }

    if (btnReinstall) {
        btnReinstall.addEventListener('click', async () => {
            const confirmed = await window.showCustomModal(translate('modal.delete_title'), translate('modal.reinstall_confirm'), true);
            if (!confirmed) return;

            try {
                const res = await fetch('/api/install/reinstall', { method: 'POST' });
                const data = await res.json();

                if (data.success) {
                    const overlay = document.getElementById('install-overlay');
                    if (overlay) {
                        overlay.style.setProperty('display', 'flex', 'important');
                        document.getElementById('setup-buttons').style.setProperty('display', 'flex', 'important');
                        document.getElementById('setup-loading').style.setProperty('display', 'none', 'important');
                    }
                }
            } catch (err) {
                console.error("erreur js :", err);
            }
        });
    }
});
async function handleFixInstallationFlow(overlayId) {
    const setupButtons = document.getElementById('setup-buttons');
    const setupLoading = document.getElementById('setup-loading');

    if (overlayId && setupButtons && setupLoading) {
        setupButtons.style.display = 'none';
        setupLoading.style.display = 'block';
    }

    try {
        const response = await fetch('/api/install/fix', { method: 'POST' });
        const result = await response.json();

        if (result.success) {
            window.showCustomModal("🎉 Success", "The Fix Edition is now fully installed! Shortcuts have been added to your Desktop and Start Menu.");
            if (overlayId) {
                setTimeout(() => {
                    document.getElementById(overlayId).style.display = 'none';
                }, 3000);
            }
            checkInstallationFlow();
        } else {
            window.showCustomModal("⚠️ Error", "Installation error: " + result.error);
            if (overlayId && setupButtons && setupLoading) {
                setupButtons.style.display = 'flex';
                setupLoading.style.display = 'none';
            }
        }
    } catch (err) {
        window.showCustomModal("⚠️ Error", "Request failed: " + err);
        if (overlayId && setupButtons && setupLoading) {
            setupButtons.style.display = 'flex';
            setupLoading.style.display = 'none';
        }
    }
}
async function checkUpdaterSelfUpdate() {
    try {
        const res = await fetch('/api/updater/check-self');
        const data = await res.json();

        if (data.update_started) {
            if (typeof window.showCustomModal === 'function') {
                window.showCustomModal(
                    "🔧 Updater Update",
                    "An update for the updater is being installed in the background. Please do not close the launcher; you can continue using it normally."
                );
            }
        }
    } catch (err) {
        console.error("Silently checking updater update failed:", err);
    }
}

document.addEventListener('DOMContentLoaded', () => {
    setTimeout(checkInstallationFlow, 1200);

    const btnPortable = document.getElementById('btn-setup-portable');
    const btnFix = document.getElementById('btn-setup-fix');
    const setupButtons = document.getElementById('setup-buttons');
    const setupLoading = document.getElementById('setup-loading');

    if (btnPortable) {
        btnPortable.addEventListener('click', async () => {
            await fetch('/api/install/portable', { method: 'POST' });
            document.getElementById('install-overlay').style.display = 'none';
            checkUpdaterSelfUpdate();
        });
    }

    if (btnFix) {
        btnFix.addEventListener('click', async () => {
            setupButtons.style.display = 'none';
            setupLoading.style.display = 'block';

            try {
                const response = await fetch('/api/install/fix', { method: 'POST' });
                const result = await response.json();

                if (result.success) {
                    if (typeof window.showCustomModal === 'function') {
                        window.showCustomModal("🎉 Success", "The Fix Edition is now fully installed! Shortcuts have been added to your Desktop and Start Menu.");
                    } else {
                        alert("Installation successful!");
                    }
                    setTimeout(() => {
                        document.getElementById('install-overlay').style.display = 'none';
                    }, 8000 );
                } else {
                    alert("Installation error: " + result.error);
                    setupButtons.style.display = 'flex';
                    setupLoading.style.display = 'none';
                }
            } catch (err) {
                alert("Request failed: " + err);
                setupButtons.style.display = 'flex';
                setupLoading.style.display = 'none';
            }
        });
    }
});
document.addEventListener('DOMContentLoaded', () => {
    setTimeout(verifyUpdaterUpdate, 1000);
});

const btnSettingsConvertFix = document.getElementById('btn-settings-convert-fix');
if (btnSettingsConvertFix) {
    btnSettingsConvertFix.addEventListener('click', async () => {
        try {
            const res = await fetch('/api/install/fix', { method: 'POST' });
            const data = await res.json();

            if (data.success) {
                window.showCustomModal("🔧 Mode Fix", "Le launcher va redémarrer instantanément en mode Fix...");
            } else {
                alert("Erreur : " + data.error);
            }
        } catch (err) {
            console.log("Redémarrage du launcher...");
        }
    });
}

async function togglePhoton(gameId, value) {
    try {
        await fetch(`${API_BASE}/api/games/${gameId}/photon-toggle`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ requires_photon: value })
        });
    } catch (err) {
        console.error('[PHOTON] Toggle error:', err);
    }
}