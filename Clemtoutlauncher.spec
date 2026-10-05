# -*- mode: python ; coding: utf-8 -*-
import os
from PyInstaller.utils.hooks import collect_all

datas = [('frontend', 'frontend'), ('backend/winmm.dll', 'backend'), ('backend/crypthook.dll', 'backend'), ('backend/app.py', 'backend'), ('backend/game_loader.exe', '.'), ('backend/steam_api64.dll', '.'), ('backend/steamcmd', 'steamcmd'), ('backend/Unpacker', 'Unpacker'), ('backend/launchertools', 'launchertools')]
datas = [(source, destination) for source, destination in datas if os.path.exists(source)]
binaries = []
hiddenimports = []
tmp_ret = collect_all('flask')
datas += tmp_ret[0]; binaries += tmp_ret[1]; hiddenimports += tmp_ret[2]


a = Analysis(
    ['launcher.py'],
    pathex=[],
    binaries=binaries,
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=['_bootlocale'],
    noarchive=False,
    optimize=0,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='Clemtoutlauncher',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=['frontend\\favicon.ico'],
)
