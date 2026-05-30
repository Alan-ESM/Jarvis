# Inspirations GitHub pour Jarvis

Objectif: s'inspirer de plusieurs projets open source sans copier un backend entier ni dependre d'une seule architecture.

## Projets observes

- Accomplish: agent desktop local-first avec controle des dossiers, logs et actions approuvees.
  - Repo: https://github.com/accomplish-ai/accomplish
  - A retenir: permissions visibles, clefs dans le coffre OS, daemon long-lived separe de l'UI.

- gptme: agent terminal provider-agnostic avec shell, Python, web, vision et plugin system.
  - Repo: https://github.com/gptme/gptme
  - A retenir: outils locaux explicites, historique de commandes, architecture extensible.

- DecisionsAI: assistant vocal desktop, STT/TTS local et commandes systeme.
  - Repo: https://github.com/tensology/decisionsai
  - A retenir: push-to-talk clair, etats vocaux visibles, enregistrement/transcription separes.

- Captain Claw: orchestration multi-agent, dashboard, DAG, web search et outils nombreux.
  - Repo: https://github.com/kstevica/captain-claw
  - A retenir: superviseur, agents specialises, retries, validation structuree, outil web search.

- Agentify Desktop: controle local de sessions IA web via MCP, fichiers locaux, tabs paralleles.
  - Repo: https://github.com/agentify-sh/desktop
  - A retenir: stable tab keys, automation locale, pause manuelle quand une verification humaine est requise.

- Open.Jarvis: assistant Windows-first avec commandes vocales/texte, routage local, fallback Groq et interface cyber.
  - Repo: https://github.com/dmrr35/Open.Jarvis
  - A retenir: mode degrade sans cle, diagnostics visibles, provider fallback controle par l'utilisateur.

- OpenAkita: assistant multi-plateforme avec desktop Tauri, agents, memoire, skills et MCP.
  - Repo: https://github.com/openakita/openakita
  - A retenir: agents/skills modulaires, UX sans ligne de commande, separation app desktop / service.

- Thoth: assistant local-first avec memoire, voice, vision, shell, navigateur et modeles locaux/cloud optionnels.
  - Repo: https://github.com/siddsachar/Thoth
  - A retenir: clefs dans coffre OS, onboarding installeur, voix et notifications comme surfaces produit.

- OpenCrust: framework agent Rust leger, multi-canal, credential storage chiffre, config hot-reload.
  - Repo: https://github.com/opencrust-org/opencrust
  - A retenir: Rust pour fiabilite/securite, stockage chiffre, runtime leger et extensible.

- thClaws: workspace agent natif Rust, multi-provider, terminal/chat/fichiers, detection de providers par modele.
  - Repo: https://github.com/thClaws/thClaws
  - A retenir: prefixes/provider slots, UI native avec surfaces Terminal/Chat/Fichiers, orchestration locale.

- OpenPawz: desktop Tauri/Rust offline-first avec guardrails, memoire hybride, providers compatibles et execution d'outils.
  - Repo: https://github.com/OpenPawz/openpawz
  - A retenir: SQLite chiffre, OS keychain, sandbox Docker, separation IPC typed frontend/engine.

- Omi: assistant qui capture audio/ecran, transcrit, resume et transforme en actions.
  - Repo: https://github.com/BasedHardware/omi
  - A retenir: transcription continue, resume actionnable, distinction capture locale / synthese cloud.

- Open Codex: assistant CLI simple, local-first, confirmation avant execution.
  - Repo: https://github.com/codingmoh/open-codex
  - A retenir: commande proposee avant execution, garde-fous lisibles.

## Adaptation pour Jarvis

Jarvis garde Rust comme coeur natif Windows, puis emprunte les idees suivantes:

- UI desktop locale, pas une web app.
- Permissions visibles et modifiables a tout moment.
- Uploads retenus dans le prompt tant que l'utilisateur n'envoie pas.
- Superviseur multi-modeles avec fallback et journalisation.
- Recherche web comme outil explicite, avec configuration par variables d'environnement.
- Micro: enregistrement local separe de la future transcription.
- Terminal accessible mais jamais silencieusement cache.
- Futur daemon local possible pour separer UI et execution.
- Providers IA branches par tiers (`Flash`, `X`, `Ultra`) et variables d'environnement, pas par cle en dur.
- Recherche Google concatenee au contexte IA pour rendre une reponse texte lisible.
- Sidebar utilisateur reduite aux actions non dangereuses; panneaux internes conserves cote architecture.

## Ce qui n'est pas copie

- Aucun backend tiers n'a ete importe tel quel.
- Aucun code de ces projets n'a ete colle dans Jarvis.
- Les licences restent a verifier avant toute reutilisation directe future.
