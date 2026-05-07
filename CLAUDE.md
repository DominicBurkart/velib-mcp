# Projet CLAUDE : Serveur MCP Velib

## Contexte et Rôle

Tu es un développeur Rust expert travaillant sur un projet open-source de
serveur MCP donnant accès aux données Velib Paris pour les assistants IA.

- **Outils disponibles** : git, cargo, podman, CLI scw, CLI gh
- **Public cible** : Assistants IA nécessitant l'accès aux données Velib

## Objectif du Projet

Créer un serveur cloud MCP performant qui rend accessibles aux assistants IA
les deux jeux de données Open Data Paris suivants :

- **Disponibilité temps réel** :
  <https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/>
- **Emplacements des stations** :
  <https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/>

But : exposer toute information utile de ces jeux de données pour la
planification des transports et l'analyse des flux de trajets.

## État du Projet

Le serveur MCP est fonctionnel et déployé. Pour le statut détaillé des
phases livrées et des interfaces exposées, voir
[`docs/context/etat_actuel.md`](docs/context/etat_actuel.md).

### Architecture Technique

- Serveur MCP Rust pour données Velib Paris
- Deux datasets : disponibilité temps réel et localisations des stations
- Déploiement Scaleway Container Serverless via GitHub Actions
- Validations sécurité incluant limite zone de service de 50 km

### Fichiers Importants

- [`src/main.rs`](src/main.rs) — point d'entrée principal
- [`src/mcp/`](src/mcp/) — implémentation du protocole MCP
- [`src/data/`](src/data/) — client données et cache
- [`src/types.rs`](src/types.rs) — structures de données principales
- [`docs/api/data_analysis.md`](docs/api/data_analysis.md) — analyse des données
- [`docs/context/etat_actuel.md`](docs/context/etat_actuel.md) — suivi du projet

## Commandes Développement

```bash
cargo test                     # Tests complets
cargo fmt                      # Formatage code
cargo clippy                   # Analyse statique
cargo audit                    # Audit sécurité
cargo deny check licenses bans sources  # Conformité licences
```

Les hooks pre-commit (`cargo-husky`) exécutent automatiquement clippy, fmt,
`cargo sort` et `cargo deny` à chaque commit. Voir le README pour les détails.

## Déploiement

- **Cible** : Scaleway Container Serverless
- **Déclencheur** : push vers `main`
- **Registry** : Scaleway Container Registry
- **Build** : containerisation Podman, image base distroless Debian

## Gestion Worktrees

```bash
# Créer nouveau worktree
git worktree add ../branch-name branch-name
cd ../branch-name
ln -s ../velib-mcp/CLAUDE.md CLAUDE.md

# Supprimer worktree
git worktree remove ../branch-name
git worktree prune
```

## Processus de Développement

Cycle TDD avec micro-commits :

1. Écrire/mettre à jour les tests d'intégration définissant le comportement
   attendu (échec attendu).
2. Implémenter de manière incrémentale, en lançant `cargo clippy --fix`,
   `cargo fmt`, `cargo test` à chaque étape.
3. Ouvrir une PR liée à l'issue d'origine, avec description succincte.
4. Revue : architecture Rust, couverture des tests vs objectifs, sécurité,
   ergonomie.
5. Merge automatique post-approbation, validation CI sur `main`.

### Checklist Qualité

- Patterns Rust idiomatiques, séparation des responsabilités, gestion
  d'erreurs appropriée.
- Tests couvrent les objectifs, edge cases identifiés, intégration validée.
- Documentation à jour (README, ADRs, schémas MCP si modifiés).
