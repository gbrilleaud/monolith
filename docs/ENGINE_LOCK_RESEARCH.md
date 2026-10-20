# Recherche de verrouillage des moteurs

Vérification effectuée le **2026-10-18T17:25:41Z** sur les publications
officielles, sans ROM, BIOS, firmware ni clé. Les SHA-256 marqués « API +
local » proviennent du champ `digest` de l'API GitHub et ont aussi été
recalculés sur l'asset téléchargé. Ceux marqués « local » ont été calculés
deux fois sur l'asset servi, mais ne sont pas attestés par l'amont.

## Conclusion exploitable

- **PCSX2 2.8.0** fournit les deux artefacts portables x86-64 recherchés. Sa
  release GitHub est déclarée `immutable: true`, et ses SHA-256 officiels ont
  été confirmés localement [S5].
- **Xenia Canary** fournit désormais officiellement Windows **et** Linux pour
  le commit épinglé `f03a9fcfcb2f30b3b958d0e5a8e6757cc2cf72fb`. Les digests
  GitHub ont été confirmés localement, mais la release est déclarée
  `immutable: false` : le SHA-256 reste donc impératif [S8].
- **RetroArch 1.20.0** fournit des archives portables versionnées Windows et
  Linux et des packs de cœurs séparés [S1][S2]. Le buildbot ne publie aucun
  fichier `.sha256` adjacent (404 constatée) : les sommes ci-dessous sont des
  mesures locales, pas une attestation amont.
- **Genesis Plus GX** est bien présent dans chacun des deux packs 1.20.0. Les
  binaires incorporent `v1.7.4 3c16987`; l'API GitHub résout ce préfixe vers
  le commit complet `3c1698778080541927f3d7011a00d2c9efe545af` [S3]. Sa
  licence interdit vente et activité/produit commercial [S4]. Le pack complet
  contient près de 200 cœurs aux licences diverses : il ne doit pas être
  redistribué comme substitut non audité au seul cœur Genesis Plus GX.
- **Aucun `config/engine-lock.example.json` n'est créé.** Tous les assets
  exécutables officiels retenus sont en 7z ou AppImage, tandis que
  `install_engine_package` n'accepte actuellement que ZIP. Les packs de cœurs
  RetroArch dépassent aussi la limite décompressée par défaut de 1 Gio, et le
  statut commercial autorisé par Genesis Plus GX n'est pas établi. Produire un
  manifeste d'installation aurait donc été trompeur malgré des sommes exactes.

## Verrous binaires exacts

| Composant | Hôte / format | URL officielle épinglée | Octets | SHA-256 | Provenance |
|---|---|---|---:|---|---|
| RetroArch 1.20.0 | Windows x86-64 / 7z portable | `https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z` | 196477554 | `968b5df7130ce9aabb45b41661e811a77eca983d78c1e1e1d4f5c14fcc58c979` | local, archive retestée par 7-Zip |
| Cœurs RetroArch 1.20.0 | Windows x86-64 / 7z | `https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch_cores.7z` | 223995512 | `3cfd93f0392feadd47f2d1d3135d196ea2a50126235845209b740df5358a5019` | local, archive retestée par 7-Zip |
| RetroArch 1.20.0 | Linux x86-64 / 7z contenant AppImage + home portable | `https://buildbot.libretro.com/stable/1.20.0/linux/x86_64/RetroArch.7z` | 175915300 | `809b3e9f02a9849719453d0f189a0edc544ad3235c8ce75a79488e710ba9668a` | local, archive retestée par 7-Zip |
| Cœurs RetroArch 1.20.0 | Linux x86-64 / 7z | `https://buildbot.libretro.com/stable/1.20.0/linux/x86_64/RetroArch_cores.7z` | 263449895 | `433f46ecbb3f1ae8d17b6c5fdffbd11761ce4532652d73a1d4e1f0029c175a87` | local, archive retestée par 7-Zip |
| PCSX2 2.8.0 | Windows x86-64 / 7z portable | `https://github.com/PCSX2/pcsx2/releases/download/v2.8.0/pcsx2-v2.8.0-windows-x64-Qt.7z` | 25664243 | `8a475119fc5950439be64e7b4d0a86b71c25aa39e19346fca7b889c6dbb6d839` | API GitHub asset `534396712` + local [S5] |
| PCSX2 2.8.0 | Linux x86-64 / AppImage | `https://github.com/PCSX2/pcsx2/releases/download/v2.8.0/pcsx2-v2.8.0-linux-appimage-x64-Qt.AppImage` | 60115448 | `f0e36b31fc885c9ccdb5339274f1d998a9e69bf736b92dd8674134091f992a3f` | API GitHub asset `534396714` + local [S5] |
| Xenia Canary `f03a9fc` | Windows x86-64 / 7z portable | `https://github.com/xenia-canary/xenia-canary/releases/download/f03a9fc/xenia_canary_windows.7z` | 3680825 | `96cd96982aeaa27834ad14e973e299fd2f7b6bf2deb67e451db6a490b0e43bf3` | API GitHub asset `652288660` + local [S8] |
| Xenia Canary `f03a9fc` | Linux x86-64 / AppImage | `https://github.com/xenia-canary/xenia-canary/releases/download/f03a9fc/xenia_canary_linux.AppImage` | 18807288 | `57926e8e24e8f85035f6c3c2e245fb40f6a9a0b73ca0e42a982104c2577f11bf` | API GitHub asset `652288656` + local [S8] |

