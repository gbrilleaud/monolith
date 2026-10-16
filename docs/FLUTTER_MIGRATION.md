# Migration incrémentale Rust → Flutter (FRB)

## Architecture et périmètre

Une seule logique métier : `db`, `catalog_query`, `cover`, `local_rom`,
`emulator_launcher`, authentification, synchronisation, backend et configuration
restent dans le crate Rust `monolith`. `src/library_service.rs` compose les DTO
de bibliothèque et résout les ROMs enregistrées dans SQLite ; l'ancien écran egui
utilise les mêmes conversions de catalogue. `src/flutter_api.rs` expose ces
services à Flutter via `flutter_rust_bridge` 2.13.0. Les bindings dans
`src/frb_generated.rs` et `flutter_app/lib/src/rust/` sont générés, non édités
manuellement. Dart ne stocke aucune règle de catalogue, aucun SQL et aucune
commande d'émulateur.

La première tranche Flutter offre : bibliothèque hors ligne, recherche wildcard
déléguée à Rust, catalogue défilant en grille ou liste, jaquettes HTTP(S) ou
`file://`, tuiles homogènes adaptatives, lancement depuis une ROM inscrite et
disponible dans SQLite. Le lancement est exécuté par Rust, sans shell. Sans
backend ni NAS, les données déjà présentes localement restent consultables.
La bibliothèque utilise le profil d'une session locale non expirée dans
`session.json` ; à défaut, elle revient au profil hors ligne `user_id=1`, comme
egui. Une session expirée est effacée. Flutter permet la connexion locale, le
choix validé du backend, le rafraîchissement du cache distant et la déconnexion.
Le catalogue téléchargé est importé dans SQLite par Rust avant l'ouverture
de la bibliothèque ; le jeton reste dans `session.json`, jamais dans Dart.
L'écran conserve un accès hors ligne volontaire.
Le client egui et la chaîne officielle de release restent en place jusqu'à
parité fonctionnelle et validation native ; **ne pas remplacer la release egui
par l'artefact Flutter pour l'instant**.

## Exécution et génération

Installer Flutter 3.47.7, Rust stable, et les dépendances natives Flutter de
l'OS. Depuis la racine du dépôt :

```bash
cargo test --locked
cd flutter_app
flutter pub get
flutter analyze
flutter test
flutter run -d linux      # ou -d windows
flutter build linux --release    # ou windows
```

