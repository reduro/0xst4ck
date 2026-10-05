# Made by the ClemtoutLauncher Team.
# You are allowed to use, modify, and redistribute this software for non-commercial purposes only.
# Sources:
# - Unpacker/Steamless: https://github.com/atom0s/Steamless
# - Steamtool : https://github.com/OpenSteam001/OpenSteamTool
import os, sys, threading, io, hashlib, re, json, uuid, time, zipfile, requests, vdf, urllib3, traceback, mimetypes, shutil, subprocess, random, socket
from pathlib import Path
from flask import Flask, jsonify, send_file, request, abort, send_from_directory, Response, redirect
from flask_cors import CORS

_logs = []

USER_AGENTS = [
    (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        " (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    ),
    (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101"
        " Firefox/121.0"
    ),
]

def log(msg):
    _logs.append(msg)
    if len(_logs) > 100:
        _logs.pop(0)
    print(msg)

urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

app = Flask(__name__)
CORS(app)
app.config['JSON_AS_ASCII'] = False
app.config['JSONIFY_MIMETYPE'] = 'application/json; charset=utf-8'

@app.after_request
def add_header(response):
    if response.mimetype and ('text/' in response.mimetype or 'javascript' in response.mimetype):
        response.charset = 'utf-8'
    return response

@app.after_request
def force_utf8_charset(response):
    if response.mimetype == 'application/json':
        response.headers['Content-Type'] = 'application/json; charset=utf-8'
    return response


if getattr(sys, 'frozen', False):
    BASE_DIR = Path(sys._MEIPASS)
    EXE_DIR = Path(sys.executable).parent
else:
    BASE_DIR = Path(__file__).resolve().parent
    EXE_DIR = BASE_DIR

USER_DIR = Path(os.getenv('APPDATA') or os.path.expanduser("~")) / 'clemtoutlauncher'
BANNERS_DIR = USER_DIR / 'banners'
GAMES_FILE = USER_DIR / 'games.json'
SETTINGS_FILE = USER_DIR / 'settings.json'
GAME_LOADER_EXE = BASE_DIR / "game_loader.exe"
UNPACKER_EXE = BASE_DIR / "Unpacker" / "ClemtoutLauncher.CLI.exe"
STEAMCMD_PATH = BASE_DIR / "steamcmd" / "steamcmd.exe"
DEPOT_KEYS_FILE_PATH = BASE_DIR / "depotkeys.json"
TOKENS_FILE_PATH = BASE_DIR / "appaccesstokens.json"
PHOTON_CACHE_FILE = USER_DIR / 'photon_cache.json'
STEAM_API_URL = "https://store.steampowered.com/api/appdetails?appids="
BACKUP_API_URL = "https://depotbox.org"
USER_AGENTS = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Safari/537.36 Edg/149.0.0.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15",
]

def get_frontend_dir():
    path_exe = BASE_DIR / 'frontend'
    if path_exe.exists():
        return path_exe

    path_dev = BASE_DIR.parent / 'frontend'
    if path_dev.exists():
        return path_dev

    return None

def load_photon_cache():
    if not PHOTON_CACHE_FILE.exists():
        return {}
    try:
        with open(PHOTON_CACHE_FILE, 'r', encoding='utf-8') as f:
            return json.load(f)
    except:
        return {}

def save_photon_cache(cache):
    try:
        with open(PHOTON_CACHE_FILE, 'w', encoding='utf-8') as f:
            json.dump(cache, f, indent=4)
    except:
        pass

import winreg

def get_steam_install_path():
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, r"Software\Valve\Steam") as key:
            path, _ = winreg.QueryValueEx(key, "SteamPath")
            return os.path.normpath(path)
    except Exception:
        pass
    return r"C:\Program Files (x86)\Steam"

STEAM_PATH = os.environ.get("STEAM_PATH") or get_steam_install_path()
LOGINUSERS_PATH = Path(STEAM_PATH) / "config" / "loginusers.vdf"
AVATAR_DIR = Path(STEAM_PATH) / "config" / "avatarcache"

STEAM_BASE_ID = 76561197960265728

def find_free_port(default_port=8000):

    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        s.bind(('127.0.0.1', default_port))
        s.close()
        return default_port
    except socket.error:
        s.bind(('127.0.0.1', 0))
        free_port = s.getsockname()[1]
        s.close()
        return free_port

def convert_steam32_to_64(steam32: str) -> str:
    """Converts a SteamID32 (AccountID) to SteamID64."""
    try:
        return str(int(steam32) + STEAM_BASE_ID)
    except (ValueError, TypeError):
        return ""

def convert_steamid64_to_32(steamid64: str) -> str:
    """Converts a SteamID64 back to SteamID32 (AccountID)."""
    try:
        return str(int(steamid64) - STEAM_BASE_ID)
    except (ValueError, TypeError):
        return ""

def parse_steam_friends(steamid64: str) -> list[dict]:
    """
    Reads the Steam friends list from localconfig.vdf or sharedconfig.vdf.
    Returns a list of dicts {steamid64, name, avatar}.
    The IDs in these files are SteamID32 (AccountID).
    """
    if not steamid64:
        return []

    # Convert the selected SteamID64 to SteamID32 to find the correct userdata folder
    try:
        steam32 = str(int(steamid64) - STEAM_BASE_ID)
    except (ValueError, TypeError):
        return []

    friends = []
    seen = set()

    # Paths to test (localconfig first, fallback sharedconfig)
    vdf_paths = [
        Path(STEAM_PATH) / "userdata" / steam32 / "config" / "localconfig.vdf",
        Path(STEAM_PATH) / "userdata" / steam32 / "7" / "remote" / "sharedconfig.vdf",
    ]

    for vdf_path in vdf_paths:
        if not vdf_path.exists():
            continue
        try:
            with open(vdf_path, "r", encoding="utf-8", errors="ignore") as f:
                data = vdf.load(f)

            # Search for the "friends" section (case-insensitive)
            def find_section(d, key):
                if not isinstance(d, dict):
                    return None
                for k, v in d.items():
                    if k.lower() == key.lower():
                        return v
                return None

            # localconfig.vdf : UserLocalConfigStore > friends
            root = find_section(data, "UserLocalConfigStore") or data
            friends_section = find_section(root, "friends")

            if not friends_section or not isinstance(friends_section, dict):
                continue

            for key, val in friends_section.items():
                if not key.isdigit():
                    continue
                if key in seen:
                    continue
                seen.add(key)

                sid64 = convert_steam32_to_64(key)
                if not sid64:
                    continue

                name = ""
                avatar = ""
                if isinstance(val, dict):
                    name   = val.get("name") or val.get("PersonaName") or ""
                    avatar = val.get("avatar") or ""

                friends.append({
                    "steamid64": sid64,
                    "name": name,
                    "avatar": avatar,
                })

            if friends:
                break  # Found in this file, no need to continue

        except Exception as e:
            print(f"[FRIENDS] Error reading {vdf_path}: {e}")

    print(f"[FRIENDS] {len(friends)} friends found for SteamID64={steamid64}")
    return friends

USER_DIR.mkdir(parents=True, exist_ok=True)
BANNERS_DIR.mkdir(parents=True, exist_ok=True)

if not GAMES_FILE.exists():
    with open(GAMES_FILE, 'w', encoding='utf-8') as f:
        json.dump({"games": []}, f)

if not SETTINGS_FILE.exists():
    with open(SETTINGS_FILE, 'w', encoding='utf-8') as f:
        json.dump({
            "language": "fr",
            "selected_steam_account": None,
            "theme": "dark"
        }, f)

def get_loader_path():
    return BASE_DIR / "game_loader.exe"

