# Jarvis

Jarvis est une application desktop native Windows, installee en `.exe`, concue comme un assistant IA local, modulaire et extensible.

Ce depot contient le socle initial du produit:

- UI desktop native Rust, sans Electron et sans application web.
- Superviseur multi-modeles: Flash, X et Ultra.
- Garde Internet: les reponses IA exigent une connexion active.
- Systeme de permissions fichiers: intermediaire, illimite et desactive.
- Journalisation d'audit en JSONL.
- Modules pour recherche Google, micro, logs Windows, inspection PC, Git et sandbox.
- Plan de packaging Windows `.exe` et futur installateur.

Voir:

- `docs/ARCHITECTURE.md`
- `docs/SECURITY_MODEL.md`
- `docs/PACKAGING_WINDOWS.md`

## Demarrage developpement

Installer Rust et les Build Tools Visual Studio C++ puis lancer:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\cargo-dev.ps1 check
powershell -ExecutionPolicy Bypass -File .\scripts\cargo-dev.ps1 build -p jarvis-app
powershell -ExecutionPolicy Bypass -File .\scripts\cargo-dev.ps1 run -p jarvis-app
```

Le script `scripts/cargo-dev.ps1` detecte MSVC et le Windows SDK, puis place le dossier de build dans `%LOCALAPPDATA%\Jarvis\target` pour eviter les problemes de compilation dans OneDrive.

La configuration se fait via `JARVIS_CONFIG` ou `config/jarvis.example.toml`. Les cles API doivent rester dans les variables d'environnement, jamais dans le code.

Important: si une cle API a ete collee dans un chat, considere-la comme compromise et regenere-la cote fournisseur avant de l'utiliser dans Jarvis.

## Raccourcis

- `Ctrl+Enter`: envoyer le message.
- `Ctrl+O`: uploader des fichiers.
- `Ctrl+N`: nouveau clavardage.
- `Ctrl+K`: ouvrir Recherche.
- `Ctrl+L`: revenir au champ de saisie.
- `Ctrl+T`: ouvrir un terminal dans le dossier du projet.
- `Ctrl+M`: demarrer/arreter l'enregistrement micro.
- `Enter` ou `Escape`: passer le portail de demarrage.

## Recherche Google

La recherche utilise Google Custom Search quand ces variables existent:

```powershell
$env:GOOGLE_SEARCH_API_KEY="..."
$env:GOOGLE_SEARCH_ENGINE_ID="..."
```

Sans ces variables, Jarvis affiche un message de configuration au lieu de faire semblant d'avoir cherche.

## Transcription vocale

Le bouton micro sauvegarde un WAV local, puis tente une transcription Hugging Face si ces variables existent:

```powershell
$env:HUGGINGFACE_API_TOKEN="..."
$env:HUGGINGFACE_TRANSCRIBE_MODEL="openai/whisper-large-v3-turbo"
```

Sans token, Jarvis garde l'audio localement et affiche clairement que la transcription est desactivee.

## Inspiration visuelle Canva

Canva a ete utilise pour generer des pistes d'aurores boreales realistes pour le portail et les fonds:

- https://www.canva.com/d/Y8YZtA-yL2zcMDe
- https://www.canva.com/d/ngm7tcN2KAF2VPc
- https://www.canva.com/d/IW0fz-Q7-8YyuPz
- https://www.canva.com/d/Ol3K7ihKPnLRee1

## Langages utilises

- Rust: application desktop native, UI, orchestration locale, securite, micro, recherche, sandbox.
- TOML: configuration.
- PowerShell: script de build Windows.
- Markdown: documentation.

MSVC et Windows SDK servent uniquement a compiler l'executable Windows.
