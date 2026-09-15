# Inventaire local des ROMs — plan d’implémentation

> **Pour Hermes :** exécuter ce plan tâche par tâche en TDD, avec revue du diff et validation complète avant chaque commit.

**Objectif :** permettre à Monolith de scanner un ou plusieurs répertoires de ROMs, d’indexer les fichiers dans SQLite sans les déplacer, de publier leur disponibilité dans le cache local et d’exposer l’opération via `monolith-admin` puis egui.

**Architecture :** un scanner Rust pur produit des observations indépendantes du stockage. La couche SQLite fait un upsert transactionnel et marque comme absents les fichiers disparus. La configuration contient des racines locales ; un futur montage CIFS du Synology sera simplement une racine supplémentaire. Aucun téléchargement, lancement d’émulateur ou accès à une API externe dans cette tranche.

**Pile :** Rust 2021, `std::fs`, rusqlite, serde/TOML, clap, egui, tempfile, SHA-256 déjà disponible.

---

## Contexte vérifié

- Dépôt : `/home/hermes/monolith`, branche `main`, propre au commit `53a799d`.
- Briques existantes : SQLite, cache JSON atomique, backend HTTP, client egui, authentification, CLI d’administration.
- Le catalogue actuel ne contient pas d’inventaire physique de ROMs.
- `src/integrity.rs` et `ROM_RESOURCE_MAP.md`, cités dans d’anciens résumés, n’existent pas dans le dépôt actuel.
- Les chemins NAS définitifs ne sont pas connus ; le scanner doit donc accepter des répertoires arbitraires.
- Priorité produit : rester local-first et utilisable sans NAS/backend.

## Critères d’acceptation

1. Scanner récursivement des racines configurées sans suivre les liens symboliques.
2. Reconnaître une liste explicite d’extensions, insensible à la casse.
3. Associer le système depuis la racine configurée, pas par heuristique fragile sur le nom.
4. Conserver chemin canonique, taille, date de modification, extension et empreinte optionnelle.
5. Réexécuter un scan sans créer de doublons.
6. Marquer comme absent un fichier supprimé, sans supprimer son historique.
7. Ne jamais déplacer, renommer ou supprimer une ROM.
8. Continuer après un fichier illisible et restituer les erreurs dans un rapport.
9. Fournir `monolith-admin library scan` et `library status`.
10. Afficher dans egui les états disponible/absent/non indexé sans bloquer le thread graphique.
11. Tous les tests, Clippy strict et le build des binaires doivent réussir.

---

## Tâche 0 — Corriger la documentation immédiatement visible

**Fichiers :**
- Modifier : `/home/hermes/monolith/BACKEND.md`

**Étapes :**
1. Corriger la ligne Bearer incomplète en : `Authorization: Bearer <jeton>`.
2. Exécuter `git diff --check`.
3. Commit : `docs: fix backend bearer header example`.

## Tâche 1 — Définir les contrats d’inventaire

**Fichiers :**
- Modifier : `/home/hermes/monolith/src/models.rs`
- Créer : `/home/hermes/monolith/tests/inventory.rs`

**Types minimaux :**
- `RomLocation { id, game_id, system_id, path, extension, size_bytes, modified_at, sha256, availability, last_seen_at }`
- `RomAvailability::{Available, Missing}` avec sérialisation `snake_case`.
- `ScanRoot { system_id, path, extensions }`.
- `ScanObservation` limité aux données produites par le système de fichiers.
- `ScanIssue { path, message }` et `ScanReport { visited, accepted, ignored, missing, issues }`.

**Cycle TDD :**
1. Écrire des tests de sérialisation et d’égalité des statuts.
2. Vérifier le passage rouge.
3. Ajouter les types minimaux.
4. Vérifier le passage vert.
5. Commit : `feat: define ROM inventory contracts`.

## Tâche 2 — Implémenter le scanner de fichiers pur

**Fichiers :**
- Créer : `/home/hermes/monolith/src/inventory.rs`
- Modifier : `/home/hermes/monolith/src/lib.rs`
- Modifier : `/home/hermes/monolith/tests/inventory.rs`