def get_insensitive(data, *keys):
    current = data
    for k in keys:
        if not isinstance(current, dict):
            return {}

        found = False
        k_lower = str(k).lower()
        for real_key in current.keys():
            if str(real_key).lower() == k_lower:
                current = current[real_key]
                found = True
                break

        if not found:
            return {}

    return current

def load_cf_clearance_from_settings():
    if os.path.exists(SETTINGS_FILE):
        try:
            with open(SETTINGS_FILE, 'r', encoding='utf-8') as f:
                data = json.load(f)
                return data.get('cf_clearance', '')
        except Exception as e:
            print(f"[!] Error reading SETTINGS_FILE: {e}")
    return ''

def save_cf_clearance_to_settings(cookie):
    data = {}
    if os.path.exists(SETTINGS_FILE):
        try:
            with open(SETTINGS_FILE, 'r', encoding='utf-8') as f:
                data = json.load(f)
        except Exception:
            data = {}

    data['cf_clearance'] = cookie.strip()

    try:
        with open(SETTINGS_FILE, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=4)
    except Exception as e:
        print(f"[!] Error writing SETTINGS_FILE: {e}")

def load_games():
    try:
        with open(GAMES_FILE, 'r', encoding='utf-8') as f:
            data = json.load(f)
        for game in data.get("games", []):
            if "last_played" not in game:
                game["last_played"] = 0
        data["games"].sort(key=lambda g: g.get("last_played", 0), reverse=True)
        return data
    except Exception as e:
        print("[ERROR] load_games:", e)
        return {"games": []}

def save_games(data):
    try:
        with open(GAMES_FILE, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=4, ensure_ascii=False)
        return True
    except Exception as e:
        print(f"Backup error: {e}")
        return False

def load_settings():
    defaults = {
        "language": "fr",
        "selected_steam_account": None,
        "theme": "dark",
        "steam_path": str(STEAM_PATH),
        "cached_playtime_accounts": [],
        "network_runtime_path": r"E:\\photon\\Loader"
    }

    if not SETTINGS_FILE.exists():
        save_settings(defaults)
        return defaults

    try:
        with open(SETTINGS_FILE, "r", encoding="utf-8") as f:
            data = json.load(f)
    except Exception:
        save_settings(defaults)
        return defaults

    for key, value in defaults.items():
        if key not in data:
            data[key] = value

    return data

def save_settings(data):
    try:
        with open(SETTINGS_FILE, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=4, ensure_ascii=False)
        return True
    except:
        return False

def get_launcher_mode_from_json():
    if os.path.exists(SETTINGS_FILE):
        try:
            with open(SETTINGS_FILE, "r", encoding="utf-8") as f:
                data = json.load(f)
                return data.get("launcher_mode", "ask")
        except:
            pass
    return "ask"

def save_launcher_mode_to_json(mode_value):
    os.makedirs(os.path.dirname(SETTINGS_FILE), exist_ok=True)
    data = {}
    if os.path.exists(SETTINGS_FILE):
        try:
            with open(SETTINGS_FILE, "r", encoding="utf-8") as f:
                data = json.load(f)
        except:
            data = {}
    data["launcher_mode"] = mode_value
    try:
        with open(SETTINGS_FILE, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=4, ensure_ascii=False)
        return True
    except:
        return False

def parse_loginusers(path):
    accounts = []
    if not Path(path).exists():
        return accounts

    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as f:
            txt = f.read()

        for m in re.finditer(r'"(\d{17,20})"\s*\{([^}]*)\}', txt, re.DOTALL):
            sid = m.group(1)
            block = m.group(2)

            persona_match = re.search(r'"PersonaName"\s*"([^"]*)"', block)
            acc_match = re.search(r'"AccountName"\s*"([^"]*)"', block)
            recent_match = re.search(r'"MostRecent"\s*"([^"]*)"', block)

            persona = persona_match.group(1) if persona_match else None
            acc = acc_match.group(1) if acc_match else None
            recent = recent_match.group(1) if recent_match else "0"

            accounts.append({
                "steamid": sid,
                "personaname": persona or acc or sid,
                "accountname": acc,
                "most_recent": recent == "1"
            })
    except Exception as e:
        print(f"Error reading loginusers: {e}")

    return accounts

def find_avatar_path(steamid):
    if not AVATAR_DIR.is_dir():
        return None

    for fname in os.listdir(AVATAR_DIR):
        if steamid in fname and fname.lower().endswith(".png"):
            return AVATAR_DIR / fname
    return None


def download_steam_banner(appid):
    filename = f"{appid}_banner.jpg"
    filepath = BANNERS_DIR / filename

    if filepath.exists():
        return filename

    library_hero_url = f"https://steamcdn-a.akamaihd.net/steam/apps/{appid}/library_hero.jpg"
    try:
        r = requests.get(library_hero_url, timeout=10)
        r.raise_for_status()
        with open(filepath, 'wb') as f:
            f.write(r.content)
        print(f"[BANNER] Large Library Hero banner retrieved for AppID {appid}")
        return filename
    except Exception:
        print(f"[BANNER] Library Hero unavailable for AppID {appid}, switch to the Store API...")

    urls = []

    details = get_steam_game_details(appid)
    if details and details.get("header_image"):
        urls.append(details["header_image"])

    urls.extend([
        f"https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg",
        f"https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/header.jpg",
        f"https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{appid}/header.jpg",
    ])

    for url in urls:
        try:
            r = requests.get(url, timeout=10)
            r.raise_for_status()
            with open(filepath, 'wb') as f:
                f.write(r.content)
            print(f"[BANNER] Emergency Store banner retrieved for AppID {appid}")
            return filename
        except:
            continue

    return None


def get_steam_game_details(appid, lang="french"):
    url = f"https://store.steampowered.com/api/appdetails?appids={appid}&l={lang}"

    try:
        r = requests.get(url, timeout=10)
        r.raise_for_status()
        data = r.json()
        details = data.get(str(appid), {}).get("data", {})

        return {
            "description": details.get("about_the_game") or details.get("short_description"),
            "requirements": details.get("pc_requirements", {}).get("minimum"),
            "name": details.get("name"),
            "header_image": details.get("header_image")
        }
    except Exception as e:
        print(f"Steam API Error: {e}")
        return None


def get_file_hash(file_path: str) -> str:
    hasher = hashlib.sha256()
    try:
        with open(file_path, 'rb') as f:
            while chunk := f.read(8192):
                hasher.update(chunk)
        return hasher.hexdigest()
    except Exception:
        return ""


def check_launchertools_needs_update(steam_base_path: str) -> bool:
    source_dir = BASE_DIR / "launchertools"
    if not source_dir.exists():
        return False

    for root, dirs, files in os.walk(source_dir):
        for file in files:
            source_file = os.path.join(root, file)
            rel_path = os.path.relpath(source_file, source_dir)
            target_file = os.path.join(steam_base_path, rel_path)

            if not os.path.exists(target_file):
                return True
            if get_file_hash(source_file) != get_file_hash(target_file):
                return True
                
    return False

def is_steam_running() -> bool:
    try:
        result = subprocess.run(
            ["tasklist", "/FI", "IMAGENAME eq steam.exe", "/NH"],
            capture_output=True, text=True
        )
        return "steam.exe" in result.stdout.lower()
    except Exception:
        return False