Sur Linux : `clang cmake ninja-build pkg-config libgtk-3-dev liblzma-dev`.
Cargokit compile le crate Rust du dépôt et joint `libmonolith` au bundle Flutter.
Conserver **tout le dossier** du bundle (pas seulement l'exécutable).
Les fichiers `rust_builder` et les bindings sont versionnés ; pas de second crate
Rust contenant la logique métier.

Après modification de `src/flutter_api.rs` ou des DTO FRB :

```bash
cargo install --locked flutter_rust_bridge_codegen --version 2.13.0
cd flutter_app
flutter_rust_bridge_codegen generate --config-file flutter_rust_bridge.yaml
flutter analyze && flutter test
```

La génération nécessite aussi `cargo-expand` (le codegen l'installe si absent).
Le répertoire de données est `data/monolith.db`, **relatif au répertoire de
travail** du processus, comme dans le client egui ; utiliser le même répertoire
de travail pour conserver le catalogue. Sinon lancer Flutter avec
`MONOLITH_DATA_DIR` pointant sur un dossier déjà peuplé, par exemple
`C:\MONOLITH\data` sous Windows. Ne pas copier ou recréer une base privée dans le
dépôt. Les profils d'émulateurs sont résolus par Rust sous ce même dossier.

## Configuration locale des manettes (état expérimental)

La gate `feature-parity.json` / `scripts/check_feature_parity.py` contrôle la
présence d'un service Rust, des deux surfaces egui/Flutter et de tests pour les
features déclarées ; elle ne prouve ni parité visuelle ni exhaustivité du registre.
La CI exécute ce contrôle avant les tests Rust, et la CI Flutter valide ses widgets.

Les deux clients proposent « Manettes », une édition contrôlée de
`<MONOLITH_DATA_DIR>/controllers.toml` et un bouton « RAZ valeurs par défaut »
par `system_id` et `device_id`. Exemple de profil manuel :

```toml
[[profile]]
system_id = 11
device_id = "xinput:0"
[profile.buttons]
a = "0"
b = "1"
```

Une seule manette active par console est prise en charge. Les boutons acceptés
sont `a,b,x,y,start,select,l,r,l2,r2,l3,r3,up,down,left,right`, avec index de
bouton physique `0..63`. Les IDs XInput sont des slots temporaires ; les IDs
Linux `jsN` ne sont pas des identifiants matériels persistants. `device_id`
identifie le profil dans Monolith, mais **ne lie pas encore ce profil à un
périphérique précis dans RetroArch** : la surcouche s'applique au joueur 1
sélectionné par RetroArch. La détection liste les périphériques, elle ne
capture pas encore chaque pression.
**Aucune correspondance Xbox 360 personnalisée n'est livrée avant réception du
fichier utilisateur** : la RAZ revient à l'autoconfiguration native de RetroArch.

Lors de l'enregistrement, Rust valide puis génère
`controllers/retroarch/system-<id>.cfg` ; le lanceur RetroArch y fait référence
via `--appendconfig` pour la console choisie, sans modifier `retroarch.cfg`.
La RAZ enlève uniquement cette surcouche et l'entrée Monolith correspondante.
**PCSX2 2.8 et Xenia ne sont pas synchronisés** : leurs formats de mapping ne
sont pas interchangeables avec RetroArch. Ajouter des adaptateurs distincts,
testés avec des fichiers réels et sauvegarde des fichiers utilisateurs, avant de
revendiquer une synchronisation générale des émulateurs. Les lancements réels
avec des manettes restent à valider sur Windows et Linux ; mobile non pris en
charge.

## Animations de lancement par système (Flutter uniquement)

Dans la bibliothèque Flutter (grille et liste), « Lancer » affiche une scène de
2,2 secondes avant de transmettre la ROM au service Rust inchangé. Mega Drive /
Genesis conserve son animation dédiée de cartouche en perspective. Les autres
systèmes ont une scène vectorielle adaptative : identité et couleur propres au
système, silhouette de console à cartouche ou à disque, portable, borne arcade
ou ordinateur, avec la jaquette (`file://` ou HTTP(S)) et un pictogramme de repli.
Les noms prévus couvrent les dossiers ROM du NAS (Lynx, FinalBurn Alpha, MAME,
NES, Super Nintendo, Nintendo 64, Game Boy / Color / Advance, GameCube,
Dreamcast, Game Gear, Master System, Saturn, PlayStation, PSP) ainsi que PS2
et PC. Un système inconnu reçoit une scène générique portant son nom. Le
réglage « Animations au lancement des jeux » coupe toutes les scènes et lance
immédiatement. Une vue détruite avant la fin annule le lancement différé.

C'est une **présentation Flutter**, non une nouvelle règle métier Rust ni une
parité visuelle egui. Les silhouettes sont stylisées et ne reproduisent pas
fidèlement le matériel ; rendu natif et timing de l'émulateur restent à vérifier
sur poste client.

## Limites avant remplacement d'egui

- Flutter expose la bibliothèque locale du profil de session valide (ou
  `user_id=1` hors ligne), le lancement, le paramétrage serveur, la connexion
  locale, les téléchargements de ROM et la configuration des émulateurs. Une
  mise à jour distincte du bundle Flutter est proposée sous Windows à partir
  d'une Release GitHub compatible, sur confirmation explicite. Le SSO, la gestion
  des comptes, les uploads, les associations, l'édition et la synchronisation
  continue restent à porter ; il n'y a pas encore de parité fonctionnelle. Les
  jaquettes privées requièrent des fichiers déjà synchronisés localement ou des
  URL accessibles.
- Les adaptateurs d'émulateur existants servent Windows et Linux. La présence
  d'un profil et d'une ROM locale est requise. Le lancement n'a été testé ici
  que par résolution de commande, pas avec un émulateur réellement installé.
- **Android** : le projet Flutter Android est généré, mais le crate Rust complet
  et le pont natif ne sont pas validés par une compilation Android ; aucun Android
  SDK n'est présent sur cette machine. Un adaptateur dédié doit convertir les
  URI `content://`/SAF vers les droits d'accès applicatifs et interagir avec
  des émulateurs installés via intents explicites. Les chemins de fichiers PC et
  la bibliothèque NAS ne sont pas directement accessibles.
- **iOS** : le projet Flutter iOS est généré, mais la compilation requiert macOS,
  Xcode et CocoaPods ; elle n'est pas validée. Le bac à sable et les permissions
  iOS imposent un stockage et des imports dédiés. Le lancement d'autres apps
  exige des URL schemes déclarés et supportés par ces apps : **aucun lancement
  d'émulateur iOS n'est actuellement promis**.
- Navigation tactile et disposition PC/tablette/smartphone sont implémentées au
  niveau Flutter. Navigation par clavier/focus de boutons possible ; une manette
  Xbox 360 réelle, le DPI et la qualité graphique sur appareils natifs doivent
  encore être vérifiés. Les jaquettes locales `file://` dépendent des permissions
  du processus Flutter.

La CI `.github/workflows/flutter.yml` analyse/teste Dart et construit des bundles
Windows/Linux séparés des archives officielles egui. Les builds mobiles et les
tests sur matériel réel restent une étape ultérieure.
