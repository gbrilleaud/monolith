# Contenus légaux et publication publique

## Principe

Monolith est un logiciel de catalogue, d'inventaire et de lancement. Il ne fournit aucun contenu de jeu, BIOS, firmware, clé, sauvegarde ou installateur commercial.

Un dépôt GitHub public de Monolith contient le code source, les tests, les schémas, les exemples de configuration et la documentation. Il ne contient jamais de contenu utilisateur ni de contenu propriétaire.

## Interdictions de publication

Ne pas ajouter au dépôt, aux releases, aux artefacts CI, aux issues, aux captures d'écran ou aux données de démonstration :

- ROMs, images disque et CHD commerciaux ;
- BIOS, firmware, IPL, NAND, clés de console ou clés de déchiffrement ;
- installateurs GOG, exécutables Windows, fichiers de données ou DLC commerciaux ;
- installations locales de jeux, préfixes Wine/Proton, sauvegardes ou profils personnels ;
- bases SQLite d'exécution, caches, journaux, jetons, mots de passe, secrets et configurations locales ;
- chemins personnels ou chemins de partage NAS pouvant révéler une infrastructure privée.

Les fixtures de tests sont permises uniquement si elles sont créées artificiellement, minimales, non exécutables et documentées comme telles.

## Installateurs GOG hors ligne

Monolith peut indexer ou vérifier des installateurs hors ligne fournis **localement** par l'utilisateur. Le projet ne télécharge pas, n'héberge pas et ne redistribue aucun installateur GOG.

Le flux privé attendu est :

1. l'utilisateur télécharge depuis son propre compte GOG tous les fichiers de setup requis ;
2. il les dépose dans son stockage privé ;
3. Monolith vérifie que le paquet est complet, enregistre des empreintes et assiste l'installation locale ;
4. le jeu s'installe sur le disque local du PC, jamais dans le dépôt source ni directement comme installation réseau.

L'utilisateur reste responsable de respecter les licences applicables. Une absence de DRM ne transforme pas un jeu commercial en contenu redistribuable.

## BIOS et firmware

Monolith peut déclarer qu'une plateforme nécessite un BIOS et en valider le nom, la taille ou l'empreinte. Le projet public ne télécharge, ne fournit, ne récupère ni ne redistribue ces fichiers.

L'administrateur d'une instance privée peut conserver ses propres dumps légalement créés sur son NAS privé et les déployer uniquement vers les appareils qu'il a autorisés dans son réseau privé. Les BIOS restent exclus du dépôt, des releases, des artefacts CI et de toute interface ou URL publique. Une instance installée par un tiers garde le répertoire BIOS vide jusqu'à ce que son propre administrateur le remplisse avec ses propres dumps.

Les BIOS typiquement concernés incluent notamment PlayStation, Saturn, Mega-CD, Dreamcast et certains fichiers système Nintendo. Les cartouches Mega Drive / Genesis ordinaires lancées avec Genesis Plus GX ne requièrent normalement pas de BIOS externe.

## Checklist avant publication GitHub

1. Vérifier l'arbre de travail et les fichiers suivis : `git status --short` et `git ls-files`.
2. Vérifier l'historique complet, pas seulement le dernier commit : `git log --all --stat` et `git rev-list --objects --all`.
3. Rechercher ROMs, BIOS, installateurs, archives, secrets, bases de données et chemins personnels dans toutes les branches et tous les tags.
4. Vérifier que les exemples de configuration sont anonymisés et que les configurations réelles restent ignorées.
5. Si un contenu interdit est déjà présent dans l'historique, le purger avant le premier push public ; le supprimer dans un commit ultérieur est insuffisant.
6. Exécuter les tests et une revue manuelle du diff à publier.

## Références

- [GOG — téléchargement des installateurs hors ligne](https://support.gog.com/hc/en-us/articles/213148105-How-do-I-download-my-purchased-items)
- [Politique BIOS de Monolith](BIOS_SETUP.md)
- [Disposition de stockage privée](NAS_LAYOUT.md)
