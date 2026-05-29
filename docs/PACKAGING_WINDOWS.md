# Packaging Windows

## Prerequis

Installer:

- Rust stable via rustup;
- Visual Studio Build Tools avec workload C++ Desktop;
- WiX Toolset ou NSIS pour un installateur;
- signtool pour la signature code si distribution publique.

## Build developpement

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\cargo-dev.ps1 run -p jarvis-app
```

## Build release

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\cargo-dev.ps1 build --release -p jarvis-app
```

Executable:

```text
%LOCALAPPDATA%/Jarvis/target/release/jarvis-app.exe
```

## Installateur

Plan recommande:

1. Construire `jarvis-app.exe`.
2. Copier `config/jarvis.example.toml`.
3. Creer `%APPDATA%/Jarvis`.
4. Installer raccourci menu demarrer.
5. Ajouter option lancement au demarrage.
6. Signer l'executable.
7. Generer MSI/EXE via WiX ou NSIS.

## Durcissement release

- Activer `windows_subsystem = "windows"` pour masquer la console.
- Signer le binaire.
- Journaliser dans `%LOCALAPPDATA%/Jarvis/logs`.
- Stocker les secrets dans les variables d'environnement ou Windows Credential Manager.
- Verifier que le mode `disabled` bloque ecriture et execution.
