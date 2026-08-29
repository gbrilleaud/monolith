# Backend et authentification

## Démarrage rapide

```bash
cp config/backend.example.toml config/backend.toml
cargo run --bin monolith-admin -- config check
printf '%s' 'change-me-now' | cargo run --bin monolith-admin -- user add admin --role admin --password-stdin
cargo run --bin monolith-backend
```

Le backend écoute par défaut sur `127.0.0.1:8787`. Placez un reverse proxy TLS devant lui pour une exposition réseau.

## Modes d'authentification

- `local` : comptes et sessions autonomes dans SQLite ;
- `sso` : jetons JWT OIDC uniquement ;
- `hybrid` : les deux méthodes sont acceptées en parallèle.

Le mode hybride ne fait pas dépendre les comptes locaux de la disponibilité du fournisseur SSO. Une session locale valide continue donc de fonctionner pendant une panne OIDC.

### Passage en mode hybride

```bash
cargo run --bin monolith-admin -- auth set hybrid \
  --issuer 'https://sso.example.net/realms/monolith' \
  --audience monolith \
  --jwks-url 'https://sso.example.net/realms/monolith/protocol/openid-connect/certs' \
  --auto-provision
```

La commande met à jour `config/backend.toml` avec une écriture atomique. Le backend recharge cette configuration au prochain démarrage ; un redémarrage contrôlé est donc requis après modification. On peut aussi éditer ce fichier manuellement puis exécuter :

```bash
cargo run --bin monolith-admin -- config check
```

Sans `auto_provision`, un sujet SSO inconnu est refusé. Avec cette option, il est créé en lecture seule par défaut, sauf si une revendication `role` reconnue (`read_only`, `standard`, `admin`) est présente dans le JWT.

## CLI d'administration

```text
monolith-admin config init [--force]
monolith-admin config show
monolith-admin config check
monolith-admin auth set <local|sso|hybrid> [options OIDC]
monolith-admin user add <nom> --role <read-only|standard|admin> --password-stdin
monolith-admin user list
monolith-admin user enable <id>
monolith-admin user disable <id>
```

Le mot de passe n'est volontairement pas accepté en argument : cela évite son exposition dans l'historique shell et la liste des processus. La désactivation d'un compte révoque toutes ses sessions locales.

Une configuration différente peut être choisie avec `--config /chemin/backend.toml` ou `MONOLITH_BACKEND_CONFIG`.

## API v1

- `GET /api/v1/health` : santé et mode d'authentification ;
- `POST /api/v1/auth/login` : session locale ;
- `GET /api/v1/auth/me` : identité, rôle et expiration du Bearer ;
- `GET /api/v1/catalog` : catalogue résolu du profil authentifié ;
- `PUT /api/v1/users/{user_id}/overrides/{game_id}` : surcharge utilisateur.

Toutes les routes, sauf la santé et le login local, attendent l’en-tête `Authorization: Bearer [JETON]`.

## Sécurité actuelle

- mots de passe : Argon2id avec sel aléatoire ;
- sessions locales : 256 bits aléatoires, seul leur SHA-256 est stocké ;
- SSO : vérification de signature via JWKS, ainsi que `issuer`, `audience` et expiration ;
- permissions : `read_only`, `standard`, `admin` ;
- les comptes en lecture seule ne peuvent pas publier de surcharge ;
- un compte standard ne peut modifier que son profil ; un administrateur peut agir sur les autres profils.

À prévoir avant production : reverse proxy HTTPS, limitation de débit du login, rotation/caching contrôlé du JWKS, journal d'audit et sauvegardes SQLite.