def smart_sync_launchertools(steam_path: str):
    if not check_launchertools_needs_update(steam_path):
        print("[+] Les fichiers launchertools sont déjà à jour. Steam n'est pas impacté.")
        return

    steam_was_running = is_steam_running()

    if steam_was_running:
        print("[*] Mise à jour des dll requise. Fermeture de Steam pour libérer les fichiers...")
        subprocess.run(["taskkill", "/F", "/IM", "steam.exe"], capture_output=True)
        time.sleep(2)
    else:
        print("[*] Mise à jour des dll requise. Steam est déjà fermé.")

    sync_launchertools(steam_path)

    if steam_was_running:
        steam_exe = os.path.join(steam_path, "steam.exe")
        subprocess.Popen([steam_exe])
        print("[*] Steam a été redémarré avec succès.")

def sync_launchertools(steam_base_path: str):
    source_dir = BASE_DIR / "launchertools"
    if not source_dir.exists():
        print(f"[-] The source folder 'launchertools' could not be found at the location : {source_dir}")
        return

    for root, dirs, files in os.walk(source_dir):
        for file in files:
            source_file = os.path.join(root, file)

            rel_path = os.path.relpath(source_file, source_dir)
            target_file = os.path.join(steam_base_path, rel_path)

            os.makedirs(os.path.dirname(target_file), exist_ok=True)

            if os.path.exists(target_file):
                if get_file_hash(source_file) == get_file_hash(target_file):
                    continue
                else:
                    print(f"[!] Corrupted or updated file detected, replacing : {rel_path}")
            else:
                print(f"[+] Installing the file : {rel_path}")

            shutil.copy2(source_file, target_file)

def solve_pow(challenge: str, dificultad: int = 4) -> str:
  objetivo = "0" * dificultad
  for nonce in range(20000000):
    digest = hashlib.sha256(f"{challenge}:{nonce}".encode("utf-8")).hexdigest()
    if digest.startswith(objetivo):
      return str(nonce)
  return None


def fetch_fallback_lua(appid: str, headers: dict) -> tuple:
  session = requests.Session()

  api_headers = headers.copy()
  api_headers.update({
      "accept": "*/*",
      "content-type": "application/json",
      "origin": "https://walftech.com",
      "sec-fetch-dest": "empty",
      "sec-fetch-mode": "cors",
      "sec-fetch-site": "same-origin",
  })

  partie1 = random.randint(100000000, 999999999)
  partie2 = random.randint(1000000000, 1999999999)
  session.cookies.set("_ga", f"GA1.1.{partie1}.{partie2}")

  try:
    resp_chal = session.post(
        "https://walftech.com/gate.php",
        json={"action": "challenge"},
        headers=api_headers,
        timeout=15,
    )
    if resp_chal.status_code != 200:
      return None, resp_chal.status_code

    chal_data = resp_chal.json()
    challenge = chal_data.get("challenge")
    dificultad = chal_data.get("difficulty", 4)

    if not challenge:
      return None, resp_chal.status_code

    nonce = solve_pow(challenge, dificultad)
    if not nonce:
      return None, resp_chal.status_code

    payload_redeem = {
        "action": "redeem",
        "id": str(appid),
        "challenge": challenge,
        "nonce": nonce,
    }

    resp_redeem = session.post(
        "https://walftech.com/gate.php",
        json=payload_redeem,
        headers=api_headers,
        timeout=15,
    )
    if resp_redeem.status_code != 200:
      return None, resp_redeem.status_code

    redeem_data = resp_redeem.json()
    token = redeem_data.get("token") or redeem_data.get("result")
    if not token:
      return None, resp_redeem.status_code

    download_url = f"https://walftech.com/depotbox_lua.php?id={appid}&token={token}&format=lua"
    resp_file = session.get(download_url, headers=headers, timeout=30)

    if resp_file.status_code == 200 and resp_file.content:
      return resp_file.content, 200

    return None, resp_file.status_code

  except Exception:
    return None, None


def save_lua_payload(
    content: bytes, config_base: str, target_folders: list, appid: str
):
  if content.startswith(b"PK"):
    with zipfile.ZipFile(io.BytesIO(content)) as z:
      for filename in z.namelist():
        if filename.endswith(".lua"):
          parts = filename.split("/")
          folder_target = (
              parts[0]
              if (len(parts) > 1 and parts[0] in target_folders)
              else "lua"
          )
          file_basename = os.path.basename(filename)
          dest_path = os.path.join(config_base, folder_target, file_basename)

          lua_content = z.read(filename).decode("utf-8", errors="ignore")
          clean_lines = [
              line
              for line in lua_content.splitlines()
              if not line.strip().lower().startswith("setmanifest")
          ]

          with open(dest_path, "w", encoding="utf-8", newline="\n") as f:
            f.write("\n".join(clean_lines))
  else:
    lua_content = content.decode("utf-8", errors="ignore")
    clean_lines = [
        line
        for line in lua_content.splitlines()
        if not line.strip().lower().startswith("setmanifest")
    ]

    dest_path = os.path.join(config_base, "lua", f"{appid}.lua")
    with open(dest_path, "w", encoding="utf-8", newline="\n") as f:
      f.write("\n".join(clean_lines))


def process_appid(appid: str):
  try:
    steam_path = get_steam_install_path()
  except NameError:
    steam_path = r"C:\Program Files (x86)\Steam"

  config_base = os.path.join(steam_path, "config")
  target_folders = ["lua", "stplug-in"]

  for folder in target_folders:
    os.makedirs(os.path.join(config_base, folder), exist_ok=True)

  print(f"[*] Querying API for AppID {appid}...")

  partie1 = random.randint(100000000, 999999999)
  partie2 = random.randint(1000000000, 1999999999)
  faux_cookie_ga = f"GA1.1.{partie1}.{partie2}"

  cookies = {"_ga": faux_cookie_ga}

  headers = {
      "User-Agent": random.choice(USER_AGENTS),
      "accept": (
          "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8"
      ),
      "accept-language": "fr,fr-FR;q=0.9,en;q=0.8",
      "referer": "https://walftech.com/generator.html",
      "upgrade-insecure-requests": "1",
      "sec-ch-ua-platform": '"Windows"',
  }

  url = f"https://walftech.com/proxy.php?id={appid}"

  try:
    response = requests.get(
        url,
        headers=headers,
        cookies=cookies,
        timeout=30,
    )

    content_bytes = None
    status_code = response.status_code

    if status_code == 200 and response.content:
      if (
          b"cf-challenge" in response.content
          or b"<html" in response.content[:100].lower()
      ):
        return (
            None,
            "Blocked by Cloudflare. The site requires manual verification.",
        )
      content_bytes = response.content
    else:
      content_bytes, fallback_status = fetch_fallback_lua(appid, headers)
      if fallback_status:
        status_code = fallback_status

    if content_bytes:
      save_lua_payload(content_bytes, config_base, target_folders, appid)

      if "smart_sync_launchertools" in globals():
        print("[*] Vérification et synchronisation des fichiers...")
        smart_sync_launchertools(steam_path)

      return (
          config_base,
          f"Lua file for {appid} retrieved from API.",
      )

    if status_code == 500:
      return (
          None,
          "Error 404: The API does not have the files for the desired game;"
          "  please import the games into the import area.",
      )
    elif status_code == 403:
      return (
          None,
          "Error 403: Access denied (Cloudflare cookie expired or invalid). You"
          " must manually import the Lua files into the designated area.",
      )
    else:
      return (
          None,
          f"Le serveur a renvoyé une erreur : {status_code}. Voplease import"
          " the games into the import area.",
      )

  except Exception as e:
    return None, f"API connection error : {e}"

