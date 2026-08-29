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
- `src/backend_client.rs` : connecteur HTTP et publication du cache hors ligne ;
- `src/backend.rs` : API HTTP versionnée ;
- `src/auth.rs` : authentification locale et vérification OIDC ;
- `src/bin/monolith-backend.rs` : service backend ;
- `src/bin/monolith-admin.rs` : CLI d’administration ;
- `tests/core.rs` : comportements critiques testés.

## Backend et authentification

Le backend propose des modes `local`, `sso` et `hybrid`. En mode hybride, l’authentification autonome et l’OIDC fonctionnent simultanément. Voir [`BACKEND.md`](BACKEND.md) et [`config/backend.example.toml`](config/backend.example.toml).

```bash
cp config/backend.example.toml config/backend.toml
cargo run --bin monolith-admin -- config check
cargo run --bin monolith-backend
```

## Limites actuelles

- aucun connecteur de récolte IGDB/SteamGridDB ;
- aucun téléchargement de jaquette ; le chemin ou l'URL est seulement enregistré ;
- l’écran de connexion du client egui n’est pas encore câblé au connecteur ;
- chemins NAS et adaptateurs d'émulateurs à définir.
