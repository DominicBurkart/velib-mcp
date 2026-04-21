# Projet CLAUDE : Serveur MCP Velib

## Contexte et Rôle
Développeur Rust travaillant sur un projet open-source de serveur MCP.

- **Outils disponibles** : git, cargo, podman, CLI scw, CLI gh
- **Public cible** : assistants IA nécessitant l'accès aux données Velib
- **Collaboration** : travail parallèle possible via `git worktree`

## Objectif du Projet
Serveur cloud MCP rendant accessibles aux assistants IA les deux jeux de
données Velib de la ville de Paris :

- **Disponibilité temps réel** : https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/information/
- **Emplacements des stations** : https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/information/

**But** : exposer l'intégralité des informations de ces datasets pour la
planification de transport et l'analyse des flux.

## État Actuel du Projet

### Phases terminées
- **Phase 0** : configuration projet, CI/CD, structure documentation
- **Phase 1** : analyse des données Velib (15+ champs documentés)
- **Phase 2A** : environnement et fondation serveur
- **Phase 2B** : protocole MCP et types de base
- **Phase 3A** : intégration API live et client de données
- **Phase 3B** : handlers MCP avec données temps réel
- **Phase 4** : nettoyage structure repository

Voir `docs/context/etat_actuel.md` pour l'historique détaillé.

### Architecture technique
- Serveur MCP en Rust pour données Velib Paris
- Deux datasets : disponibilité temps réel + métadonnées stations
- Déploiement Scaleway Container Serverless via GitHub Actions
- Suite de tests (18+ tests)
- Validations sécurité dont limite zone de service 50 km

### Fichiers importants
- `src/main.rs` — point d'entrée
- `src/mcp/` — implémentation MCP
- `src/data/` — client données et cache
- `src/types.rs` — structures principales
- `docs/api/data_analysis.md` — analyse des données
- `docs/api/mcp_interface_spec.md` — spécification MCP
- `docs/api/mcp_schemas.md` — schémas Rust

## Commandes Développement
```bash
cargo test       # Tests
cargo fmt        # Formatage
cargo clippy     # Lints
cargo audit      # Audit sécurité
```

Les hooks pre-commit (`cargo-husky`) sont détaillés dans `README.md` section
« Pre-commit hooks ».

## Déploiement
- **Cible** : Scaleway Container Serverless
- **Déclencheur** : push sur `main`
- **Registry** : Scaleway Container Registry
- **Build** : image distroless construite avec Podman

## Worktrees
```bash
# Créer un worktree
git worktree add ../branch-name branch-name

# Supprimer un worktree
git worktree remove ../branch-name
git worktree prune
```

## Processus de Développement

TDD : écrire les tests avant l'implémentation. À chaque commit, le hook
pre-commit exécute `cargo clippy --fix`, `cargo fmt`, `cargo sort`, et
`cargo deny check` — un échec annule le commit.

Chaque PR référence son issue d'origine et passe la CI complète avant merge.