### Cœurs Genesis Plus GX extraits

Ces sommes portent sur les fichiers internes, pas sur une archive officielle
individuelle :

| Hôte | Chemin exact dans le pack | Octets | SHA-256 du cœur |
|---|---|---:|---|
| Windows | `RetroArch-Win64/cores/genesis_plus_gx_libretro.dll` | 8378896 | `3af1aeafb1a0bcf838dc5dc31ae3ecaac369a13ed02f80223d73a87749d8a2ce` |
| Linux | `RetroArch-Linux-x86_64/RetroArch-Linux-x86_64.AppImage.home/.config/retroarch/cores/genesis_plus_gx_libretro.so` | 12272856 | `8207771e3ba1f200c2a79d238693743e9f0c73d53c4825e63f6d4cf5cae669b2` |

Le `.so` est un ELF x86-64 dynamique qui ne réclame directement que
`libm.so.6` et `libc.so.6`; la DLL est une PE32+ x86-64. Les deux affichent la
même révision incorporée `v1.7.4 3c16987`.

## Disposition et dépendances constatées

- **RetroArch Windows** : racine `RetroArch-Win64/`, exécutable
  `retroarch.exe`, ressources/configuration incluses, et nombreuses DLL
  adjacentes (Qt 5, SDL2, FFmpeg, OpenSSL, codecs). Le pack des cœurs réutilise
  exactement cette racine. L'archive principale compte 14 970 entrées et
  environ 536,8 Mo décompressés.
- **RetroArch Linux** : la 7z contient
  `RetroArch-Linux-x86_64/RetroArch-Linux-x86_64.AppImage` (9 868 480 octets,
  SHA-256 interne
  `63ae618d34e64c36d8ad68b82de2a6b85d7e951f05d38558a4dfb534c4a852f4`)
  et le dossier compagnon `.AppImage.home/.config/retroarch/`. Le pack de
  cœurs cible ce dernier. L'AppImage embarque beaucoup de bibliothèques et
  leurs notices, mais conserve des dépendances hôte graphiques/audio
  (`libGL`, `libEGL`, ALSA, JACK, X11/Wayland, DRM/GBM, etc.).
- **Packs de cœurs RetroArch** : environ 1,43 Go décompressé sous Windows et
  1,60 Go sous Linux. Ils dépassent donc `InstallLimits::max_total_bytes`
  (1 Gio) et mélangent les licences de tous les cœurs.
- **PCSX2 Windows** : 259 entrées, environ 119 Mo décompressés ;
  `pcsx2-qt.exe` est à la racine avec Qt 6, SDL3, FFmpeg, Vulkan/shaderc et les
  autres DLL. `docs/GPL.html` et `docs/ThirdPartyLicenses.html` sont inclus.
  Aucun marqueur de données portables précréé n'a été trouvé : le mode de
  stockage des réglages doit être validé séparément avant intégration.
