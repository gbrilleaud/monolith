# BIOS, firmware et clés

## Politique

Monolith ne télécharge, ne fournit, ne récupère et ne redistribue publiquement aucun BIOS, firmware, clé de console, NAND, IPL ou fichier système propriétaire.

Une instance auto-hébergée peut conserver les dumps que son administrateur a créés légalement dans son stockage NAS privé. Elle peut les provisionner vers les seuls appareils explicitement autorisés du même réseau privé lors de leur installation. Cette copie privée ne doit jamais être exposée par une URL publique, incluse dans une release, déposée dans GitHub, transmise à un utilisateur non autorisé, ni servir de contenu de démonstration.

Le logiciel peut contrôler le nom, la taille et l'empreinte des fichiers. Les fichiers doivent provenir de dumps légalement créés depuis le matériel ou les médias de l'administrateur de l'instance.

## État des plateformes actuellement prévues

| Plateforme | Émulateur/cœur prévu | Fichiers propriétaires | État |
|---|---|---|---|
| Mega Drive / Genesis | RetroArch + Genesis Plus GX | aucun pour les cartouches ordinaires | prêt sans BIOS externe |
| Master System / Game Gear | RetroArch + Genesis Plus GX | aucun pour l'usage courant | prêt sans BIOS externe |
| PlayStation | DuckStation, repli SwanStation | BIOS `SCPH-xxxx` régional | requis avant déclaration de compatibilité |
| Saturn | RetroArch + Mednafen Saturn | BIOS Saturn régional | requis avant déclaration de compatibilité |
| Mega-CD / Sega CD, si ajouté | Genesis Plus GX | BIOS régionaux `bios_CD_J.bin`, `bios_CD_U.bin`, `bios_CD_E.bin` | hors périmètre actuel |
| Dreamcast, si ajoutée | Flycast | `dc_boot.bin`, `dc_flash.bin` | hors périmètre actuel |
| Game Boy Advance | mGBA | `gba_bios.bin` facultatif pour une fidélité maximale | non bloquant |
| GameCube / Wii, si ajoutées | Dolphin | IPL, NAND, clés et fichiers système selon les usages | hors périmètre actuel |
| Switch, hors périmètre | aucun | `prod.keys`, `title.keys`, firmware, NAND | jamais distribué par Monolith |

## Stockage serveur privé et déploiement client

Une instance personnelle peut conserver un exemplaire de référence sur son NAS privé, hors du dépôt :

```text
<stockage-privé>/bios/
├── sega_saturn/
├── sega_dreamcast/
├── nintendo_game_boy_advance/
├── sony_playstation/
└── nintendo_gamecube/
```

Le serveur ne doit proposer ce contenu qu'après authentification et autorisation explicite d'un appareil appartenant à l'instance. Le transfert doit utiliser un canal authentifié, enregistrer un audit non sensible et vérifier l'empreinte après copie. Le client installe ensuite les fichiers dans son répertoire de données local, par exemple :

```text
<répertoire-de-données-client>/firmware/<plateforme>/
```

Une plateforme qui nécessite un BIOS n'est considérée comme prête à lancer qu'après validation locale du nom, de la taille et de l'empreinte du fichier provisionné. Une instance distribuée à un tiers possède la même arborescence logique, mais son répertoire `bios/` est vide : son administrateur doit y ajouter ses propres dumps.

Ne jamais placer ces fichiers sous `src/`, `tests/`, `docs/`, `config/`, dans une release publique ou dans Git.
