# Apparence et thèmes de Monolith

Le menu **Apparence** est accessible depuis l’accueil (y compris avant connexion) dans les clients egui et Flutter. Le choix s’applique immédiatement, est enregistré par poste et survit au redémarrage. L’option « Animation d’insertion Mega Drive » est enregistrée au même endroit mais n’a d’effet que dans Flutter.

## Source et fichiers

- Le backend publie `GET /api/v1/styles` sans identifiants. Il lit `<répertoire de configuration backend>/style-presets.json` si valide, sinon le catalogue embarqué `assets/theme-presets.json`. Modifier le fichier serveur et utiliser **Actualiser depuis le serveur** sur les clients ; une réponse invalide ne remplace pas les thèmes précédents.
- Le client utilise `<répertoire de données>/style-catalog.json` comme copie locale de la dernière liste reçue ; les sept presets embarqués permettent le choix hors ligne avant toute synchronisation.
- `<répertoire de données>/style.json` contient uniquement `selected`, `cartridge_animation` et, facultativement, un preset `custom`. Il est distinct de `client.toml` et de `session.json` : aucun mot de passe, jeton ou URL n’y est écrit. Sous Windows, Flutter utilise `C:\MONOLITH\data` par défaut ; egui utilise `data` (ou `MONOLITH_DATA_DIR` si défini), comme ses autres réglages locaux.
- Les modifications locales de `custom` ne sont pas écrasées par le backend. L’éditeur JSON du menu duplique initialement le preset actif. Pour le modifier à la main, arrêter le client puis éditer `style.json` ; garder `custom.id` égal à `custom` et `selected` à `custom` pour l’activer.

## Schéma des presets

Copier `assets/theme-presets.json` vers `<répertoire de configuration backend>/style-presets.json`, conserver les sept identifiants intégrés **dans leur ordre** (personnaliser leurs valeurs et ajouter éventuellement d’autres presets à la suite), puis adapter :

| Jeton | Interprétation |
|---|---|
| `id`, `name` | Identifiant stable ASCII minuscule/chiffres/`_`, libellé dans le menu. `custom` est réservé au client. |
| `colors.background`, `surface`, `surface_raised` | Fond général, panneaux, surfaces surélevées. |
| `colors.primary`, `secondary`, `text`, `text_muted`, `border`, `error` | Accents, actions et avertissements, texte, texte atténué, contours, erreurs. Couleurs `#RRGGBB`. |
| `font_family` | `sans`, `serif` ou `mono` ; familles génériques du poste, sans téléchargement de police. egui assimile `serif` à sa police proportionnelle par défaut. |
| `density` | Échelle de compacité de `0.75` à `1.4` (espacement de boutons/menus). |
| `tile_spacing` | Écart des tuiles en points de `4` à `24`. |
| `corner_radius` | Rayon des cadres en points de `0` à `24`. |
| `background_image` | Nom facultatif d’une image `.png`, `.jpg`, `.jpeg` ou `.webp` à installer sur **chaque poste** dans `<données>/themes/backgrounds/`. Pas de chemin absolu, de répertoire parent ou d’URL distante. Une image absente laisse le fond uni. |

Les deux clients interprètent ces jetons au mieux de leurs composants natifs : les apparences ne sont pas pixel-perfect entre egui et Flutter. Aucun fond sous licence ni police tierce n’est distribué avec les presets. La scène de cartouche Mega Drive conserve ses couleurs matérielles, indépendantes du thème.

Les sept choix embarqués sont **Clair, Sombre, Windows XP, Rose, Retour vers le futur, Zelda, Futuriste**. Les noms évoquent une ambiance graphique ; ils n’incluent aucun visuel ou marque protégée dans le paquet.

**Évolutions possibles** (hors schéma actuel) : taille des polices, intensité des ombres/élévation, contour de focus pour l’accessibilité, opacité du fond, contraste renforcé, vitesse des transitions et style des icônes. Ces paramètres demandent des équivalents testables dans les deux moteurs avant d’entrer dans le contrat partagé.