- **PCSX2 Linux** : l'AppImage s'extrait correctement ; exécutable
  `usr/bin/pcsx2-qt`, Qt 6/SDL3/FFmpeg et de nombreuses bibliothèques
  embarquées, notices Debian et `resources/docs/{GPL,ThirdPartyLicenses}.html`.
  Des bibliothèques système restent requises, notamment libc/libstdc++, GLX /
  OpenGL, X11, fontconfig et freetype.
- **Xenia Windows** : exactement `LICENSE` et `xenia_canary.exe`; c'est le
  paquet le plus clairement autonome.
- **Xenia Linux** : AppImage officielle avec `usr/bin/xenia_canary` et les
  bibliothèques GTK3/SDL2/audio/X11 usuelles. Elle embarque les notices de ses
  bibliothèques Debian, mais aucun fichier de licence Xenia distinct n'a été
  trouvé : joindre explicitement [S10] lors d'une redistribution.

## Sources épinglées et licences

| Projet | Révision source liée | Accès source stable | SHA-256 de l'instantané observé | Licence / obligations de redistribution |
|---|---|---|---|---|
| RetroArch 1.20.0 | `ab3b175848fa6cd8b2340809631e30bc0fe1d136` [S6] | `https://github.com/libretro/RetroArch/releases/download/v1.20.0/retroarch-sourceonly-1.20.0.tar.xz` ; arbre : `https://github.com/libretro/RetroArch/tree/ab3b175848fa6cd8b2340809631e30bc0fe1d136` | `1dec616cf80d66fa1888b40a25a202b5c842d57f0348caac9880444e7a8eda2c` (11616548 octets, local ; l'API ne fournit pas de digest) | GPL-3.0 détectée par l'amont [S7] : conserver notices/licence ; pour le binaire, fournir le code source correspondant suivant GPLv3 §6, sans restrictions supplémentaires ; signaler les modifications. Auditer aussi les DLL/assets inclus. |
| Genesis Plus GX du pack | `3c1698778080541927f3d7011a00d2c9efe545af` [S3] | `https://codeload.github.com/libretro/Genesis-Plus-GX/tar.gz/3c1698778080541927f3d7011a00d2c9efe545af` ; arbre : `https://github.com/libretro/Genesis-Plus-GX/tree/3c1698778080541927f3d7011a00d2c9efe545af` | `533c5c71ba02cf7a6114015d1b9f163779f038f7ef66ed5c2ded363604f77b73` (14357663 octets, local) | Licence personnalisée non commerciale [S4] : aucune vente ni utilisation dans un produit/une activité commerciale ; reproduire copyright, conditions et exclusion de garantie ; si redistribution modifiée, fournir le code source complet des composants du binaire, sous réserve de l'exception système. Les sous-composants ont aussi leurs propres notices. |
| PCSX2 2.8.0 | `c10a5c9ad951c517dc48b722b5b84eb9bf7fdb42` [S5][S11] | `https://codeload.github.com/PCSX2/pcsx2/tar.gz/c10a5c9ad951c517dc48b722b5b84eb9bf7fdb42` ; arbre : `https://github.com/PCSX2/pcsx2/tree/c10a5c9ad951c517dc48b722b5b84eb9bf7fdb42` | `9f02b2ff5a3eef5c71c6db7f56d830e7d2b03423702a462963e45d3c4d42ccd7` (23267235 octets, local) | GPL-3.0 détectée par l'amont [S12] : mêmes obligations GPLv3 §6 ; les archives binaires incluent la GPL et les licences tierces. Pour une publication durable, héberger soi-même le code correspondant et les scripts/dépendances de construction plutôt que dépendre uniquement de GitHub. |
| Xenia Canary | `f03a9fcfcb2f30b3b958d0e5a8e6757cc2cf72fb` [S8][S9] | `https://codeload.github.com/xenia-canary/xenia-canary/tar.gz/f03a9fcfcb2f30b3b958d0e5a8e6757cc2cf72fb` ; arbre : `https://github.com/xenia-canary/xenia-canary/tree/f03a9fcfcb2f30b3b958d0e5a8e6757cc2cf72fb` | `4d1563ef08d7927c33ddb40d732fbaba10d1ef0a9f9ac6571dfc8fa50a39c908` (10058434 octets, local) | Licence de type BSD-3-Clause, sans identifiant SPDX déclaré par GitHub [S10] : reproduire copyright, conditions et exclusion de garantie avec les binaires ; ne pas utiliser le nom du projet/contributeurs pour promotion. Les notices des dépendances restent applicables. |

Les tarballs `codeload.github.com` sont pratiques pour l'accès, mais leur
encodage d'archive n'est pas promis immuable par ce document. Le verrou légal
robuste consiste à les copier dans un stockage immuable avec leur SHA-256.
L'instantané Xenia généré par codeload contient `.gitmodules`, mais pas le
contenu complet de tous les sous-modules ; il ne faut donc pas le présenter
comme une archive de reconstruction autonome.

## Gaps bloquants avant publication Monolith

1. Obtenir une attestation amont ou figer dans un stockage contrôlé les quatre
   archives buildbot RetroArch ; une URL versionnée n'est pas une preuve
   d'immutabilité.
2. Décider si Monolith et son canal de distribution respectent la clause
   **non commerciale** de Genesis Plus GX. À défaut, ne pas distribuer ce cœur.
3. Ne pas redistribuer les packs complets de cœurs : extraire/reconditionner le
   seul cœur autorisé nécessiterait un nouvel artefact, un nouveau SHA-256, sa
   licence et un test de lancement par OS.
4. Constituer, pour RetroArch et PCSX2, un dépôt durable du code source
   correspondant et des notices de toutes les dépendances effectivement
   livrées ; les simples liens amont ne transfèrent pas l'obligation GPL du
   distributeur.
5. Ajouter explicitement la licence Xenia à côté de l'AppImage Linux et
   conserver les notices des bibliothèques embarquées.
6. Ajouter au moteur d'installation le support vérifié de 7z/AppImage, ou
   publier des ZIP dérivés audités ; ne jamais renommer un 7z en ZIP.
7. Effectuer les essais de lancement sur vrais Windows x86-64 et Linux x86-64.
   Ici, seules intégrité, extraction, chemins et dépendances statiques ont été
   vérifiés. Aucun BIOS/firmware propriétaire ne doit être distribué ; le BIOS
   PS2 reste fourni localement par l'utilisateur.

## Sources primaires

- [S1] Index officiel RetroArch 1.20.0 Windows x86-64 :
  https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/
- [S2] Index officiel RetroArch 1.20.0 Linux x86-64 :
  https://buildbot.libretro.com/stable/1.20.0/linux/x86_64/
- [S3] Commit Genesis Plus GX résolu depuis la révision incorporée :
  https://api.github.com/repos/libretro/Genesis-Plus-GX/commits/3c16987
- [S4] Licence Genesis Plus GX à cette révision :
  https://raw.githubusercontent.com/libretro/Genesis-Plus-GX/3c1698778080541927f3d7011a00d2c9efe545af/LICENSE.txt
- [S5] API de la release immuable PCSX2 2.8.0 (assets, tailles, digests) :
  https://api.github.com/repos/PCSX2/pcsx2/releases/tags/v2.8.0
- [S6] Référence du tag RetroArch 1.20.0 :
  https://api.github.com/repos/libretro/RetroArch/git/ref/tags/v1.20.0
- [S7] Licence RetroArch épinglée :
  https://raw.githubusercontent.com/libretro/RetroArch/ab3b175848fa6cd8b2340809631e30bc0fe1d136/COPYING
- [S8] API de la release Xenia Canary épinglée (assets, digests,
  `immutable: false`) :
  https://api.github.com/repos/xenia-canary/xenia-canary/releases/tags/f03a9fc
- [S9] Commit complet Xenia Canary :
  https://api.github.com/repos/xenia-canary/xenia-canary/commits/f03a9fc
- [S10] Licence Xenia Canary épinglée :
  https://raw.githubusercontent.com/xenia-canary/xenia-canary/f03a9fcfcb2f30b3b958d0e5a8e6757cc2cf72fb/LICENSE
- [S11] Commit complet PCSX2 2.8.0 :
  https://api.github.com/repos/PCSX2/pcsx2/commits/v2.8.0
- [S12] Licence PCSX2 épinglée :
  https://raw.githubusercontent.com/PCSX2/pcsx2/c10a5c9ad951c517dc48b722b5b84eb9bf7fdb42/COPYING.GPLv3