**Comportement :**
- parcours itératif avec `std::fs::read_dir` ;
- aucune traversée de lien symbolique ;
- extensions normalisées en minuscules ;
- ordre final déterministe par chemin ;
- un fichier illisible devient un `ScanIssue` ;
- pas de hash par défaut afin de garder le premier scan rapide.

**Tests avec `tempfile` :**
1. reconnaît `.chd`, `.cso`, `.iso`, `.rvz`, `.gdi`, `.cue`, `.bin`, `.zip`, `.7z` selon la liste de la racine ;
2. accepte les extensions en majuscules ;
3. ignore les extensions non configurées ;
4. ne suit pas un lien symbolique ;
5. retourne un ordre déterministe ;
6. une racine absente produit un rapport d’erreur, pas un panic.

**Validation :**
- `cargo test --test inventory scanner`
- Commit : `feat: scan configured ROM roots safely`.

## Tâche 3 — Ajouter la migration SQLite et l’upsert transactionnel

**Fichiers :**
- Modifier : `/home/hermes/monolith/src/db.rs`
- Modifier : `/home/hermes/monolith/tests/inventory.rs`

**Schéma proposé :**
```sql
CREATE TABLE IF NOT EXISTS rom_locations (
    id INTEGER PRIMARY KEY,
    game_id INTEGER,
    system_id INTEGER NOT NULL,
    path TEXT NOT NULL UNIQUE,
    extension TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    modified_at INTEGER,
    sha256 TEXT,
    availability TEXT NOT NULL CHECK (availability IN ('available', 'missing')),
    last_seen_at TEXT NOT NULL,
    FOREIGN KEY(game_id) REFERENCES games(id),
    FOREIGN KEY(system_id) REFERENCES systems(id)
);
CREATE INDEX IF NOT EXISTS idx_rom_locations_system_availability
ON rom_locations(system_id, availability);
```

**Règles :**
- upsert par chemin canonique ;
- transaction unique par scan ;
- fichiers vus => `available` ;
- anciens fichiers de la même racine non vus => `missing` ;
- ne pas supprimer les lignes manquantes ;
- `game_id` reste nullable tant que la correspondance titre/jeu n’est pas fiable.

**Tests :**
1. premier scan insère ;
2. second scan est idempotent ;
3. modification de taille/date met à jour ;
4. suppression physique marque `missing` ;
5. une autre racine n’est pas affectée.

**Validation :** `cargo test --test inventory database`

**Commit :** `feat: persist ROM inventory transactionally`.

## Tâche 4 — Configurer les racines sans coder le chemin du NAS en dur

**Fichiers :**
- Modifier : `/home/hermes/monolith/src/backend_config.rs`
- Modifier : `/home/hermes/monolith/config/backend.example.toml`
- Créer ou modifier : `/home/hermes/monolith/tests/inventory_config.rs`

**Format TOML proposé :**
```toml
[[library.roots]]
system_id = 1
path = "/mnt/roms/megadrive"
extensions = ["zip", "7z", "bin", "md"]

[[library.roots]]
system_id = 2
path = "/mnt/roms/dreamcast"
extensions = ["chd", "gdi", "cue"]
```

**Tests :**
- parsing de plusieurs racines ;
- rejet d’une liste d’extensions vide ;
- rejet d’un chemin relatif ;
- message précis pour un `system_id` invalide.

**Commit :** `feat: configure ROM library roots`.

## Tâche 5 — Exposer le scan dans `monolith-admin`

**Fichiers :**
- Modifier : `/home/hermes/monolith/src/bin/monolith-admin.rs`
- Modifier : `/home/hermes/monolith/BACKEND.md`
- Créer : `/home/hermes/monolith/tests/admin_library.rs` si l’architecture permet un test de commande ; sinon extraire une fonction testable dans `src/inventory.rs`.

**Commandes :**
```text
monolith-admin library scan [--hash-new]
monolith-admin library status [--system-id <id>]
```

**Sortie de `scan` :** nombre visité, accepté, ignoré, marqué absent, erreurs. Retour non nul uniquement pour une erreur globale (configuration/DB), pas pour un fichier isolé illisible.

