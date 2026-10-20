# Moteurs : audit de distribution et état vérifié

État du dépôt au départ : `1ddc48c` ; tag public `v0.1.0` : `ca056d3`.
Ce document distingue les moteurs **référencés**, leur présence effective et leur capacité à lancer un jeu. Une scène graphique Flutter n'est pas une association à un moteur.

| Composant | Référence actuelle | Association réelle au départ | Situation de la distribution |
|---|---|---|---|
| RetroArch | 1.20.0 Windows x64, setup interactif | Profil TOML seulement ; cœur Genesis Plus GX explicitement nommé pour le système 11 | Aucune archive officielle Monolith ne contient RetroArch ni ses cœurs. Le setup ne prouve ni le chemin installé ni le cœur disponible. |
| PCSX2 | 2.8.0 Windows x64, setup interactif | Aucun profil automatique et aucun identifiant PS2 canonique | Aucune archive Monolith ne contient PCSX2 ; le BIOS PS2 de l'utilisateur reste séparé. |
| Xenia Canary | URL `releases/latest` de l'archive Windows | Aucun profil automatique et aucun identifiant Xbox 360 canonique | URL non reproductible, téléchargement non vérifié par empreinte, aucune archive Monolith ne contient Xenia. |
| DuckStation / SwanStation / Mednafen Saturn / mGBA / Flycast / Dolphin | Références documentaires dans `BIOS_SETUP.md` | Aucun installateur ou profil actif prouvé | Ne pas les présenter comme des moteurs actuellement pris en charge. |

Les identifiants des systèmes proviennent du catalogue SQLite ; hors du système 11, ils ne forment pas un registre stable. Une association basée sur un ID présumé pourrait lancer le mauvais moteur. Un catalogue de moteurs destiné à couvrir tous les systèmes doit d'abord définir et migrer une identité stable.

## Séparation impérative

- Binaires remplaçables : sous `<dossier de Monolith>/tools/`, sauf racine absolue explicitement configurée.
- Archives téléchargées : sous une zone de téléchargement distincte, jamais dans le moteur à extraire.
- Données personnelles : `<MONOLITH_DATA_DIR>` (`emulators.toml`, `controllers.toml`, ROMs locales, BIOS personnels, sauvegardes et réglages). Ne jamais les purger lors d'une réparation ou d'une mise à jour.
- Le lancement d'un moteur portable doit définir explicitement son répertoire de travail. La seule présence d'un exécutable et d'un cœur ne constitue pas une vérification de leur intégrité ou de leur fonctionnement.

## Limites des binaires publics audités

Les six assets de la release GitHub `v0.1.0` comprennent clients egui Linux/Windows et bundle Flutter Windows, plus leurs SHA-256. Le workflow de release compile le tag, et les fichiers README/configuration extraits correspondent au tag (différences de fins de ligne Windows). Les sommes des archives ont été vérifiées ; cela n'est **pas** une attestation reproductible octet pour octet des exécutables. Aucune archive ne fournit les moteurs, les cœurs, les licences de ces composants ou les BIOS.

## Garde-fous de publication

Ne pas annoncer de distribution « complète » ou « légère » sans manifeste verrouillé par plateforme/architecture, SHA-256 de chaque composant et dépendance, licences et accès aux sources exigées, tests d'extraction, migration, sauvegarde et lancement sur Windows propre. RetroArch et PCSX2 sont soumis à leurs licences GPL et aux obligations de redistribution associées ; les licences des cœurs et des dépendances doivent être vérifiées séparément. En particulier [la licence officielle de Genesis Plus GX](https://github.com/ekeeke/Genesis-Plus-GX/blob/master/LICENSE.txt) interdit la vente et l'usage dans un produit ou une activité commerciale, impose la reproduction du texte et, pour une version modifiée, le code source complet. Un paquet contenant ce cœur ne peut donc pas être présenté comme librement redistribuable sans restriction. Ne pas remplacer un lien `latest` par un nom de fichier supposé stable. Aucun contenu propriétaire (ROM, BIOS, firmware, clé) ne doit entrer dans les archives publiques.

Pour les jeux Genesis/Mega Drive ordinaires, aucun BIOS externe n'est requis ; cela ne signifie pas que RetroArch et son cœur ont été installés et validés. Pour PS2 et les systèmes à firmware requis, une détection et un import locaux sont nécessaires avant d'afficher « prêt à jouer ».

## Évaluation hors ligne (non publiée)

`config/engine-lock.json` épingle quatre archives officielles Windows x64, avec empreinte SHA-256 et taille. `scripts/build_engine_bundle.py` vérifie toutes les empreintes avant extraction et compose le payload `tools/` : RetroArch complet et son seul cœur Genesis Plus GX (usage non commercial), PCSX2, Xenia Canary. Les notices RetroArch/Genesis sont incluses, de même que les notices présentes dans les archives PCSX2/Xenia. Le ZIP technique obtenu localement a passé `zipfile -t` ; aucun lancement Windows réel n'en résulte.

`scripts/assemble_offline_windows.py` ajoute ce payload à une archive cliente egui ou Flutter sans changer les archives légères existantes (les updaters n'acceptent pas un paquet moteur). Le workflow manuel `.github/workflows/offline-windows-validation.yml` vise deux artifacts d'évaluation Windows, **sans publication GitHub Release automatique**. Il n'a pas encore été exécuté : compiler et produire un artifact CI ne prouvera pas les lancements Windows.

Le lanceur partagé prépare RetroArch pour Mega Drive/Master System/Game Gear et sait construire une commande PCSX2 avec `-datapath` hors des binaires après présence d'un BIOS `.bin` personnel. Cette détection par extension n'atteste pas la validité du BIOS. Xenia est seulement empaqueté, sans association automatique ni séparation validée de ses sauvegardes ; son runtime Microsoft VC++ externe est encore à auditer. Les notices de dépendances et l'offre de code source correspondant GPL doivent être finalisées avant une distribution publique. La distribution légère et les tests sur un Windows propre restent à faire.
