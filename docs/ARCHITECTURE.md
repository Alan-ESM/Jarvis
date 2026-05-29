# Architecture Jarvis

## Positionnement

Jarvis est une application desktop locale pour Windows. Elle n'est pas une application web. Le coeur tourne sur le PC, garde une journalisation locale, controle les acces fichiers, execute des travaux dans des sandboxes temporaires et utilise Internet pour les reponses IA, la recherche et les modeles externes.

## Stack recommandee

Stack principale:

- Rust pour le coeur: orchestration, securite, acces systeme, sandbox, fichiers, logs, configuration, audit.
- egui/eframe pour l'UI native initiale: executable desktop Windows, rendu GPU, animations legeres, pas de navigateur embarque.
- Windows API via crate `windows` dans les prochaines etapes: Event Log, Job Objects, services, processus, registre.
- Python uniquement comme worker optionnel de taches IA/data dans sandbox, jamais comme coeur securite.
- Google Custom Search JSON API pour la recherche Google officielle.
- API IA OpenAI-compatible via configuration: base URL, modele Flash, X, Ultra, cle en variable d'environnement.

Pourquoi Rust:

- controle memoire et concurrence robuste;
- tres bon packaging en `.exe`;
- acces natif aux APIs Windows;
- surface d'attaque plus faible qu'une app Electron;
- modules facilement testables.

## Processus

Version initiale:

- `jarvis-app.exe`: UI native + superviseur in-process.

Version produit avancee:

- `jarvis-app.exe`: shell graphique.
- `jarvis-core.exe`: daemon local optionnel avec IPC nomme.
- `jarvis-runner.exe`: worker sandbox ephemeral pour builds, tests et commandes.

## Modules

- `jarvis-app`: interface desktop, conversation, sidebar, panels, animation de chargement.
- `jarvis-core`: superviseur, classification d'intention, routage multi-modeles, fallback.
- `jarvis-providers`: fournisseurs IA, routes Flash/X/Ultra, client OpenAI-compatible.
- `jarvis-tools`: fichiers, Git, Google Search, micro.
- `jarvis-system`: Internet gate, logs, processus, inspection PC.
- `jarvis-sandbox`: execution isolee, timeout, repertoire temporaire, futur Job Object Windows.
- `jarvis-audit`: journalisation obligatoire des actions sensibles.
- `jarvis-config`: configuration TOML + variables d'environnement.

Voir aussi `docs/GITHUB_INSPIRATION.md` pour les projets open source observes et les decisions adaptees a Jarvis.

## Routage multi-modeles

Niveaux:

- Flash: classification, triage, reponses simples, reformulation courte.
- X: raisonnement, planification, decomposition, analyse.
- Ultra: synthese finale, generation de code, decisions critiques.

Pipeline:

1. Verifier Internet.
2. Classifier l'intention.
3. Choisir une chaine: Flash seul, Flash -> X, ou Flash -> X -> Ultra.
4. Appeler les modeles avec contexte minimal utile.
5. Evaluer la qualite.
6. Escalader si score faible ou erreur.
7. Retourner une reponse auditee.

## Internet obligatoire

Si Internet est absent:

- aucun appel IA;
- aucune recherche Google;
- aucune transcription cloud;
- mode local lecture seule;
- UI explicite: etat offline, actions sensibles bloquees.

Les modules locaux restent disponibles pour inspection deja autorisee, historique, configuration et consultation des logs existants.

## Fichiers

Trois niveaux:

- `intermediate`: demande de permission pour chaque fichier lu ou modifie.
- `unlimited`: acces total avec audit obligatoire.
- `disabled`: aucune ecriture ni execution; lecture seulement si explicitement autorisee.

Toutes les operations passent par `FileAccessController`.

## Sandbox

Le sandbox initial cree un repertoire temporaire, execute une commande avec timeout, capture stdout/stderr et nettoie. L'etape Windows avancee ajoute:

- Windows Job Objects;
- limites CPU/memoire/processus;
- token restreint;
- low integrity;
- AppContainer ou Windows Sandbox pour les executions a haut risque;
- reseau desactive par defaut sauf autorisation.

## Logs et inspection PC

Logs:

- Windows Event Log via `wevtutil` au debut, puis API native Windows Event Log.
- Logs applicatifs texte/JSON.
- Detection d'erreurs: error, failed, exception, panic, denied, timeout.

Inspection PC en mode `unlimited`:

- processus;
- services;
- applications installees;
- chemins de configuration;
- logs systeme;
- variables d'environnement pertinentes;
- synthese comprehensible pour l'utilisateur.

## Micro

Flux prevu:

1. Push-to-talk dans l'UI.
2. Capture WASAPI/CPAL.
3. Transcription via provider configure.
4. Envoi au superviseur.
5. Audit: activation, duree, provider, resultat technique.

## UI

Surfaces:

- conversation centrale;
- sidebar: taches, memoire, outils, fichiers;
- bouton micro;
- indicateur modele actif;
- animation pendant generation;
- panels: configuration, logs, sandbox, fichiers;
- style sombre premium, accents cyan/acier/vert, sans page marketing.

## Extensions futures

- plugins d'outils;
- memoire vectorielle locale chiffree;
- connecteur GitHub complet;
- mode agent projet;
- politiques par dossier;
- RBAC local;
- signatures de commandes autorisees;
- telemetry locale optionnelle.