**Sécurité :** `--hash-new` calcule SHA-256 uniquement pour les nouveaux fichiers ou ceux dont taille/date ont changé ; aucun contenu de ROM dans les logs.

**Validation :**
- `cargo run --bin monolith-admin -- --help`
- test sur un répertoire temporaire ;
- vérification du deuxième scan idempotent.

**Commit :** `feat: add ROM library administration commands`.

## Tâche 6 — Publier la disponibilité dans le cache local

**Fichiers :**
- Modifier : `/home/hermes/monolith/src/models.rs`
- Modifier : `/home/hermes/monolith/src/db.rs`
- Modifier : `/home/hermes/monolith/src/sync.rs`
- Modifier : `/home/hermes/monolith/tests/core.rs`

**Approche :** ajouter à la vue résolue du jeu un résumé non ambigu :
```rust
pub struct LaunchAvailability {
    pub available: bool,
    pub location_count: usize,
    pub preferred_path: Option<String>,
}
```

Le chemin est local au client/serveur qui construit le cache. Aucun chemin Windows ne doit être prétendu valide sur Linux ; la future résolution de launcher restera séparée.

**Tests :**
- jeu sans emplacement ;
- un emplacement disponible ;
- plusieurs emplacements avec choix déterministe ;
- emplacement manquant ignoré.

**Commit :** `feat: expose ROM availability in local cache`.

## Tâche 7 — Ajouter le scan asynchrone dans egui

**Fichiers :**
- Modifier : `/home/hermes/monolith/src/ui.rs`
- Créer éventuellement : `/home/hermes/monolith/src/client_inventory.rs`
- Modifier : `/home/hermes/monolith/src/lib.rs`
- Créer : `/home/hermes/monolith/tests/client_inventory.rs`

**Comportement :**
- bouton « Actualiser la bibliothèque » visible pour `standard` et `admin` ;
- thread de fond + canal `mpsc`, sur le même modèle que `ClientAuth` ;
- état `Idle/Scanning/Completed/Error` ;
- résultat synthétique sans liste massive d’erreurs dans l’écran principal ;
- les comptes `read_only` voient le statut mais ne déclenchent pas un scan serveur ;
- aucune opération disque longue dans `eframe::App::update`.

**Tests :** transitions d’état pures et rapport rendu disponible après polling.

**Commit :** `feat: refresh ROM inventory outside egui thread`.

## Tâche 8 — Documentation, contrôle qualité et commit final

**Fichiers :**
- Modifier : `/home/hermes/monolith/README.md`
- Modifier : `/home/hermes/monolith/BACKEND.md`
- Modifier : `/home/hermes/monolith/.gitignore` seulement si un nouvel artefact local apparaît.

**Documentation :**
- formats de racines ;
- exemple Synology monté sous `/mnt/roms` sans imposer ce chemin ;
- garantie de lecture seule sur les ROMs ;
- comportement des fichiers absents ;
- différence entre scan rapide et `--hash-new`.

**Validation finale :**
```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --bins
git diff --check
git status --short
```

Effectuer un scénario réel sur un répertoire temporaire contenant trois faux fichiers vides, supprimer l’un d’eux, rescanner et vérifier qu’il est marqué `missing`. Ne jamais tester sur le NAS réel avant une relecture de la configuration.

**Commit :** `feat: add local-first ROM inventory pipeline` si les tâches ont été regroupées, sinon conserver les commits atomiques précédents.

---

## Extension produit — système PC et déploiement de jeux natifs

Le système logique **PC** complète les consoles émulées, mais ne les assimile pas à des ROMs. Il référence des jeux natifs dont la distribution et le lancement sont autorisés par leur licence, avec une priorité initiale **Windows x86_64**.

### Principes

- Une tuile « PC » apparaît au même niveau que les systèmes émulés.
- Les paquets PC ne sont jamais mélangés avec les ROMs ni les émulateurs sur le NAS.
- Le client détecte OS et architecture, télécharge le paquet compatible dans son répertoire local Monolith, contrôle son SHA-256, puis applique le mode de déploiement déclaré.
- Aucun exécutable ne doit être lancé avant validation de l’intégrité du paquet et de la compatibilité de la plate-forme.
- Le backoffice référence exclusivement des sources officielles et conserve la licence, l’URL source, la version et l’empreinte de chaque paquet.

