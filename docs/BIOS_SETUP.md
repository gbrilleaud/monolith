# BIOS, firmware et clés

## Politique

Monolith ne télécharge, ne fournit, ne récupère et ne redistribue aucun BIOS, firmware, clé de console, NAND, IPL ou fichier système propriétaire.

Le logiciel peut uniquement signaler qu'un fichier est requis et, si l'utilisateur l'a fourni localement, contrôler son nom, sa taille et son empreinte. Les fichiers doivent provenir de dumps légalement créés depuis le matériel ou les médias de l'utilisateur.

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

## Règle de validation

Une plateforme qui nécessite un BIOS n'est pas considérée comme prête à lancer tant que le client n'a pas validé localement le fichier requis. Une validation réussie ne transfère jamais ce fichier au backend ni au NAS partagé.

## Disposition locale recommandée

Les fichiers privés restent dans un emplacement local exclu de Git, par exemple :

```text
<répertoire-de-données-client>/firmware/<plateforme>/
```

Le chemin et l'empreinte peuvent être référencés dans une configuration locale non versionnée. Ne jamais placer ces fichiers sous `src/`, `tests/`, `docs/`, `config/` ou une release publique.
