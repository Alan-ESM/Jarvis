# Modele de securite

## Principes

- Aucun secret dans le code.
- Audit obligatoire pour toute action sensible.
- Le mode d'acces fichier est applique par un module central, pas par l'UI.
- Les appels IA sont bloques hors ligne.
- Les commandes externes passent par le sandbox.

## Niveaux fichiers

### Intermediate

Chaque lecture ou modification demande une autorisation explicite. La reponse utilisateur est journalisee.

### Unlimited

Jarvis peut lire, modifier, inspecter et ouvrir sans redemander. Toutes les actions restent journalisees. Ce mode est fait pour le controle total.

### Disabled

Aucune modification, suppression ou execution. La lecture exige une permission explicite.

## Audit

Format: JSONL.

Chaque evenement contient:

- id;
- timestamp UTC;
- acteur;
- action;
- cible;
- decision;
- metadata.

## Sandbox

Le runner doit etre temporaire, limite et nettoye. La version Windows avancee doit utiliser Job Objects et token restreint pour eviter qu'un build ou script casse le systeme.

## Garde Internet

Sans Internet:

- pas de provider IA;
- pas de recherche Google;
- pas de transcription cloud;
- pas d'ecriture automatique;
- UI en lecture seule.