### Arborescence NAS cible

```text
MONOLITH_PROJECT/
├── roms/
├── emulators/
└── pc-games/
    └── <slug>/<version>/<platform-architecture>/
        ├── package.json
        └── payload/
```

`package.json` décrit au minimum : slug, titre, version, plate-forme, architecture, licence, URL officielle, SHA-256, taille, type de déploiement, commande de lancement et prérequis déclarés.

### Types de déploiement

| Type | Comportement du client |
|---|---|
| `portable_archive` | Télécharge, vérifie, extrait dans le répertoire local Monolith et lance l’exécutable déclaré. |
| `installer` | Télécharge et vérifie l’installateur ; l’exécution requiert une confirmation explicite de l’utilisateur. |
| `launcher_managed` | Télécharge et vérifie le lanceur ; les assets et mises à jour restent gérés par celui-ci. |

### Candidats initiaux vérifiés

- **Warsow 2.1.2** : la source officielle propose une archive `.tar.gz` unifiée Windows/Linux, ainsi que des paquets Windows et macOS. Candidat pilote pour qualifier `portable_archive` sur Windows ; le contenu de l’archive et la commande de lancement devront être testés avant publication au catalogue.
- **Beyond All Reason 1.2988.0** : la source officielle fournit un installateur Windows et une AppImage Linux. Le jeu impose un lanceur qui télécharge ensuite les assets et reçoit des mises à jour fréquentes ; il relève donc de `launcher_managed`, non de `portable_archive`. macOS est officiellement non pris en charge.

### Incrément PC à planifier après l’inventaire ROM

1. Ajouter les contrats `PcGame`, `PcGameRelease` et `DeploymentKind`, testés en sérialisation et validation.
2. Ajouter le système logique `PC` et afficher ses jeux dans les vues système existantes, sans logique de lancement.
3. Ajouter un service de téléchargement atomique avec limite de taille, SHA-256 obligatoire et nettoyage du fichier temporaire en erreur.
4. Ajouter l’extracteur sécurisé pour `portable_archive` : interdire les chemins absolus et les traversées `..`; n’autoriser que les archives explicitement prises en charge.
5. Ajouter la résolution de commande de lancement et le dry-run ; le lancement réel reste une étape ultérieure validée explicitement.
6. Enregistrer Warsow dans le backoffice uniquement après test réel de son archive Windows. Enregistrer BAR séparément en `launcher_managed`, avec son installation initiale explicitement signalée.

## Ordre recommandé après cet incrément

1. **Association ROM ↔ jeu** : rapprochement conservateur par noms normalisés avec validation manuelle des ambiguïtés.
2. **Adaptateurs d’émulateurs** : génération de commande en mode dry-run, puis `Command::spawn` avec configuration locale.
3. **Système PC** : exécuter l’incrément PC ci-dessus, avec Warsow comme pilote portable et BAR comme lanceur géré.
4. **Récolte de métadonnées** : interface fournisseur, cache et priorité `fr > en > default`; branchement IGDB/SteamGridDB seulement après fourniture des accès.
5. **Contrôleurs** : profils centralisés et génération de configurations par émulateur.
6. **Durcissement backend** : rate limit, audit, sauvegarde SQLite, reverse proxy TLS.
7. **SSO complet** : Authorization Code + PKCE lorsque le fournisseur OIDC réel est choisi.
8. **XTTSv2** : notifications vocales après stabilisation des événements métier.

## Risques et décisions ouvertes

- Le mapping `system_id` doit être vérifié contre les données réelles avant de scanner le NAS.
- Les couples `.cue/.bin` et `.gdi` nécessiteront ensuite une logique de regroupement ; la première tranche indexe les fichiers sans prétendre qu’un fichier équivaut toujours à un jeu.
- Les archives ne doivent pas être ouvertes pendant le scan initial.
- Le hash complet peut être coûteux sur un DS418 ; il reste optionnel et incrémental.
- Le scanner ne doit recevoir aucune capacité d’écriture sur les répertoires ROMs.
