# Monolith

Client de bibliothèque de jeux en **Rust + egui**, conçu selon une architecture **Local-First**.

## État du prototype

- navigation : accueil → consoles → catalogue → fiche ;
- recherche locale dans un catalogue ;
- métadonnées globales issues de la récolte ;
- surcharge de description et de jaquette par utilisateur ;
- conservation des données globales lors d'une surcharge ;
- stockage SQLite et cache JSON publié atomiquement ;
- priorité linguistique prévue : `fr`, puis `en`, puis valeur par défaut.

Les connexions au NAS, aux fournisseurs de métadonnées et au backend distant ne sont pas encore configurées. Le présent socle utilise des chemins locaux configurables.

## Prérequis

```bash
rustup toolchain install stable --component rustfmt,clippy
```

Sous Linux, les bibliothèques graphiques requises par `eframe` doivent être présentes.

## Compilation et validation

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build
```

## Exécution

```bash
MONOLITH_DATA_DIR="$HOME/.local/share/monolith" cargo run
```

Sans variable, les données sont écrites dans `./data` :

- `monolith.db` : source SQLite locale ;
- `monolith_cache_data.json` : vue résolue du catalogue pour le profil actif.

## Surcharges utilisateur

La table `user_overrides` est indexée par `(user_id, game_id)`. Les champs non renseignés retombent automatiquement sur les données récoltées. Une sauvegarde depuis la fiche de jeu actualise immédiatement le cache JSON.

## Organisation

- `src/models.rs` : contrats de données sérialisables ;
- `src/db.rs` : migrations et requêtes SQLite ;
- `src/navigation.rs` : machine de navigation de *The Heart* ;
- `src/ui.rs` : interface egui ;
- `src/cache.rs` : lecture et écriture atomique du cache ;
- `src/sync.rs` : première tranche de *The Brain* ;
- `tests/core.rs` : comportements critiques testés.

## Limites actuelles

- aucun connecteur réseau de récolte ou de synchronisation distante ;
- aucun téléchargement de jaquette ; le chemin ou l'URL est seulement enregistré ;
- authentification encore remplacée par un identifiant de profil local (`1`) ;
- chemins NAS et adaptateurs d'émulateurs à définir.
