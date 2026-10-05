import os
import subprocess
import shutil

def build():
    for folder in ['dist', 'build']:
        if os.path.exists(folder):
            shutil.rmtree(folder)

    result = subprocess.run(
        ['python', '-m', 'PyInstaller', 'Clemtoutlauncher.spec'],
        shell=True
    )

    if result.returncode == 0:
        print("\nGG")
    else:
        print("\nError")

if __name__ == "__main__":
    build()