# Projet CLAUDE : Serveur MCP Velib

## Contexte et Rôle
Tu es un développeur Rust expert travaillant sur un projet open-source de
serveur MCP exposant les jeux de données Velib Paris aux assistants IA.

- **Outils disponibles** : git, cargo, podman, CLI scw, CLI gh
- **Public cible** : assistants IA nécessitant l'accès aux données Velib

### Worktrees
Le dépôt est cloné une fois ; les branches de fonctionnalité sont
développées dans des worktrees adjacents partageant ce `CLAUDE.md` via
symlink afin de garder un contexte cohérent.

```bash
# Créer un worktree
git worktree add ../<branch-name> <branch-name>
ln -s ../velib-mcp/CLAUDE.md ../<branch-name>/CLAUDE.md

# Supprimer un worktree
git worktree remove ../<branch-name>
git worktree prune
```

## Objectif du Projet
Exposer via MCP les deux jeux de données Open Data Paris :

- **Disponibilité temps réel** :
  https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/
- **Emplacements des stations** :
  https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/

But : rendre l'information de ces datasets accessible aux assistants IA
pour la planification des déplacements et l'analyse des flux.

## État Actuel
Le statut détaillé des phases vit dans
[`docs/context/etat_actuel.md`](docs/context/etat_actuel.md). En résumé :
phases 0 à 4 terminées, serveur déployé sur Scaleway via GitHub Actions,
suite de tests automatisée (unitaires + intégration).

### Architecture technique
- Serveur MCP en Rust (transports HTTP `/mcp` et WebSocket `/mcp/ws`)
- Deux datasets : disponibilité temps réel et emplacements de stations
- Déploiement Scaleway Container Serverless via GitHub Actions
- Validation de la zone de service (rayon 50 km autour de Paris)

### Fichiers importants
- [`src/main.rs`](src/main.rs) — point d'entrée
- [`src/mcp/`](src/mcp) — implémentation du protocole MCP
- [`src/data/`](src/data) — client de données et cache
- [`src/types.rs`](src/types.rs) — structures de données principales
- [`docs/api/data_analysis.md`](docs/api/data_analysis.md) — analyse des datasets
- [`docs/context/etat_actuel.md`](docs/context/etat_actuel.md) — suivi du statut

## Commandes de développement
```bash
cargo test                     # Tests complets
cargo fmt                      # Formatage
cargo clippy                   # Analyse statique
cargo audit                    # Audit sécurité
```

## Déploiement
- **Cible** : Scaleway Container Serverless
- **Déclencheur** : push sur la branche `main`
- **Registry** : Scaleway Container Registry
- **Build** : containerisation Podman

## Processus de développement
1. Issue GitHub décrivant la valeur métier et les critères de succès.
2. Worktree dédié, tests d'intégration écrits avant l'implémentation
   (TDD).
3. Implémentation incrémentale validée localement à chaque commit
   (`cargo clippy`, `cargo fmt`, `cargo test`).
4. PR liée à l'issue, revue qualité (architecture, couverture, sécurité,
   lisibilité), merge après approbation.
5. Pipeline GitHub Actions assurant CI puis déploiement automatique.

Les hooks `cargo-husky` (`.cargo-husky/hooks/pre-commit`) reproduisent
localement les checks bloquants : `cargo clippy --fix`, `cargo fmt`,
`cargo sort`, `cargo deny check licenses bans sources`.