@app.route('/api/lua/upload', methods=['POST'])
def api_lua_upload():
    req_data = request.get_json()
    if not req_data:
        return jsonify({"error": "Aucune donnée reçue"}), 400

    filename = req_data.get('filename')
    content = req_data.get('content')

    if not filename or not content:
        return jsonify({"error": "Nom de fichier ou contenu manquant"}), 400

    try:
        lines = content.splitlines()
        cleaned_lines = [line for line in lines if "SetManifestId" not in line]
        cleaned_content = "\n".join(cleaned_lines)

        steam_path = get_steam_install_path()
        output_dir = os.path.join(steam_path, "config", "lua")
        os.makedirs(output_dir, exist_ok=True)

        lua_file_path = os.path.join(output_dir, filename)

        if os.path.exists(lua_file_path):
            os.remove(lua_file_path)

        with open(lua_file_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(cleaned_content)

        if 'smart_sync_launchertools' in globals():
            smart_sync_launchertools(steam_path)

        return jsonify({
            "success": True,
            "message": f"Fichier {filename} installé sans modification de nom."
        })

    except Exception as e:
        return jsonify({"error": str(e)}), 500

def get_playtime_for_account(steam_path: str, steamid64: str, appid: str) -> int:
    steamid32 = convert_steamid64_to_32(steamid64)

    localconfig = (
            Path(steam_path)
            / "userdata"
            / steamid32
            / "config"
            / "localconfig.vdf"
    )

    if not localconfig.exists():
        return 0

    try:
        with open(localconfig, "r", encoding="utf-8", errors="ignore") as f:
            data = vdf.load(f)

        apps = get_insensitive(
            data,
            "UserLocalConfigStore",
            "Software",
            "Valve",
            "Steam",
            "apps",
        )

        node = apps.get(str(appid))
        if not node:
            return 0

        playtime_minutes = get_insensitive(node, "Playtime")
        if not playtime_minutes or not isinstance(playtime_minutes, (str, int, float)) or str(playtime_minutes).strip() == "":
            playtime_minutes = get_insensitive(node, "PlaytimeTotal")
            
        if isinstance(playtime_minutes, (str, int, float)) and str(playtime_minutes).isdigit():
            return int(playtime_minutes) * 60
            
        return 0

    except ValueError:
        return steamid64

active_sessions = {}

def get_local_steam_playtime(appid):
    settings = load_settings()
    steam_path = settings.get("steam_path", str(STEAM_PATH))
    steamid64 = settings.get("selected_steam_account")

    if not steamid64:
        return 0

    return get_playtime_for_account(steam_path, steamid64, appid)


@app.route('/')
def index():
    frontend_dir = get_frontend_dir()
    if frontend_dir:
        index_path = frontend_dir / 'index.html'
        if index_path.exists():
            return send_file(index_path)

    return f"Error : No index.html found. Checked in {BASE_DIR} and {BASE_DIR.parent}", 404


@app.route('/<path:path>')
def static_files(path):
    frontend_dir = get_frontend_dir()
    if frontend_dir:
        file_path = frontend_dir / path
        if file_path.exists():
            mimetype, _ = mimetypes.guess_type(path)

            if path.endswith('.css'):
                mimetype = 'text/css'
            elif path.endswith('.js'):
                mimetype = 'text/javascript'

            return send_from_directory(frontend_dir, path, mimetype=mimetype)

    return "Not Found", 404


@app.route('/banners/<filename>')
def serve_banners(filename):
    return send_from_directory(str(BANNERS_DIR), filename)


@app.route('/api/settings', methods=['GET', 'POST'])
def api_settings_fixed():
    if request.method == 'POST':
        settings = load_settings()
        new_settings = request.get_json()

        if 'steam_path' in new_settings:
            global STEAM_PATH, LOGINUSERS_PATH, AVATAR_DIR
            val = new_settings['steam_path'].strip()
            if not val:
                val = get_steam_install_path()
                new_settings['steam_path'] = val
            
            STEAM_PATH = Path(val)
            LOGINUSERS_PATH = STEAM_PATH / "config" / "loginusers.vdf"
            AVATAR_DIR = STEAM_PATH / "config" / "avatarcache"

        settings.update(new_settings)
        if save_settings(settings):
            return jsonify(settings)
        return jsonify({"error": "Failed to save"}), 500

    current = load_settings()
    if 'steam_path' not in current:
        current['steam_path'] = str(STEAM_PATH)
    return jsonify(current)

@app.route('/api/steam/accounts')
def api_accounts():
    accounts = parse_loginusers(LOGINUSERS_PATH)
    settings = load_settings()
    selected = settings.get('selected_steam_account')

    if not selected and accounts:
        selected = accounts[0]['steamid']
        settings['selected_steam_account'] = selected
        save_settings(settings)

    for acc in accounts:
        acc['selected'] = (acc['steamid'] == selected)

    return jsonify({"accounts": accounts})

@app.route('/api/steam/avatar/<steamid>')
def api_avatar(steamid):
    path = find_avatar_path(steamid)
    if not path:
        return redirect("https://images.steamusercontent.com/ugc/868480752636433334/1D2881C5C9B3AD28A1D8852903A8F9E1FF45C2C8/")
    return send_file(path, mimetype="image/png")



@app.post("/api/steam/update-playtime-for-selected")
def api_update_playtime_selected():
    settings = load_settings()
    selected_id = settings.get("selected_steam_account")

    if not selected_id:
        return jsonify({"error": "No Steam account selected"}), 400

    if selected_id in settings.get("cached_playtime_accounts", []):
        return jsonify({"success": True, "cached": True})

    steam_path = settings.get("steam_path", str(STEAM_PATH))
    data = load_games()

    updated = 0

    for game in data["games"]:
        appid = game.get("steam_appid")
        if not appid:
            continue

        try:
            seconds = get_playtime_for_account(steam_path, selected_id, str(appid))
        except:
            seconds = 0

        if "playtime" not in game or not isinstance(game["playtime"], dict):
            game["playtime"] = {}

        game["playtime"][selected_id] = seconds
        updated += 1

    save_games(data)

    settings["cached_playtime_accounts"].append(selected_id)
    save_settings(settings)

    return jsonify({"success": True, "updated": updated, "cached": False})

@app.route('/api/steam/select', methods=['POST'])
def api_select_steam_account():
    steamid = request.get_json().get('steamid')
    settings = load_settings()
    settings['selected_steam_account'] = steamid
    save_settings(settings)

    def quick_update():
        steam_path = settings.get("steam_path", str(STEAM_PATH))
        data = load_games()
        for game in data.get("games", []):
            appid = game.get("steam_appid")
            if appid and steamid:
                try:
                    if "playtime" not in game or not isinstance(game["playtime"], dict):
                        game["playtime"] = {}
                    game["playtime"][steamid] = get_playtime_for_account(steam_path, steamid, str(appid))
                except:
                    pass
        save_games(data)

    threading.Thread(target=quick_update, daemon=True).start()

    return jsonify({"success": True})


@app.route('/api/steam/search')
def api_steam_search():
    """Proxy for Steam search (bypasses CORS)"""
    q = request.args.get('q', '').strip()
    if not q or len(q) < 2:
        return jsonify([])
    try:
        r = requests.get(
            f"https://steamcommunity.com/actions/SearchApps/{requests.utils.quote(q)}",
            timeout=8,
            headers={"User-Agent": "Mozilla/5.0"}
        )
        r.raise_for_status()
        return jsonify(r.json())
    except Exception as e:
        print(f"[Steam Search Proxy] Error: {e}")
        return jsonify([]), 200

@app.route('/api/games')
def api_games():
    data = load_games()
    for game in data['games']:
        ensure_game_network_runtime(game)
    return jsonify(data)

@app.route('/api/games', methods=['POST'])
def api_add_game():
    game_data = request.get_json()

    if not game_data.get('name'):
        return jsonify({"error": "Name required"}), 400

    game_data['id'] = str(uuid.uuid4())
    game_data.setdefault('playtime', 0)
    game_data.setdefault('path', '')
    game_data.setdefault('arguments', '')
    update_game_network_runtime(game_data)

    data = load_games()
    data['games'].append(game_data)

    if save_games(data):
        return jsonify(game_data), 201
    return jsonify({"error": "Failed to save"}), 500

@app.route('/api/games/<game_id>', methods=['PUT'])
def api_update_game(game_id):
    game_data = request.get_json()
    data = load_games()

    for i, g in enumerate(data['games']):
        if g['id'] == game_id:
            game_data['id'] = game_id
            game_data['playtime'] = g.get('playtime', 0)
            
            # Preserve old banner if not provided
            if 'banner' not in game_data or not game_data['banner']:
                game_data['banner'] = g.get('banner')
                
                # If appid changed, try to download a new banner
                new_appid = str(game_data.get('steam_appid', ''))
                old_appid = str(g.get('steam_appid', ''))
                if new_appid and new_appid != old_appid:
                    new_banner = download_steam_banner(new_appid)
                    if new_banner:
                        game_data['banner'] = new_banner

            # Runtime detection is derived locally and never trusted from the client.
            game_data.pop('requires_photon', None)
            game_data.pop('requires_eos', None)
            game_data.pop('network_runtime', None)
            update_game_network_runtime(game_data)

            data['games'][i] = game_data
            if save_games(data):
                return jsonify(game_data)
            return jsonify({"error": "Failed to save"}), 500

    return jsonify({"error": "Game not found"}), 404

@app.route('/api/games/<game_id>', methods=['GET'])
def api_get_game(game_id):
    data = load_games()
    for g in data['games']:
        if g['id'] == game_id:
            ensure_game_network_runtime(g)
            return jsonify(g)
    return jsonify({"error": "Game not found"}), 404

@app.route('/api/games/<game_id>', methods=['DELETE'])
def api_delete_game(game_id):
    data = load_games()
    settings = load_settings()
    settings["cached_playtime_accounts"] = []
    save_settings(settings)
    data['games'] = [g for g in data['games'] if g['id'] != game_id]

    if save_games(data):
        return jsonify({"success": True})
    return jsonify({"error": "Failed to save"}), 500

@app.route('/api/games/bulk-delete', methods=['POST'])
def api_bulk_delete():
    ids = request.get_json().get('ids', [])
    data = load_games()
    data['games'] = [g for g in data['games'] if g['id'] not in ids]

    if save_games(data):
        return jsonify({"success": True, "deleted": len(ids)})
    return jsonify({"error": "Failed to save"}), 500

@app.route('/api/games/<game_id>/details')
def api_game_details(game_id):
    data = load_games()
    game = next((g for g in data['games'] if g['id'] == game_id), None)

    if not game:
        return jsonify({"error": "Game not found"}), 404

    appid = game.get('steam_appid')
    if not appid:
        return jsonify({"error": "No Steam AppID"}), 400

    lang = request.args.get("lang", "french")

    details = get_steam_game_details(appid, lang=lang)

    if details:
        return jsonify(details)

    return jsonify({"error": "Failed to fetch details"}), 500


def detect_game_path(game_path):
    path_obj = Path(game_path)
    if not path_obj.exists():
        return game_path
        
    root_dir = path_obj.parent
    exe_stem = path_obj.stem.lower()
    unity_indicators = ["UnityPlayer.dll", "UnityCrashHandler64.exe"]
    for indicator in unity_indicators:
        if (root_dir / indicator).exists():
            print(f"[DEBUG] Unity detected via {indicator}. Keeping original path.")
            return game_path
    try:
        
        potential_dirs = [d for d in root_dir.iterdir() if d.is_dir()]
        
        def get_score(name, target):
            n, t = name.lower(), target.lower()
            if n == t: return 100
            if n in t or t in n: return 50
            score = 0
            for i in range(min(len(n), len(t))):
                if n[i] == t[i]: score += 1
                else: break
            return score

        potential_dirs.sort(key=lambda d: get_score(d.name, exe_stem), reverse=True)
        
        search_roots = [root_dir] + potential_dirs
        
        for base in search_roots:
            shipping_dir = base / "Binaries" / "Win64"
            if shipping_dir.exists():
                shipping_exes = list(shipping_dir.glob("*Shipping.exe"))
                if not shipping_exes:
                    shipping_exes = [f for f in shipping_dir.glob("*.exe") if "crash" not in f.name.lower()]
                
                if shipping_exes:
                    shipping_exes.sort(key=lambda x: x.stat().st_size, reverse=True)
                    new_path = str(shipping_exes[0])
                    print(f"[DEBUG] Unreal Shipping EXE detected in {base.name}: {new_path}")
                    return new_path

        print(f"[DEBUG] Unreal Shipping not found in standard paths, wide search in {root_dir}")
        for root, dirs, files in os.walk(root_dir):
            depth = Path(root).relative_to(root_dir).parts
            if len(depth) > 3:
                del dirs[:]
                continue
                
            if "Binaries" in dirs and "Win64" in os.listdir(Path(root) / "Binaries"):
                win64_path = Path(root) / "Binaries" / "Win64"
                exes = [f for f in win64_path.glob("*.exe") if "shipping" in f.name.lower()]
                if exes:
                    exes.sort(key=lambda x: x.stat().st_size, reverse=True)
                    return str(exes[0])

    except Exception as e:
        print(f"[ERROR] Engine detection failed: {e}")

    return game_path


# Local network-runtime detection
# The launcher only inspects files belonging to the selected game. It never
# changes DNS/hosts settings and does not start or inject third-party loaders.
NETWORK_RUNTIME_MAX_FILES = 1500
NETWORK_RUNTIME_MAX_BYTES = 2 * 1024 * 1024
PHOTON_FILE_MARKERS = {
    "photon3unity3d.dll", "photonrealtime.dll", "photonnetwork.dll",
    "exitgames.client.photon.dll", "photonunitynetworking.dll",
}
EOS_FILE_MARKERS = {
    "eossdk-win64-shipping.dll", "eossdk-win32-shipping.dll",
    "eossdk-win64.dll", "eossdk-win32.dll", "eossdk.dll",
}
PHOTON_TEXT_MARKERS = (b"exitgames", b"photonengine", b"photon.realtime")
EOS_TEXT_MARKERS = (b"epiconlineservices", b"eossdk", b"eos_productid")


def _game_scan_root(game_path):
    if not game_path:
        return None
    try:
        path = Path(game_path).expanduser().resolve()
    except (OSError, RuntimeError):
        return None
    return path.parent if path.is_file() else path


def detect_network_runtime(game_path):
    """Return local Photon/EOS evidence without contacting or modifying a service."""
    root = _game_scan_root(game_path)
    result = {"runtime": "none", "photon": False, "eos": False, "evidence": [], "scanned": False}
    if not root or not root.is_dir():
        result["runtime"] = "unknown"
        result["error"] = "Game directory unavailable"
        return result

    files_seen = 0
    try:
        for candidate in root.rglob("*"):
            if files_seen >= NETWORK_RUNTIME_MAX_FILES:
                result["truncated"] = True
                break
            if not candidate.is_file():
                continue
            files_seen += 1
            name = candidate.name.lower()
            relative_name = str(candidate.relative_to(root))

            if name in PHOTON_FILE_MARKERS:
                result["photon"] = True
                result["evidence"].append({"type": "file", "runtime": "photon", "path": relative_name})
            if name in EOS_FILE_MARKERS:
                result["eos"] = True
                result["evidence"].append({"type": "file", "runtime": "eos", "path": relative_name})

            # Inspect only likely binaries and only a bounded prefix. This is
            # sufficient for metadata markers and avoids broad content collection.
            if candidate.suffix.lower() not in {".exe", ".dll", ".bin"}:
                continue
            try:
                with candidate.open("rb") as binary:
                    sample = binary.read(NETWORK_RUNTIME_MAX_BYTES).lower()
            except OSError:
                continue

            if not result["photon"] and any(marker in sample for marker in PHOTON_TEXT_MARKERS):
                result["photon"] = True
                result["evidence"].append({"type": "marker", "runtime": "photon", "path": relative_name})
            if not result["eos"] and any(marker in sample for marker in EOS_TEXT_MARKERS):
                result["eos"] = True
                result["evidence"].append({"type": "marker", "runtime": "eos", "path": relative_name})
    except OSError as exc:
        result["error"] = str(exc)

    result["scanned"] = True
    result["files_scanned"] = files_seen
    if result["photon"] and result["eos"]:
        result["runtime"] = "photon_eos"
    elif result["photon"]:
        result["runtime"] = "photon"
    elif result["eos"]:
        result["runtime"] = "eos"
    return result


def ensure_game_network_runtime(game):
    """Return the last scan result without delaying the games list."""
    detection = game.get("network_runtime")
    if not isinstance(detection, dict):
        detection = {"runtime": "unknown", "photon": False, "eos": False,
                     "evidence": [], "scanned": False}
        game["network_runtime"] = detection
    game["requires_photon"] = bool(detection.get("photon", False))
    game["requires_eos"] = bool(detection.get("eos", False))
    return detection


def update_game_network_runtime(game):
    """Perform an explicit, bounded local scan for one selected game."""
    detection = detect_network_runtime(game.get("path"))
    game["network_runtime"] = detection
    game["requires_photon"] = detection["photon"]
    game["requires_eos"] = detection["eos"]
    return detection


def check_is_photon(_steam_appid):
    # AppID databases are intentionally not used. Detection is based on the
    # game installation selected by the user, not on remote guessing.
    return False


def get_clean_env():
    env = os.environ.copy()
    env.pop('PYTHONHOME', None)
    env.pop('PYTHONPATH', None)
    return env

@app.route('/api/logs')
def api_logs():
    return jsonify(_logs)


@app.route('/api/games/<game_id>/launch', methods=['POST'])
def api_launch_game(game_id):
    data = request.get_json()
    mode = data.get('mode', 'crack_fix')

    games_data = load_games()
    game = next((g for g in games_data['games'] if g['id'] == game_id), None)

    if not game:
        return jsonify({"error": "Game not found"}), 404

    game_path = game.get('path')

    try:
        original_path = Path(game_path).resolve()
        backup_path = Path(str(original_path) + ".pack")
        target_exe = str(original_path)

        if mode == 'direct':
            subprocess.Popen([target_exe], cwd=str(original_path.parent), creationflags=0x08000000)
            game["last_played"] = int(time.time())
            save_games(games_data)
            return jsonify({"success": True, "message": "Launched in direct mode"})

        loader_path = str(get_loader_path())
        loader_dir = str(Path(loader_path).parent)

        if not Path(loader_path).exists():
            return jsonify({"error": f"Loader not found : {loader_path}"}), 400

        target_appid = "480"
        env = os.environ.copy()
        env["SteamAppId"] = target_appid
        env["SteamGameId"] = target_appid
        env["SteamEnv"] = "1"

        if UNPACKER_EXE.exists():
            try:
                unpacked_v1 = original_path.with_suffix('.exe.unpacked.exe')
                unpacked_v2 = original_path.with_name(original_path.stem + ".unpacked.exe")

                unpacked_found = unpacked_v1 if unpacked_v1.exists() else (
                    unpacked_v2 if unpacked_v2.exists() else None)

                if not unpacked_found:
                    print("[UNPACK] File not found. Launching unpacker...")
                    subprocess.run(
                        [str(UNPACKER_EXE), "--quiet", target_exe],
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                        cwd=str(UNPACKER_EXE.parent),
                        creationflags=0x08000000,
                        timeout=5
                    )

                    if unpacked_v1.exists():
                        unpacked_found = unpacked_v1
                    elif unpacked_v2.exists():
                        unpacked_found = unpacked_v2

                if unpacked_found and unpacked_found.exists():
                    print(f"[UNPACK] File found ({unpacked_found.name}). Swap immediately...")

                    if backup_path.exists():
                        try:
                            os.remove(str(backup_path))
                        except:
                            pass

                    os.rename(str(original_path), str(backup_path))
                    os.rename(str(unpacked_found), str(original_path))
                    print("[UNPACK] Swap complete.")
                else:
                    print("[UNPACK] No unpacked files were found and the unpacker generated nothing.")

            except Exception as unpack_e:
                print(f"[UNPACK ERROR] Quick error : {unpack_e}")


        target_dir = original_path.parent
        flask_root = Path(__file__).resolve().parent

        source_proxy = flask_root / "winmm.dll"
        source_fix = flask_root / "crypthook.dll"

        dest_proxy = target_dir / "winmm.dll"
        dest_fix = target_dir / "crypthook.dll"

        if source_proxy.exists():
            try:
                shutil.copy2(str(source_proxy), str(dest_proxy))
            except PermissionError:
                pass

        if source_fix.exists():
            try:
                shutil.copy2(str(source_fix), str(dest_fix))
            except PermissionError:
                pass

        subprocess.Popen(
            [loader_path, target_exe],
            cwd=loader_dir,
            env=env,
            creationflags=0x08000000
        )

        game["last_played"] = int(time.time())
        save_games(games_data)
        return jsonify({"success": True, "message": "Game launch"})

    except Exception as e:
        print(f"[LAUNCH ERROR] {traceback.format_exc()}")
        return jsonify({"error": str(e)}), 500


@app.route('/api/games/<game_id>/playtime', methods=['POST'])
def api_update_playtime(game_id):
    seconds = request.get_json().get('seconds', 0)
    data = load_games()
    for g in data['games']:
        if g['id'] == game_id:
            g['playtime'] = g.get('playtime', 0) + seconds
            break
    save_games(data)
    return jsonify({"success": True})

@app.route('/api/debug')
def api_debug():
    return jsonify({
        "executable": sys.executable,
        "frozen": getattr(sys, 'frozen', False),
        "meipass": getattr(sys, '_MEIPASS', None),
        "cwd": os.getcwd(),
        "path": os.environ.get('PATH', '')[:500]
    })

def scan_all_drives():
    steam_paths = []
    drives = []

    import string
    available_drives = ['%s:' % d for d in string.ascii_uppercase if os.path.exists('%s:' % d)]

    common_folders = [
        r"Program Files (x86)\Steam",
        r"Program Files\Steam",
        r"Steam",
        r"SteamLibrary",
        r"Games\Steam",
        r"Jeux\Steam"
    ]

    print(f"[DEBUG] Disks detected : {available_drives}")

    for drive in available_drives:
        for folder in common_folders:
            full_path = Path(drive) / folder
            if (full_path / "steamapps").exists():
                steam_paths.append(full_path)
                print(f"[INFO] Library found (Scan) : {full_path}")

    return steam_paths


@app.post("/api/steam/update-playtime")
def api_update_steam_playtime():
    data = load_games()
    settings = load_settings()
    steam_path = settings.get('steam_path', str(STEAM_PATH))
    steamid64 = settings.get("selected_steam_account")

    for game in data.get("games", []):
        appid = game.get("steam_appid")
        if appid:
            try:
                if steamid64:
                    game["playtime"] = get_playtime_for_account(steam_path, steamid64, str(appid))
                else:
                    game["playtime"] = 0
            except Exception as e:
                print(f"[WARN] Unable to recover playtime for {appid}: {e}")

    save_games(data)
    return jsonify({"success": True, "updated": len([g for g in data.get("games", []) if g.get("steam_appid")])})

@app.route('/api/import/steam', methods=['POST'])
def api_import_steam_fixed():
    settings = load_settings()
    settings["cached_playtime_accounts"] = []
    save_settings(settings)
    data = load_games()
    existing_appids = {g.get('steam_appid') for g in data['games'] if g.get('steam_appid')}
    imported_count = 0

    settings = load_settings()
    main_path = Path(settings.get('steam_path', str(STEAM_PATH)))

    libraries = []

    if main_path.exists():
        libraries.append(main_path)

    found_paths = scan_all_drives()
    for p in found_paths:
        if p not in libraries:
            libraries.append(p)

    for lib_path in libraries:
        steamapps_path = lib_path / "steamapps"
        if not steamapps_path.exists(): continue


        try:
            files = os.listdir(steamapps_path)
        except Exception as e:
            continue

        for fname in files:
            if fname.startswith("appmanifest_") and fname.endswith(".acf"):

                playtime = 0

                try:
                    with open(steamapps_path / fname, 'r', encoding='utf-8', errors='ignore') as f:
                        txt = f.read()

                    name_m = re.search(r'"name"\s+"([^"]+)"', txt, re.IGNORECASE)
                    id_m = re.search(r'"appid"\s+"(\d+)"', txt, re.IGNORECASE)
                    install_m = re.search(r'"installdir"\s+"([^"]+)"', txt, re.IGNORECASE)

                    if not name_m or not id_m: continue

                    name = name_m.group(1)
                    appid = id_m.group(1)

                    if appid in existing_appids: continue

                    install_dir_name = install_m.group(1) if install_m else ""
                    full_install_path = steamapps_path / "common" / install_dir_name
                    exe_path = find_game_executable(full_install_path, name)

                    playtime = get_local_steam_playtime(appid)

                    # Runtime detection is performed locally after the executable is resolved.
                    is_photon = False

                    data['games'].append({
                        "id": str(uuid.uuid4()),
                        "name": name,
                        "path": exe_path,
                        "arguments": "",
                        "steam_appid": appid,
                        "banner": download_steam_banner(appid),
                        "playtime": playtime,
                        "requires_photon": is_photon
                    })

                    existing_appids.add(appid)
                    imported_count += 1

                except Exception as e:
                    print(f"[Error] {fname}: {e}")

    if imported_count > 0:
        save_games(data)

    return jsonify({"success": True, "imported": imported_count})


@app.route('/api/get_cf_cookie', methods=['GET'])
def get_cf_cookie():
    cookie = load_cf_clearance_from_settings()
    return jsonify({"cf_clearance": cookie})

@app.route('/api/save_cf_cookie', methods=['POST'])
def api_save_cf_cookie():
    req_data = request.get_json()
    if not req_data:
        return jsonify({"error": "No data received"}), 400

    cookie = req_data.get('cf_clearance', '')
    save_cf_clearance_to_settings(cookie)

    return jsonify({"success": True, "message": "Cookie Cloudflare synchronisé"})

@app.route('/api/generate', methods=['POST'])
def api_generate():
    req_data = request.get_json()
    if not req_data:
        return jsonify({"error": "No data received"}), 400

    appid = req_data.get('appid')

    if not appid:
        return jsonify({"error": "AppID required"}), 400

    try:
        output_dir, game_name = process_appid(appid)
        if not output_dir:
            return jsonify({"error": game_name}), 500

        return jsonify({
            "success": True,
            "game": {"name": game_name},
            "output_dir": output_dir,
            "message": f"Files generated for {game_name}"
        })
    except Exception as e:
        return jsonify({"error": str(e)}), 500

@app.route('/api/lua/upload', methods=['POST'])
def api_lua_upload_custom():
    req_data = request.get_json()
    if not req_data:
        return jsonify({"error": "No data received"}), 400

    appid = req_data.get('appid')
    content = req_data.get('content')

    if not appid or not content:
        return jsonify({"error": "Missing AppID or content"}), 400

    try:
        lines = content.splitlines()
        cleaned_lines = [line for line in lines if "setmanifest" not in line]
        cleaned_content = "\n".join(cleaned_lines)

        steam_path = get_steam_install_path()
        output_dir = os.path.join(steam_path, "config", "lua")
        os.makedirs(output_dir, exist_ok=True)

        lua_file_path = os.path.join(output_dir, f"{appid}.lua")

        if os.path.exists(lua_file_path):
            os.remove(lua_file_path)
        with open(lua_file_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(cleaned_content)
        smart_sync_launchertools(steam_path)

        return jsonify({
            "success": True,
            "message": f".lua file installed for {appid} in {output_dir}"
        })

    except Exception as e:
        return jsonify({"error": str(e)}), 500



def find_game_executable(install_dir, game_name):
    install_dir = Path(install_dir)
    if not install_dir.exists():
        return ""

    blacklist = [
        "unitycrashhandler", "crashreport", "setup", "install",
        "uninstall", "vcredist", "python", "steamcmd", "clemtoutlauncher",
        "game_loader", "helper", "overlay", "config", "touch"
    ]

    all_exes = list(install_dir.rglob("*.exe"))
    deep_candidates = [e for e in all_exes if not any(bad in e.name.lower() for bad in blacklist)]

    if deep_candidates:
        shipping = [d for d in deep_candidates if "shipping" in d.name.lower()]
        if shipping:
            def clean_return(path_obj):
                return str(os.path.abspath(path_obj)).replace('\\', '/')
            return clean_return(max(shipping, key=lambda x: x.stat().st_size))

    root_exes = list(install_dir.glob("*.exe"))
    candidates = [exe for exe in root_exes if not any(bad in exe.name.lower() for bad in blacklist)]

    def clean_return(path_obj):
        return str(os.path.abspath(path_obj)).replace('\\', '/')

    if candidates:
        sanitized_name = re.sub(r'[^\w]', '', game_name).lower()

        for c in candidates:
            if sanitized_name in c.name.lower():
                return clean_return(c)

        chosen = max(candidates, key=lambda x: x.stat().st_size)
        return clean_return(chosen)

    if deep_candidates:
        return clean_return(max(deep_candidates, key=lambda x: x.stat().st_size))

    return ""

@app.route('/api/banners/<filename>')
def api_banner(filename):
    return send_from_directory(BANNERS_DIR, filename)

@app.route('/api/games/<game_id>/network-runtime', methods=['GET', 'POST'])
def api_game_network_runtime(game_id):
    games_data = load_games()
    game = next((item for item in games_data['games'] if item['id'] == game_id), None)
    if not game:
        return jsonify({"error": "Game not found"}), 404

    detection = update_game_network_runtime(game)
    if request.method == 'POST':
        save_games(games_data)
    return jsonify(detection)


@app.route('/api/network-runtime/scan', methods=['POST'])
def api_scan_network_runtimes():
    games_data = load_games()
    results = []
    for game in games_data['games']:
        detection = update_game_network_runtime(game)
        results.append({"id": game['id'], "name": game.get('name', ''), **detection})
    save_games(games_data)
    return jsonify({"success": True, "games": results})


@app.route('/api/photon/status')
def api_photon_status():
    # Compatibility endpoint retained for the existing UI. It reports the local
    # detector state; no hosts/DNS configuration is read or changed.
    settings = load_settings()
    configured_path = Path(settings.get('network_runtime_path', r"E:\photon\Loader"))
    return jsonify({
        "status": "scanner_ready",
        "active": True,
        "scanner": "local_installation",
        "configured_path": str(configured_path),
        "configured_path_exists": configured_path.is_dir(),
        "external_loader_started": False,
    })


@app.route('/api/photon/cache/clear', methods=['POST'])
def api_photon_cache_clear():
    # Kept for compatibility: a fresh scan is local and does not flush DNS.
    return api_scan_network_runtimes()


@app.route('/api/photon/db-status', methods=['GET'])
def api_photon_db_status():
    return jsonify({"loaded": True, "syncing": False, "source": "local_installation_scan"})



TARGET_DIR = r"%LOCALAPPDATA%\ClemtoutLauncher"
_is_installing = False
_install_lock = threading.Lock()


@app.route('/api/install/status', methods=['GET'])
def api_install_status():
    current_dir = os.path.abspath(os.path.dirname(sys.executable if getattr(sys, 'frozen', False) else __file__))
    target_dir_expanded = os.path.abspath(os.path.expandvars(TARGET_DIR))

    if current_dir.lower() == target_dir_expanded.lower():
        return jsonify({"status": "fix"})

    mode = get_launcher_mode_from_json()
    if mode not in ["fix", "portable", "ask"]:
        mode = "ask"

    return jsonify({"status": mode})


@app.route('/api/install/portable', methods=['POST'])
def api_install_portable():
    success = save_launcher_mode_to_json("portable")
    return jsonify({"success": success})

@app.route('/api/install/fix', methods=['POST'])
def api_install_fix():
    global _is_installing
    if not getattr(sys, 'frozen', False):
        return jsonify({"success": False, "error": "Must be compiled (.exe) to run Fix installation."})

    with _install_lock:
        if _is_installing:
            return jsonify({"success": True, "already_running": True})
        _is_installing = True

    try:
        target_dir_expanded = os.path.abspath(os.path.expandvars(TARGET_DIR))
        os.makedirs(target_dir_expanded, exist_ok=True)

        current_exe = os.path.abspath(sys.executable)
        target_launcher = os.path.join(target_dir_expanded, "clemtoutlauncher.exe")

        if current_exe.lower() != target_launcher.lower():
            shutil.copy2(current_exe, target_launcher)

        updater_api_url = "https://api.github.com/repos/reduro/0xst4ck/releases/latest"
        res = requests.get(updater_api_url, timeout=15).json()

        if 'assets' not in res or len(res['assets']) == 0:
            _is_installing = False
            return jsonify({"success": False, "error": "No update assets found on GitHub."})

        download_url = res['assets'][0]['browser_download_url']
        target_updater = os.path.join(target_dir_expanded, "update.exe")

        r = requests.get(download_url, timeout=30)
        if r.status_code == 200:
            with open(target_updater, "wb") as f:
                f.write(r.content)
        else:
            _is_installing = False
            return jsonify({"success": False, "error": f"Failed to download updater. HTTP {r.status_code}"})

        desktop_lnk = os.path.join(os.path.expanduser("~"), "Desktop", "clemtoutlauncher.lnk")
        start_menu_programs = os.path.join(os.environ["APPDATA"], "Microsoft", "Windows", "Start Menu", "Programs")
        os.makedirs(start_menu_programs, exist_ok=True)
        start_lnk = os.path.join(start_menu_programs, "clemtoutlauncher.lnk")

        def create_shortcut(target, lnk_path):
            ps_cmd = (
                f'$WshShell = New-Object -ComObject WScript.Shell; '
                f'$Shortcut = $WshShell.CreateShortcut("{lnk_path}"); '
                f'$Shortcut.TargetPath = "{target}"; '
                f'$Shortcut.IconLocation = "{target},0"; '
                f'$Shortcut.WorkingDirectory = "{os.path.dirname(target)}"; '
                f'$Shortcut.Save()'
            )
            subprocess.run(["powershell", "-Command", ps_cmd], creationflags=0x08000000, stdout=subprocess.DEVNULL,
                           stderr=subprocess.DEVNULL)

        create_shortcut(target_updater, desktop_lnk)
        create_shortcut(target_updater, start_lnk)

        save_launcher_mode_to_json("fix")

        def run_delayed_restart_and_cleanup():
            time.sleep(0.4)

            subprocess.Popen([target_launcher], cwd=target_dir_expanded)

            cleanup_cmd = f':loop\ntimeout /t 1 /nobreak >nul\ndel /f /q "{current_exe}"\nif exist "{current_exe}" goto loop'
            subprocess.Popen(["cmd.exe", "/c", cleanup_cmd], creationflags=0x08000000)

            os._exit(0)

        threading.Thread(target=run_delayed_restart_and_cleanup, daemon=True).start()

        _is_installing = False
        return jsonify({"success": True, "message": "Installation réussie, transfert vers le mode Fix..."})

    except Exception as e:
        _is_installing = False
        return jsonify({"success": False, "error": str(e)})


@app.route('/api/install/reinstall', methods=['POST'])
def api_install_reinstall():
    try:
        if not os.path.exists(SETTINGS_FILE):
            return jsonify({"success": False, "error": "Fichier settings introuvable"})

        with open(SETTINGS_FILE, "r+") as f:
            data = json.load(f)
            data["is_configured"] = False
            f.seek(0)
            json.dump(data, f, indent=4)
            f.truncate()
        return jsonify({"success": True})
    except Exception as e:
        return jsonify({"success": False, "error": str(e)})


@app.route('/api/updater/check-self', methods=['GET'])
def api_updater_check_self():
    if not getattr(sys, 'frozen', False):
        return jsonify({"update_started": False})

    try:
        mode = get_launcher_mode_from_json()
        if mode == "portable":
            return jsonify({"update_started": False})

        current_dir = os.path.dirname(sys.executable)
        updater_path = os.path.join(current_dir, "update.exe")

        if not os.path.exists(updater_path):
            return jsonify({"update_started": False})

        api_url = "https://api.github.com/repos/reduro/0xst4ck/releases/latest"
        response = requests.get(api_url, timeout=10).json()

        asset = response['assets'][0]
        download_url = asset['browser_download_url']
        github_hash = asset.get('digest')

        local_hash = ""
        if os.path.exists(updater_path):
            sha256_hash = hashlib.sha256()
            with open(updater_path, "rb") as f:
                for byte_block in iter(lambda: f.read(4096), b""):
                    sha256_hash.update(byte_block)
            local_hash = "sha256:" + sha256_hash.hexdigest()

        if local_hash != github_hash:
            def bg_download():
                try:
                    temp_path = updater_path + ".tmp"
                    r = requests.get(download_url, timeout=30)
                    if r.status_code == 200:
                        with open(temp_path, "wb") as f:
                            f.write(r.content)
                        if os.path.exists(updater_path):
                            os.remove(updater_path)
                        os.rename(temp_path, updater_path)
                except:
                    pass

            threading.Thread(target=bg_download, daemon=True).start()
            return jsonify({"update_started": True})

    except:
        pass

    return jsonify({"update_started": False})


if __name__ == '__main__':
    if "--cleanup" in sys.argv:
        try:
            idx = sys.argv.index("--cleanup")
            old_exe_path = sys.argv[idx + 1]

            time.sleep(0.4)
            if os.path.exists(old_exe_path):
                os.remove(old_exe_path)
        except Exception as e:
            print(f"Error during cleaning : {e}")

    port_to_use = find_free_port(8000)
    app.run(host='127.0.0.1', port=port_to_use)