# Organisation du stockage privé

Cette disposition est un exemple local. Elle ne doit pas être publiée avec les fichiers qu'elle contient.

```text
<stockage-privé>/
├── bios/
│   └── <plateforme>/
├── roms/
│   └── sega_genesis/
├── pc-games/
│   ├── installers/
│   │   └── gog/
│   │       └── <jeu>/<version>/
│   │           ├── setup_<jeu>.exe
│   │           ├── setup_<jeu>-1.bin
│   │           └── manifest.json
│   └── metadata/
│       ├── covers/
│       └── provenance/
└── emulators/
    └── manifests/
```

## BIOS et firmware privés

Le répertoire `bios/` est une bibliothèque de référence privée de l'administrateur de l'instance. Il peut être utilisé par le serveur pour provisionner un appareil client explicitement autorisé lors de l'installation. Il reste exclu de Git, des releases et de toute exposition publique.

Chaque appareil conserve sa copie validée dans son répertoire de données local. Le serveur transmet uniquement les fichiers nécessaires à la plateforme demandée, via un canal authentifié, puis le client vérifie taille et SHA-256. Une installation tierce de Monolith crée la structure sans y inclure de BIOS ; son administrateur doit la remplir avec ses propres dumps.

## ROMs

Les ROMs sont inventoriées en lecture seule. Monolith ne les déplace, renomme ni supprime pendant un scan. Pour les cartouches Mega Drive, les archives ZIP peuvent être conservées si le cœur cible accepte leur contenu ; les archives à plusieurs fichiers demandent un test de lancement réel.

## Jeux PC DRM-free

Les installateurs sont des **paquets source privés**, pas des jeux prêts à lancer depuis un partage réseau.

- Conserver ensemble le setup principal et toutes ses parties `.bin`.
- Conserver une version par dossier et un manifeste décrivant les fichiers et leurs SHA-256.
- Vérifier les empreintes avant l'installation sur un PC.
- Installer le jeu sur le SSD/NVMe local de chaque machine, par exemple `D:\Games\Monolith\<jeu>` sous Windows ou `~/Games/Monolith/<jeu>` sous Linux.
- Ne pas exécuter directement une installation ou un préfixe Proton/Wine depuis SMB/CIFS.

## Bazzite et jeux Windows

Bazzite peut utiliser Heroic, Lutris ou Steam pour gérer un jeu Windows local et son préfixe Proton/Wine. Monolith doit enregistrer un statut de compatibilité par jeu, version, cible et runner ; il ne doit pas annoncer une compatibilité Proton universelle sans test réel.

## Manifeste de paquet GOG

```json
{
  "schema_version": 1,
  "game_id": "example-game",
  "platform": "windows_x86_64",
  "source": "gog_offline",
  "version": "1.0.0",
  "install_mode": "installer",
  "installer": {
    "entrypoint": "setup_example_game.exe",
    "files": [
      {"name": "setup_example_game.exe", "sha256": "<sha256>"},
      {"name": "setup_example_game-1.bin", "sha256": "<sha256>"}
    ]
  },
  "launch": {"executable": "ExampleGame.exe"}
}
```

Ce manifeste décrit des fichiers déjà fournis localement ; il ne contient aucun lien de téléchargement, aucune clé ni aucun fichier commercial.
